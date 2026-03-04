use std::fs;
use std::io;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;

use memmap2::Mmap;

use crate::cache::CacheEntry;
use crate::pattern::CompiledPattern;

/// Files larger than this threshold are memory-mapped instead of read
/// into a heap buffer.
const MMAP_THRESHOLD: u64 = 64 * 1024;

/// Number of leading bytes inspected for NUL when detecting binary files.
const BINARY_CHECK_LEN: usize = 8192;

/// Aggregated search results for a single file.
///
/// When the file is binary, `is_binary` is set and `matches` contains
/// at most one sentinel entry (no line content).
pub struct FileResult {
    pub path: PathBuf,
    pub matches: Vec<LineMatch>,
    pub is_binary: bool,
}

/// A single matching (or, with `-v`, non-matching) line.
pub struct LineMatch {
    pub line_no: u32,
    pub line: Vec<u8>,
    pub match_ranges: Vec<Range<usize>>,
    pub byte_offset: u64,
    pub line_len: u32,
}

impl FileResult {
    /// Converts the result into a [`CacheEntry`] suitable for
    /// persisting to the JSONL cache.
    ///
    /// # Example
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use fastgrep::searcher::FileResult;
    /// use fastgrep::searcher::LineMatch;
    ///
    /// let result = FileResult {
    ///     path: PathBuf::from("test.txt"),
    ///     matches: vec![LineMatch {
    ///         line_no: 1,
    ///         line: b"hello".to_vec(),
    ///         match_ranges: vec![0..5],
    ///         byte_offset: 0,
    ///         line_len: 5,
    ///     }],
    ///     is_binary: false,
    /// };
    /// let entry = result.to_cache_entry();
    /// assert_eq!(entry.line_nos, vec![1]);
    /// ```
    pub fn to_cache_entry(&self) -> CacheEntry {
        CacheEntry {
            line_nos: self.matches.iter().map(|m| m.line_no).collect(),
            offsets: self.matches.iter().map(|m| (m.byte_offset, m.line_len)).collect(),
        }
    }
}

/// Returns `true` if the first [`BINARY_CHECK_LEN`] bytes contain a NUL.
fn is_binary(data: &[u8]) -> bool {
    let check_len = data.len().min(BINARY_CHECK_LEN);
    data[..check_len].contains(&0)
}

/// Searches `path` for lines matching `pattern`.
///
/// Files larger than 64 KiB are memory-mapped; smaller files are read
/// into memory. Binary files (detected by a NUL byte in the first
/// 8 KiB) produce at most a single sentinel match with no line content.
///
/// When `invert_match` is `true`, non-matching lines are returned
/// instead.
///
/// # Errors
///
/// Returns [`io::Error`] if the file cannot be read.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
///
/// use clap::Parser;
/// use fastgrep::cli::Cli;
/// use fastgrep::pattern::CompiledPattern;
/// use fastgrep::searcher::search_file;
///
/// let cli = Cli::parse_from(["grep", "fn", "src/lib.rs"]);
/// let config = cli.resolve();
/// let pattern = CompiledPattern::compile(&config).unwrap();
/// let result = search_file(Path::new("src/lib.rs"), &pattern, false).unwrap();
/// for m in &result.matches {
///     println!("{}:{}", m.line_no, String::from_utf8_lossy(&m.line));
/// }
/// ```
pub fn search_file(
    path: &Path,
    pattern: &CompiledPattern,
    invert_match: bool,
) -> io::Result<FileResult> {
    let metadata = fs::metadata(path)?;
    let size = metadata.len();

    let data: Box<dyn AsRef<[u8]>> = if size > MMAP_THRESHOLD {
        let file = fs::File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        Box::new(mmap)
    } else {
        Box::new(fs::read(path)?)
    };

    let bytes: &[u8] = (*data).as_ref();

    if is_binary(bytes) {
        let has_match = if invert_match {
            true
        } else {
            pattern.regex.is_match(std::str::from_utf8(bytes).unwrap_or(""))
        };
        return Ok(FileResult {
            path: path.to_owned(),
            matches: if has_match {
                vec![LineMatch {
                    line_no: 0,
                    line: Vec::new(),
                    match_ranges: Vec::new(),
                    byte_offset: 0,
                    line_len: 0,
                }]
            } else {
                Vec::new()
            },
            is_binary: true,
        });
    }

    let mut matches = Vec::new();
    let mut line_no: u32 = 0;
    let mut offset: u64 = 0;

    let data = bytes.strip_suffix(b"\n").unwrap_or(bytes);

    for line_bytes in data.split(|&b| b == b'\n') {
        line_no += 1;
        let line_len = line_bytes.len() as u32;
        let line_str = String::from_utf8_lossy(line_bytes);

        let is_match = pattern.regex.is_match(&line_str);
        let should_include = if invert_match { !is_match } else { is_match };

        if should_include {
            let match_ranges = if !invert_match {
                pattern.regex.find_iter(&line_str).map(|m| m.start()..m.end()).collect()
            } else {
                Vec::new()
            };

            matches.push(LineMatch {
                line_no,
                line: line_bytes.to_vec(),
                match_ranges,
                byte_offset: offset,
                line_len,
            });
        }

        offset += line_len as u64 + 1;
    }

    Ok(FileResult { path: path.to_owned(), matches, is_binary: false })
}
