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

/// Increment this whenever the on-disk format changes.
pub const INDEX_VERSION: u32 = 2;

/// On-disk trigram index mapping 3-byte substrings to file IDs.
#[derive(Serialize, Deserialize)]
pub struct TrigramIndex {
    version: u32,
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

/// Metadata captured before reading a file for indexing.
pub struct FileSnapshot {
    record: FileRecord,
}

/// Unique trigrams extracted from one stable, non-binary file.
pub struct IndexedFile {
    record: FileRecord,
    trigrams: Vec<[u8; 3]>,
}

impl FileSnapshot {
    /// Capture the path and metadata before reading the contents.
    pub fn capture(path: &Path) -> Option<Self> {
        let path = std::path::absolute(path).ok()?;
        let (mtime_s, mtime_ns, size) = file_mtime(&path).ok()?;
        Some(Self { record: FileRecord { path, mtime_s, mtime_ns, size } })
    }

    /// Extract trigrams from the search buffer. Changed and binary files stay unindexed.
    pub fn extract(self, data: &[u8]) -> Option<IndexedFile> {
        if data.len() as u64 != self.record.size || memchr::memchr(0, data).is_some() {
            return None;
        }
        // Do not reserve from windows().size_hint(): repeated bytes may have very few
        // unique trigrams even in a large file.
        let mut trigrams = HashSet::new();
        for t in data.windows(3) {
            trigrams.insert([t[0], t[1], t[2]]);
        }
        let stamp = file_mtime(&self.record.path).ok()?;
        if stamp != (self.record.mtime_s, self.record.mtime_ns, self.record.size) {
            return None;
        }
        Some(IndexedFile { record: self.record, trigrams: trigrams.into_iter().collect() })
    }
}

/// Per-search index decisions. Unknown paths are always searched.
pub struct IndexPlan {
    excluded: HashSet<PathBuf>,
    rebuild: bool,
}

impl IndexPlan {
    /// Whether the loaded index should be replaced after this search.
    pub fn needs_rebuild(&self) -> bool {
        self.rebuild
    }

    /// Only known, unchanged, non-matching files may be skipped.
    pub fn can_skip(&self, path: &Path) -> bool {
        !self.excluded.is_empty()
            && std::path::absolute(path).is_ok_and(|p| self.excluded.contains(&p))
    }
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
        let index_path = dir.join("index.bin");
        let data = fs::read(&index_path).ok()?;
        let Ok(index) = bitcode::deserialize::<Self>(&data) else {
            let _ = fs::remove_dir_all(&dir);
            return None;
        };
        if index.version != INDEX_VERSION {
            let _ = fs::remove_dir_all(&dir);
            return None;
        }
        Some(index)
    }

    /// Builds a new index for `root` from the supplied non-binary files.
    pub fn build(root: &Path, paths: &[PathBuf]) -> Self {
        Self::from_files(
            root,
            paths.iter().filter_map(|path| {
                let snapshot = FileSnapshot::capture(path)?;
                let data = fs::read(path).ok()?;
                snapshot.extract(&data)
            }),
        )
    }

    /// Assemble an index from files observed by search workers, without reading them again.
    pub fn from_files(root: &Path, files: impl IntoIterator<Item = IndexedFile>) -> Self {
        let mut index = Self {
            version: INDEX_VERSION,
            files: Vec::new(),
            postings: BTreeMap::new(),
            root: root.to_owned(),
        };
        let mut seen = HashSet::new();
        for file in files {
            if !seen.insert(file.record.path.clone()) {
                continue;
            }
            let Ok(id) = u32::try_from(index.files.len()) else { break };
            index.files.push(file.record);
            for trigram in file.trigrams {
                index.postings.entry(trigram).or_default().push(id);
            }
        }
        index
    }

    /// Check metadata once and prepare conservative filtering and rebuild decisions.
    /// Disable filtering for inverted searches and modes that output non-matching files.
    pub fn plan(&self, trigrams: &[[u8; 3]], allow_filter: bool) -> IndexPlan {
        let stale: HashSet<_> = self.stale_files().into_iter().collect();
        let rebuild = stale.len() as f64 > self.files.len() as f64 * STALE_REBUILD_RATIO;
        let mut excluded = HashSet::new();
        if allow_filter && !rebuild && !trigrams.is_empty() {
            let candidates = self.candidate_files(trigrams);
            // Common patterns cannot save enough reads to justify filtering.
            if candidates.len() * 10 < self.files.len() * 9 {
                for file in &self.files {
                    if !candidates.contains(&file.path) && !stale.contains(&file.path) {
                        excluded.insert(file.path.clone());
                    }
                }
            }
        }
        IndexPlan { excluded, rebuild }
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

        let mut result = lists[0].clone();
        for list in &lists[1..] {
            // Postings are sorted by file ID; intersect with a monotonic cursor.
            let mut cursor = 0;
            result.retain(|id| {
                while cursor < list.len() && list[cursor] < *id {
                    cursor += 1;
                }
                cursor < list.len() && list[cursor] == *id
            });
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
