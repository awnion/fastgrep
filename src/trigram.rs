use std::collections::BTreeMap;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;

/// Maximum total size of all trigram indexes before LRU eviction kicks in.
const MAX_CACHE_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB

/// Fraction of stale files that triggers a full rebuild instead of incremental.
const STALE_REBUILD_RATIO: f64 = 0.10;

/// On-disk trigram index mapping 3-byte substrings to file IDs.
#[derive(Serialize, Deserialize)]
pub struct TrigramIndex {
    files: Vec<FileRecord>,
    postings: BTreeMap<[u8; 3], Vec<u32>>,
    root: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct FileRecord {
    path: PathBuf,
    mtime_s: i64,
    mtime_ns: u32,
    size: u64,
}

/// Returns the cache directory for a given root path's trigram index.
fn index_dir(root: &Path) -> Option<PathBuf> {
    let base = dirs::cache_dir()?;
    let mut hasher = DefaultHasher::new();
    root.hash(&mut hasher);
    let hash = format!("{:016x}", hasher.finish());
    Some(base.join("fastgrep").join("trigram").join(hash))
}

/// Returns the top-level fastgrep trigram cache directory.
fn trigram_cache_root() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("fastgrep").join("trigram"))
}

fn file_mtime(path: &Path) -> io::Result<(i64, u32, u64)> {
    let meta = fs::metadata(path)?;
    let mtime = meta.modified()?.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    Ok((mtime.as_secs() as i64, mtime.subsec_nanos(), meta.len()))
}

impl TrigramIndex {
    /// Loads a previously saved index for `root`. Returns `None` if no
    /// index exists or deserialization fails.
    pub fn load(root: &Path) -> Option<Self> {
        let dir = index_dir(root)?;
        let data = fs::read(dir.join("index.bin")).ok()?;
        bitcode::deserialize(&data).ok()
    }

    /// Builds a new trigram index by walking `root` and extracting
    /// trigrams from every non-binary file.
    pub fn build(root: &Path, paths: &[PathBuf]) -> Self {
        let mut files = Vec::new();
        let mut postings: BTreeMap<[u8; 3], Vec<u32>> = BTreeMap::new();

        for path in paths {
            let Ok((mtime_s, mtime_ns, size)) = file_mtime(path) else { continue };
            let Ok(data) = fs::read(path) else { continue };

            // Skip binary files (NUL in first 8KiB)
            let check = data.len().min(8192);
            if memchr::memchr(0, &data[..check]).is_some() {
                continue;
            }

            let file_id = files.len() as u32;
            files.push(FileRecord { path: path.clone(), mtime_s, mtime_ns, size });

            let mut seen = HashSet::new();
            for tri in data.windows(3) {
                let key = [tri[0], tri[1], tri[2]];
                if seen.insert(key) {
                    postings.entry(key).or_default().push(file_id);
                }
            }
        }

        Self { files, postings, root: root.to_owned() }
    }

    /// Returns the set of files that contain ALL given trigrams.
    /// If `trigrams` is empty, returns all indexed file paths.
    pub fn candidate_files(&self, trigrams: &[[u8; 3]]) -> HashSet<PathBuf> {
        if trigrams.is_empty() {
            return self.files.iter().map(|f| f.path.clone()).collect();
        }

        // Start with the shortest postings list for efficiency.
        let mut lists: Vec<&Vec<u32>> =
            trigrams.iter().filter_map(|tri| self.postings.get(tri)).collect();

        if lists.len() < trigrams.len() {
            // At least one trigram has no postings → no file can match.
            return HashSet::new();
        }

        lists.sort_by_key(|l| l.len());

        let mut result: HashSet<u32> = lists[0].iter().copied().collect();
        for list in &lists[1..] {
            let set: HashSet<u32> = list.iter().copied().collect();
            result.retain(|id| set.contains(id));
            if result.is_empty() {
                return HashSet::new();
            }
        }

        result.iter().map(|&id| self.files[id as usize].path.clone()).collect()
    }

    /// Returns paths of files whose mtime/size has changed since indexing.
    pub fn stale_files(&self) -> Vec<PathBuf> {
        self.files
            .iter()
            .filter(|f| {
                match file_mtime(&f.path) {
                    Ok((s, ns, sz)) => s != f.mtime_s || ns != f.mtime_ns || sz != f.size,
                    Err(_) => true, // file gone or unreadable
                }
            })
            .map(|f| f.path.clone())
            .collect()
    }

    /// Returns true if more than `STALE_REBUILD_RATIO` of files are stale.
    pub fn needs_rebuild(&self) -> bool {
        if self.files.is_empty() {
            return false;
        }
        let stale = self.stale_files().len();
        stale as f64 > self.files.len() as f64 * STALE_REBUILD_RATIO
    }

    /// Number of indexed files.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Serializes the index to disk.
    pub fn save(&self) -> io::Result<()> {
        let Some(dir) = index_dir(&self.root) else {
            return Err(io::Error::other("cannot determine cache dir"));
        };
        fs::create_dir_all(&dir)?;
        let data = bitcode::serialize(self).map_err(io::Error::other)?;
        let tmp = dir.join("index.bin.tmp");
        fs::write(&tmp, &data)?;
        fs::rename(&tmp, dir.join("index.bin"))?;
        Ok(())
    }
}

/// Evicts oldest trigram index directories until total size is under `MAX_CACHE_BYTES`.
pub fn evict_if_needed() {
    let Some(root) = trigram_cache_root() else { return };
    let Ok(entries) = fs::read_dir(&root) else { return };

    let mut dirs: Vec<(PathBuf, u64, SystemTime)> = Vec::new();
    let mut total: u64 = 0;

    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let index_file = path.join("index.bin");
        if let Ok(meta) = fs::metadata(&index_file) {
            let size = meta.len();
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            total += size;
            dirs.push((path, size, mtime));
        }
    }

    if total <= MAX_CACHE_BYTES {
        return;
    }

    // Sort oldest first
    dirs.sort_by_key(|(_, _, mtime)| *mtime);

    for (dir, size, _) in &dirs {
        if total <= MAX_CACHE_BYTES {
            break;
        }
        let _ = fs::remove_dir_all(dir);
        total -= size;
    }
}
