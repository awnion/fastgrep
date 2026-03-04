use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;

/// On-disk JSON representation of a cached search result for one file.
#[derive(Debug, Serialize, Deserialize)]
struct CacheRecord {
    path: PathBuf,
    mtime_s: i64,
    mtime_ns: u32,
    size: u64,
    line_nos: Vec<u32>,
    offsets: Vec<(u64, u32)>,
}

/// Cached match data for a single file, stored in memory.
pub struct CacheEntry {
    pub line_nos: Vec<u32>,
    /// Each element is `(byte_offset, line_byte_length)`.
    pub offsets: Vec<(u64, u32)>,
}

/// Snapshot of a file's mtime and size used for cache invalidation.
struct FileIdentity {
    mtime_s: i64,
    mtime_ns: u32,
    size: u64,
}

/// In-memory index of all cached results for a single pattern+flags
/// combination.
///
/// Backed by a JSONL file at
/// `~/.cache/fastgrep/v1/<pattern_hash>/index.jsonl`.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
///
/// use fastgrep::cache::CacheEntry;
/// use fastgrep::cache::CacheIndex;
///
/// if let Some(index) = CacheIndex::load("abcdef0123456789") {
///     if let Some(entry) = index.lookup(Path::new("src/main.rs")) {
///         println!("cached {} matching lines", entry.line_nos.len());
///     }
/// }
/// ```
pub struct CacheIndex {
    entries: HashMap<PathBuf, (FileIdentity, CacheEntry)>,
    cache_dir: PathBuf,
    /// Whether the on-disk file had duplicate entries and needs compaction.
    needs_compact: bool,
}

/// Maximum JSONL lines before triggering compaction on load.
const COMPACT_THRESHOLD_RATIO: f64 = 1.5;

/// Returns the cache directory for the given pattern key.
fn cache_dir(pattern_key: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".cache").join("fastgrep").join("v1").join(pattern_key))
}

/// Reads a file's mtime and size from the filesystem.
fn file_identity(path: &Path) -> io::Result<FileIdentity> {
    let meta = fs::metadata(path)?;
    let mtime = meta.modified()?.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    Ok(FileIdentity {
        mtime_s: mtime.as_secs() as i64,
        mtime_ns: mtime.subsec_nanos(),
        size: meta.len(),
    })
}

impl CacheIndex {
    /// Loads (or creates an empty) cache index for `pattern_key`.
    ///
    /// Returns `None` only when `$HOME` is unset.  A missing JSONL
    /// file is not an error — an empty index is returned instead.
    ///
    /// Automatically compacts the on-disk file when duplicate entries
    /// exceed 50% of the unique entry count.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fastgrep::cache::CacheIndex;
    ///
    /// let index = CacheIndex::load("deadbeef12345678").unwrap();
    /// assert!(index.lookup(std::path::Path::new("/no/such/file")).is_none());
    /// ```
    pub fn load(pattern_key: &str) -> Option<Self> {
        let dir = cache_dir(pattern_key)?;
        let index_path = dir.join("index.jsonl");

        let mut entries = HashMap::new();
        let mut total_lines: usize = 0;

        if let Ok(file) = fs::File::open(&index_path) {
            let reader = io::BufReader::with_capacity(64 * 1024, file);
            for line in reader.lines() {
                let Ok(line) = line else { continue };
                let Ok(record) = serde_json::from_str::<CacheRecord>(&line) else {
                    continue;
                };
                total_lines += 1;
                let identity = FileIdentity {
                    mtime_s: record.mtime_s,
                    mtime_ns: record.mtime_ns,
                    size: record.size,
                };
                let entry = CacheEntry { line_nos: record.line_nos, offsets: record.offsets };
                entries.insert(record.path, (identity, entry));
            }
        }

        let unique_count = entries.len();
        let needs_compact =
            unique_count > 0 && total_lines as f64 > unique_count as f64 * COMPACT_THRESHOLD_RATIO;

        let mut index = Self { entries, cache_dir: dir, needs_compact };

        if needs_compact {
            let _ = index.compact();
        }

        Some(index)
    }

    /// Returns the cached entry for `path` if it exists **and** the
    /// file has not been modified since the entry was recorded.
    ///
    /// Invalidation compares `(mtime_s, mtime_ns, size)` from the
    /// filesystem against the stored values.
    pub fn lookup(&self, path: &Path) -> Option<&CacheEntry> {
        let (identity, entry) = self.entries.get(path)?;

        let current = file_identity(path).ok()?;
        if current.mtime_s != identity.mtime_s
            || current.mtime_ns != identity.mtime_ns
            || current.size != identity.size
        {
            return None;
        }

        Some(entry)
    }

    /// Appends a new cache entry for `path` to the JSONL file.
    ///
    /// Creates the cache directory if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] on filesystem failures.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::Path;
    ///
    /// use fastgrep::cache::CacheEntry;
    /// use fastgrep::cache::CacheIndex;
    ///
    /// let index = CacheIndex::load("abcdef0123456789").unwrap();
    /// let entry = CacheEntry { line_nos: vec![10, 42], offsets: vec![(200, 30), (900, 25)] };
    /// index.append(Path::new("src/lib.rs"), &entry).unwrap();
    /// ```
    pub fn append(&self, path: &Path, entry: &CacheEntry) -> io::Result<()> {
        fs::create_dir_all(&self.cache_dir)?;
        let index_path = self.cache_dir.join("index.jsonl");

        let ident = file_identity(path)?;
        let record = CacheRecord {
            path: path.to_owned(),
            mtime_s: ident.mtime_s,
            mtime_ns: ident.mtime_ns,
            size: ident.size,
            line_nos: entry.line_nos.clone(),
            offsets: entry.offsets.clone(),
        };

        let mut file = fs::OpenOptions::new().create(true).append(true).open(index_path)?;

        let line = serde_json::to_string(&record)?;
        writeln!(file, "{line}")?;

        Ok(())
    }

    /// Appends multiple cache entries in a single file open/write.
    ///
    /// Much more efficient than calling [`append`](Self::append)
    /// in a loop when there are many results.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] on filesystem failures.
    pub fn append_batch(&self, entries: &[(PathBuf, CacheEntry)]) -> io::Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        fs::create_dir_all(&self.cache_dir)?;
        let index_path = self.cache_dir.join("index.jsonl");
        let file = fs::OpenOptions::new().create(true).append(true).open(index_path)?;
        let mut writer = io::BufWriter::new(file);

        for (path, entry) in entries {
            let Ok(ident) = file_identity(path) else { continue };
            let record = CacheRecord {
                path: path.clone(),
                mtime_s: ident.mtime_s,
                mtime_ns: ident.mtime_ns,
                size: ident.size,
                line_nos: entry.line_nos.clone(),
                offsets: entry.offsets.clone(),
            };
            if let Ok(line) = serde_json::to_string(&record) {
                let _ = writeln!(writer, "{line}");
            }
        }

        writer.flush()
    }

    /// Rewrites the JSONL file with only unique entries (latest per path).
    fn compact(&mut self) -> io::Result<()> {
        let index_path = self.cache_dir.join("index.jsonl");
        let tmp_path = self.cache_dir.join("index.jsonl.tmp");

        let file = fs::File::create(&tmp_path)?;
        let mut writer = io::BufWriter::new(file);

        for (path, (identity, entry)) in &self.entries {
            let record = CacheRecord {
                path: path.clone(),
                mtime_s: identity.mtime_s,
                mtime_ns: identity.mtime_ns,
                size: identity.size,
                line_nos: entry.line_nos.clone(),
                offsets: entry.offsets.clone(),
            };
            if let Ok(line) = serde_json::to_string(&record) {
                writeln!(writer, "{line}")?;
            }
        }

        writer.flush()?;
        drop(writer);
        fs::rename(&tmp_path, &index_path)?;
        self.needs_compact = false;
        Ok(())
    }
}
