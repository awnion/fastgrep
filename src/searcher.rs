use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;

use memchr::memchr;
use memchr::memchr_iter;
use memchr::memrchr;
use memmap2::Mmap;

use crate::output::OutputConfig;
use crate::output::write_line_match;
use crate::pattern::CompiledPattern;

/// Files larger than this threshold are memory-mapped instead of read
/// into a heap buffer.
const MMAP_THRESHOLD: u64 = 256 * 1024;

/// Owned or borrowed file data for reusable-buffer reads.
enum FileDataRef<'a> {
    Mmap(Mmap),
    Borrowed(&'a [u8]),
}

impl<'a> AsRef<[u8]> for FileDataRef<'a> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        match self {
            FileDataRef::Mmap(m) => m.as_ref(),
            FileDataRef::Borrowed(b) => b,
        }
    }
}

impl<'a> std::ops::Deref for FileDataRef<'a> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.as_ref()
    }
}

/// Reads a file, reusing `buf` for small files and mmap for large ones.
fn read_file_reuse<'a>(path: &Path, buf: &'a mut Vec<u8>) -> io::Result<FileDataRef<'a>> {
    let file = fs::File::open(path)?;
    let size = file.metadata()?.len();
    if size > MMAP_THRESHOLD {
        let mmap = unsafe { Mmap::map(&file)? };
        #[cfg(unix)]
        mmap.advise(memmap2::Advice::Sequential)?;
        Ok(FileDataRef::Mmap(mmap))
    } else {
        buf.clear();
        let mut file = file;
        file.read_to_end(buf)?;
        Ok(FileDataRef::Borrowed(buf.as_slice()))
    }
}

/// Files larger than this are searched with parallel threads in single-file mode.
const PARALLEL_THRESHOLD: usize = 4 * 1024 * 1024;

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

/// A file's contents, either memory-mapped or heap-allocated.
enum FileData {
    Mmap(Mmap),
    Read(Vec<u8>),
}

impl AsRef<[u8]> for FileData {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        match self {
            FileData::Mmap(m) => m.as_ref(),
            FileData::Read(v) => v.as_ref(),
        }
    }
}

impl std::ops::Deref for FileData {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.as_ref()
    }
}

/// Reads a file, using mmap for large files and `fs::read` for small ones.
/// Uses `fstat` on the opened fd instead of a separate `stat` syscall.
fn read_file(path: &Path) -> io::Result<FileData> {
    let file = fs::File::open(path)?;
    let size = file.metadata()?.len();
    if size > MMAP_THRESHOLD {
        let mmap = unsafe { Mmap::map(&file)? };
        #[cfg(unix)]
        mmap.advise(memmap2::Advice::Sequential)?;
        Ok(FileData::Mmap(mmap))
    } else {
        Ok(FileData::Read(fs::read(path)?))
    }
}

/// Returns `true` if `data` contains a NUL byte (matches GNU grep behaviour).
fn is_binary(data: &[u8]) -> bool {
    memchr(0, data).is_some()
}

/// Strips a trailing `\n` line terminator.
#[inline]
fn strip_line_terminator(data: &[u8]) -> &[u8] {
    data.strip_suffix(b"\n").unwrap_or(data)
}

/// Returns `true` if at least one line in `data` does NOT match `pattern`.
fn has_non_matching_line(data: &[u8], pattern: &CompiledPattern) -> bool {
    let data = strip_line_terminator(data);
    let mut start = 0;
    loop {
        let end = match memchr(b'\n', &data[start..]) {
            Some(pos) => start + pos,
            None => data.len(),
        };
        if !pattern.is_match(&data[start..end]) {
            return true;
        }
        if end == data.len() {
            break;
        }
        start = end + 1;
    }
    false
}

/// Searches `path` for lines matching `pattern`.
///
/// Files larger than 256 KiB are memory-mapped; smaller files are read
/// into memory. Binary files (detected by a NUL byte in the first
/// 8 KiB) produce at most a single sentinel match with no line content.
///
/// When `invert_match` is `true`, non-matching lines are returned
/// instead. When `need_ranges` is `false`, match highlight ranges
/// are not computed (faster for `-c` / `-l` modes).
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
/// let result = search_file(Path::new("src/lib.rs"), &pattern, false, true, false).unwrap();
/// for m in &result.matches {
///     println!("{}:{}", m.line_no, String::from_utf8_lossy(&m.line));
/// }
/// ```
pub fn search_file(
    path: &Path,
    pattern: &CompiledPattern,
    invert_match: bool,
    need_ranges: bool,
    count_only: bool,
) -> io::Result<FileResult> {
    let bytes = read_file(path)?;
    let bytes: &[u8] = &bytes;

    if is_binary(bytes) {
        if count_only {
            let count = count_matches(bytes, pattern, invert_match);
            let matches = (0..count)
                .map(|_| LineMatch {
                    line_no: 0,
                    line: Vec::new(),
                    match_ranges: Vec::new(),
                    byte_offset: 0,
                    line_len: 0,
                })
                .collect();
            return Ok(FileResult { path: path.to_owned(), matches, is_binary: true });
        }
        let has_match = if invert_match { true } else { pattern.is_match(bytes) };
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

    if count_only {
        let count = count_matches(bytes, pattern, invert_match);
        let matches = (0..count)
            .map(|_| LineMatch {
                line_no: 0,
                line: Vec::new(),
                match_ranges: Vec::new(),
                byte_offset: 0,
                line_len: 0,
            })
            .collect();
        return Ok(FileResult { path: path.to_owned(), matches, is_binary: false });
    }

    let matches = search_bytes(bytes, pattern, invert_match, need_ranges);
    Ok(FileResult { path: path.to_owned(), matches, is_binary: false })
}

/// Searches raw bytes for lines matching `pattern`.
///
/// Used for stdin input where no file path is involved. The returned
/// [`FileResult`] has an empty path and `is_binary` is always `false`
/// (binary detection is skipped for stdin).
///
/// # Errors
///
/// Returns [`io::Error`] if reading from the source fails.
///
/// # Example
///
/// ```
/// use clap::Parser;
/// use fastgrep::cli::Cli;
/// use fastgrep::pattern::CompiledPattern;
/// use fastgrep::searcher::search_reader;
///
/// let cli = Cli::parse_from(["grep", "hello"]);
/// let config = cli.resolve();
/// let pattern = CompiledPattern::compile(&config).unwrap();
/// let mut input = std::io::Cursor::new(b"hello world\ngoodbye\nhello again\n");
/// let result = search_reader(&mut input, &pattern, false, true, false).unwrap();
/// assert_eq!(result.matches.len(), 2);
/// ```
pub fn search_reader(
    reader: &mut dyn Read,
    pattern: &CompiledPattern,
    invert_match: bool,
    need_ranges: bool,
    count_only: bool,
) -> io::Result<FileResult> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    if count_only {
        let count = count_matches(&buf, pattern, invert_match);
        let matches = (0..count)
            .map(|_| LineMatch {
                line_no: 0,
                line: Vec::new(),
                match_ranges: Vec::new(),
                byte_offset: 0,
                line_len: 0,
            })
            .collect();
        return Ok(FileResult { path: PathBuf::new(), matches, is_binary: false });
    }

    let matches = search_bytes(&buf, pattern, invert_match, need_ranges);
    Ok(FileResult { path: PathBuf::new(), matches, is_binary: false })
}

/// Counts matching lines without allocating line content.
///
/// For literal patterns, uses whole-buffer memmem scan counting
/// unique line hits. For regex, line-by-line with `is_match`.
fn count_matches(data: &[u8], pattern: &CompiledPattern, invert_match: bool) -> usize {
    let data = strip_line_terminator(data);

    if !invert_match && let Some(finder) = pattern.literal_finder() {
        let mut count = 0;
        let mut last_line_start: usize = usize::MAX;
        for match_pos in finder.find_iter(data) {
            let line_start = match memrchr(b'\n', &data[..match_pos]) {
                Some(pos) => pos + 1,
                None => 0,
            };
            if line_start != last_line_start {
                count += 1;
                last_line_start = line_start;
            }
        }
        return count;
    }

    // Prefix-accelerated count: find candidate lines via literal prefix,
    // then verify with full regex. Skips lines that can't match.
    if !invert_match && let Some(pfx) = pattern.prefix_finder() {
        let mut count = 0;
        let mut last_line_start: usize = usize::MAX;
        for match_pos in pfx.find_iter(data) {
            let line_start = match memrchr(b'\n', &data[..match_pos]) {
                Some(pos) => pos + 1,
                None => 0,
            };
            if line_start == last_line_start {
                continue;
            }
            last_line_start = line_start;
            let line_end = match memchr(b'\n', &data[line_start..]) {
                Some(pos) => line_start + pos,
                None => data.len(),
            };
            if pattern.regex.is_match(&data[line_start..line_end]) {
                count += 1;
            }
        }
        return count;
    }

    // Fallback: line-by-line count
    let mut count = 0;
    let mut start = 0;
    loop {
        let end = match memchr(b'\n', &data[start..]) {
            Some(pos) => start + pos,
            None => data.len(),
        };
        let line_bytes = &data[start..end];
        let is_match = pattern.is_match(line_bytes);
        if invert_match { !is_match } else { is_match }.then(|| count += 1);
        if end == data.len() {
            break;
        }
        start = end + 1;
    }
    count
}

/// Core search operating on raw byte slices.
///
/// For literal patterns (no regex), searches the whole buffer with
/// SIMD-accelerated memmem and resolves line boundaries only for hits.
/// For regex or invert mode, falls back to line-by-line scanning.
fn search_bytes(
    data: &[u8],
    pattern: &CompiledPattern,
    invert_match: bool,
    need_ranges: bool,
) -> Vec<LineMatch> {
    let data = strip_line_terminator(data);

    if !invert_match && let Some(finder) = pattern.literal_finder() {
        return search_literal_whole_buffer(data, finder, need_ranges);
    }

    if !invert_match && let Some(pfx) = pattern.prefix_finder() {
        return search_prefix_accelerated(data, pfx, pattern, need_ranges);
    }

    search_bytes_line_by_line(data, pattern, invert_match, need_ranges)
}

/// Prefix-accelerated regex search: uses memmem to find candidate
/// positions via the literal prefix, resolves line boundaries, then
/// verifies with the full regex. Skips non-candidate lines entirely.
fn search_prefix_accelerated(
    data: &[u8],
    pfx: &memchr::memmem::Finder<'_>,
    pattern: &CompiledPattern,
    need_ranges: bool,
) -> Vec<LineMatch> {
    let mut matches: Vec<LineMatch> = Vec::new();
    let mut last_line_start: usize = usize::MAX;
    let mut counted_up_to: usize = 0;
    let mut current_line_no: u32 = 1;

    for match_pos in pfx.find_iter(data) {
        let line_start = match memrchr(b'\n', &data[..match_pos]) {
            Some(pos) => pos + 1,
            None => 0,
        };

        if line_start == last_line_start {
            continue; // already processed this line
        }
        last_line_start = line_start;

        let line_end = match memchr(b'\n', &data[line_start..]) {
            Some(pos) => line_start + pos,
            None => data.len(),
        };

        let line_bytes = &data[line_start..line_end];

        if !pattern.regex.is_match(line_bytes) {
            continue; // prefix matched but full regex didn't
        }

        if line_start > counted_up_to {
            current_line_no += memchr_iter(b'\n', &data[counted_up_to..line_start]).count() as u32;
        }
        counted_up_to = line_start;

        let match_ranges = if need_ranges {
            pattern.regex.find_iter(line_bytes).map(|m| m.start()..m.end()).collect()
        } else {
            Vec::new()
        };

        matches.push(LineMatch {
            line_no: current_line_no,
            line: line_bytes.to_vec(),
            match_ranges,
            byte_offset: line_start as u64,
            line_len: line_bytes.len() as u32,
        });
    }

    matches
}

/// Whole-buffer literal search using `memchr::memmem::Finder`.
///
/// Scans the entire buffer for literal byte matches, then resolves
/// line boundaries only for hits — avoiding per-line overhead for
/// non-matching lines.
fn search_literal_whole_buffer(
    data: &[u8],
    finder: &memchr::memmem::Finder<'_>,
    need_ranges: bool,
) -> Vec<LineMatch> {
    let mut matches: Vec<LineMatch> = Vec::new();
    let mut last_line_start: usize = usize::MAX;
    // Track the byte offset up to which newlines have been counted
    let mut counted_up_to: usize = 0;
    let mut current_line_no: u32 = 1;

    let needle_len = finder.needle().len();

    for match_pos in finder.find_iter(data) {
        // Find line start: byte after the preceding newline (or 0)
        let line_start = match memrchr(b'\n', &data[..match_pos]) {
            Some(pos) => pos + 1,
            None => 0,
        };

        // Deduplicate: multiple matches on the same line.
        // No need to push extra ranges — the new-line branch below
        // re-scans the entire line and collects all matches at once.
        if line_start == last_line_start {
            continue;
        }
        last_line_start = line_start;

        // Find line end
        let line_end = match memchr(b'\n', &data[line_start..]) {
            Some(pos) => line_start + pos,
            None => data.len(),
        };

        // Incremental line number: count newlines between last counted
        // position and the start of this line
        if line_start > counted_up_to {
            current_line_no += memchr_iter(b'\n', &data[counted_up_to..line_start]).count() as u32;
        }
        counted_up_to = line_start;

        let line_bytes = &data[line_start..line_end];

        let match_ranges = if need_ranges {
            finder.find_iter(line_bytes).map(|pos| pos..(pos + needle_len)).collect()
        } else {
            Vec::new()
        };

        matches.push(LineMatch {
            line_no: current_line_no,
            line: line_bytes.to_vec(),
            match_ranges,
            byte_offset: line_start as u64,
            line_len: line_bytes.len() as u32,
        });
    }

    matches
}

/// Line-by-line search for regex patterns and invert-match mode.
fn search_bytes_line_by_line(
    data: &[u8],
    pattern: &CompiledPattern,
    invert_match: bool,
    need_ranges: bool,
) -> Vec<LineMatch> {
    let mut matches = Vec::new();
    let mut line_no: u32 = 1;
    let mut start = 0;

    loop {
        let end = match memchr(b'\n', &data[start..]) {
            Some(pos) => start + pos,
            None => data.len(),
        };

        let line_bytes = &data[start..end];
        let line_len = line_bytes.len() as u32;

        let is_match = pattern.is_match(line_bytes);
        let should_include = if invert_match { !is_match } else { is_match };

        if should_include {
            let match_ranges = if need_ranges && !invert_match {
                pattern.regex.find_iter(line_bytes).map(|m| m.start()..m.end()).collect()
            } else {
                Vec::new()
            };

            matches.push(LineMatch {
                line_no,
                line: line_bytes.to_vec(),
                match_ranges,
                byte_offset: start as u64,
                line_len,
            });
        }

        if end == data.len() {
            break;
        }
        start = end + 1;
        line_no += 1;
    }

    matches
}

/// Splits `data` into `n` chunks at newline boundaries.
fn split_at_newlines(data: &[u8], n: usize) -> Vec<&[u8]> {
    let chunk_size = data.len() / n;
    let mut chunks = Vec::with_capacity(n);
    let mut start = 0;

    for _ in 0..n - 1 {
        let target = start + chunk_size;
        if target >= data.len() {
            break;
        }
        let boundary = match memchr(b'\n', &data[target..]) {
            Some(pos) => target + pos + 1,
            None => data.len(),
        };
        chunks.push(&data[start..boundary]);
        start = boundary;
        if start >= data.len() {
            break;
        }
    }

    if start < data.len() {
        chunks.push(&data[start..]);
    }

    chunks
}

/// Parallel count for large files: splits data into chunks and counts
/// matches in each chunk using `std::thread::scope`.
fn parallel_count_matches(data: &[u8], pattern: &CompiledPattern, invert_match: bool) -> usize {
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    if n <= 1 || data.len() < PARALLEL_THRESHOLD {
        return count_matches(data, pattern, invert_match);
    }

    let data = strip_line_terminator(data);

    let chunks = split_at_newlines(data, n);
    std::thread::scope(|s| {
        let handles: Vec<_> = chunks
            .iter()
            .map(|chunk| s.spawn(|| count_matches(chunk, pattern, invert_match)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    })
}

/// Parallel search for large files: splits data into chunks, searches
/// each in parallel, then writes results in order with correct line numbers.
fn parallel_search_streaming(
    data: &[u8],
    pattern: &CompiledPattern,
    invert_match: bool,
    need_ranges: bool,
    config: &OutputConfig,
    path_bytes: Option<&[u8]>,
    writer: &mut impl Write,
) -> io::Result<usize> {
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    if n <= 1 || data.len() < PARALLEL_THRESHOLD {
        if !invert_match && let Some(finder) = pattern.literal_finder() {
            return stream_literal_whole_buffer(
                data,
                finder,
                need_ranges,
                config,
                path_bytes,
                writer,
            );
        }
        return stream_line_by_line(
            data,
            pattern,
            invert_match,
            need_ranges,
            config,
            path_bytes,
            writer,
        );
    }

    let data = strip_line_terminator(data);

    let chunks = split_at_newlines(data, n);

    // Phase 1: Search each chunk in parallel, collecting matches as
    // (relative_line_no, line_slice, match_ranges)
    type ChunkMatch<'a> = (u32, &'a [u8], Vec<Range<usize>>);
    let chunk_results: Vec<Vec<ChunkMatch<'_>>> = std::thread::scope(|s| {
        let handles: Vec<_> = chunks
            .iter()
            .map(|chunk| {
                s.spawn(|| search_chunk_collect(chunk, pattern, invert_match, need_ranges))
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // Phase 2: Compute starting line number for each chunk
    let mut line_offsets = Vec::with_capacity(chunks.len());
    let mut cumulative_lines: u32 = 0;
    for (i, chunk) in chunks.iter().enumerate() {
        line_offsets.push(cumulative_lines);
        if i < chunks.len() - 1 {
            cumulative_lines += memchr_iter(b'\n', chunk).count() as u32;
        }
    }

    // Phase 3: Write results in order with adjusted line numbers
    let mut total = 0;
    for (chunk_matches, line_offset) in chunk_results.iter().zip(line_offsets.iter()) {
        for (rel_line_no, line, ranges) in chunk_matches {
            write_line_match(writer, config, path_bytes, rel_line_no + line_offset, line, ranges)?;
            total += 1;
        }
    }

    Ok(total)
}

/// Searches a chunk and collects matches as (relative_line_no, line_slice, match_ranges).
fn search_chunk_collect<'a>(
    data: &'a [u8],
    pattern: &CompiledPattern,
    invert_match: bool,
    need_ranges: bool,
) -> Vec<(u32, &'a [u8], Vec<Range<usize>>)> {
    let data = strip_line_terminator(data);

    // Use literal whole-buffer approach for literal patterns
    if !invert_match && let Some(finder) = pattern.literal_finder() {
        return collect_literal_whole_buffer(data, finder, need_ranges);
    }

    // Prefix-accelerated regex: find candidates via literal prefix, verify with regex
    if !invert_match && let Some(pfx) = pattern.prefix_finder() {
        return collect_prefix_accelerated(data, pfx, pattern, need_ranges);
    }

    // Line-by-line fallback
    let mut results = Vec::new();
    let mut line_no: u32 = 1;
    let mut start = 0;

    loop {
        let end = match memchr(b'\n', &data[start..]) {
            Some(pos) => start + pos,
            None => data.len(),
        };

        let line_bytes = &data[start..end];
        let is_match = pattern.is_match(line_bytes);
        let should_include = if invert_match { !is_match } else { is_match };

        if should_include {
            let match_ranges = if need_ranges && !invert_match {
                pattern.regex.find_iter(line_bytes).map(|m| m.start()..m.end()).collect()
            } else {
                Vec::new()
            };
            results.push((line_no, line_bytes, match_ranges));
        }

        if end == data.len() {
            break;
        }
        start = end + 1;
        line_no += 1;
    }

    results
}

/// Prefix-accelerated regex collection for parallel chunks.
fn collect_prefix_accelerated<'a>(
    data: &'a [u8],
    pfx: &memchr::memmem::Finder<'_>,
    pattern: &CompiledPattern,
    need_ranges: bool,
) -> Vec<(u32, &'a [u8], Vec<Range<usize>>)> {
    let mut results = Vec::new();
    let mut last_line_start: usize = usize::MAX;
    let mut counted_up_to: usize = 0;
    let mut current_line_no: u32 = 1;

    for match_pos in pfx.find_iter(data) {
        let line_start = match memrchr(b'\n', &data[..match_pos]) {
            Some(pos) => pos + 1,
            None => 0,
        };

        if line_start == last_line_start {
            continue;
        }
        last_line_start = line_start;

        let line_end = match memchr(b'\n', &data[line_start..]) {
            Some(pos) => line_start + pos,
            None => data.len(),
        };

        let line_bytes = &data[line_start..line_end];
        if !pattern.regex.is_match(line_bytes) {
            continue;
        }

        if line_start > counted_up_to {
            current_line_no += memchr_iter(b'\n', &data[counted_up_to..line_start]).count() as u32;
        }
        counted_up_to = line_start;

        let match_ranges = if need_ranges {
            pattern.regex.find_iter(line_bytes).map(|m| m.start()..m.end()).collect()
        } else {
            Vec::new()
        };

        results.push((current_line_no, line_bytes, match_ranges));
    }

    results
}

/// Collects literal matches from a chunk using whole-buffer memmem.
fn collect_literal_whole_buffer<'a>(
    data: &'a [u8],
    finder: &memchr::memmem::Finder<'_>,
    need_ranges: bool,
) -> Vec<(u32, &'a [u8], Vec<Range<usize>>)> {
    let mut results = Vec::new();
    let mut last_line_start: usize = usize::MAX;
    let mut counted_up_to: usize = 0;
    let mut current_line_no: u32 = 1;
    let needle_len = finder.needle().len();

    for match_pos in finder.find_iter(data) {
        let line_start = match memrchr(b'\n', &data[..match_pos]) {
            Some(pos) => pos + 1,
            None => 0,
        };

        if line_start == last_line_start {
            continue;
        }
        last_line_start = line_start;

        let line_end = match memchr(b'\n', &data[line_start..]) {
            Some(pos) => line_start + pos,
            None => data.len(),
        };

        if line_start > counted_up_to {
            current_line_no += memchr_iter(b'\n', &data[counted_up_to..line_start]).count() as u32;
        }
        counted_up_to = line_start;

        let line_bytes = &data[line_start..line_end];
        let match_ranges = if need_ranges {
            finder.find_iter(line_bytes).map(|pos| pos..(pos + needle_len)).collect()
        } else {
            Vec::new()
        };

        results.push((current_line_no, line_bytes, match_ranges));
    }

    results
}

/// Searches a file and streams output directly to `writer`, avoiding
/// intermediate `Vec<LineMatch>` allocation. Returns the match count.
///
/// This is the fast path for single-file mode: results are written
/// as they are found, with zero-copy line output from the file buffer.
pub fn search_file_streaming(
    path: &Path,
    pattern: &CompiledPattern,
    invert_match: bool,
    output_config: &OutputConfig,
    writer: &mut impl Write,
) -> io::Result<usize> {
    let data = read_file(path)?;
    let bytes: &[u8] = &data;

    if is_binary(bytes) {
        if output_config.count {
            let count = count_matches(bytes, pattern, invert_match);
            if output_config.multi_file {
                let path_bytes = path.as_os_str().as_encoded_bytes();
                if output_config.color {
                    writer.write_all(crate::output::COLOR_FILENAME)?;
                    writer.write_all(path_bytes)?;
                    writer.write_all(crate::output::COLOR_RESET)?;
                    writer.write_all(crate::output::COLOR_SEP)?;
                    writer.write_all(b":")?;
                    writer.write_all(crate::output::COLOR_RESET)?;
                } else {
                    writer.write_all(path_bytes)?;
                    writer.write_all(b":")?;
                }
            }
            let mut itoa_buf = itoa::Buffer::new();
            writer.write_all(itoa_buf.format(count).as_bytes())?;
            writer.write_all(b"\n")?;
            return Ok(count);
        }
        let has_match = if invert_match { true } else { pattern.is_match(bytes) };
        if has_match {
            if output_config.files_with_matches {
                let path_bytes = path.as_os_str().as_encoded_bytes();
                if output_config.color {
                    writer.write_all(crate::output::COLOR_FILENAME)?;
                    writer.write_all(path_bytes)?;
                    writer.write_all(crate::output::COLOR_RESET)?;
                } else {
                    writer.write_all(path_bytes)?;
                }
                writer.write_all(b"\n")?;
            } else {
                eprintln!("grep: {}: binary file matches", path.display());
            }
            return Ok(1);
        }
        return Ok(0);
    }

    if output_config.files_with_matches {
        let has_match = if invert_match {
            // Check if any line does NOT match the pattern
            has_non_matching_line(bytes, pattern)
        } else {
            pattern.is_match(bytes)
        };
        if has_match {
            let path_bytes = path.as_os_str().as_encoded_bytes();
            if output_config.color {
                writer.write_all(b"\x1b[35m")?;
                writer.write_all(path_bytes)?;
                writer.write_all(b"\x1b[0m")?;
            } else {
                writer.write_all(path_bytes)?;
            }
            writer.write_all(b"\n")?;
            return Ok(1);
        }
        return Ok(0);
    }

    if output_config.count {
        let count = parallel_count_matches(bytes, pattern, invert_match);
        let path_bytes = path.as_os_str().as_encoded_bytes();
        if output_config.multi_file {
            if output_config.color {
                writer.write_all(b"\x1b[35m")?;
                writer.write_all(path_bytes)?;
                writer.write_all(b"\x1b[0m\x1b[36m:\x1b[0m")?;
            } else {
                writer.write_all(path_bytes)?;
                writer.write_all(b":")?;
            }
        }
        let mut itoa_buf = itoa::Buffer::new();
        writer.write_all(itoa_buf.format(count).as_bytes())?;
        writer.write_all(b"\n")?;
        return Ok(count);
    }

    let need_ranges = output_config.color;
    let path_bytes =
        if output_config.multi_file { Some(path.as_os_str().as_encoded_bytes()) } else { None };

    if bytes.len() >= PARALLEL_THRESHOLD {
        return parallel_search_streaming(
            bytes,
            pattern,
            invert_match,
            need_ranges,
            output_config,
            path_bytes,
            writer,
        );
    }

    if !invert_match && let Some(finder) = pattern.literal_finder() {
        return stream_literal_whole_buffer(
            bytes,
            finder,
            need_ranges,
            output_config,
            path_bytes,
            writer,
        );
    }

    stream_line_by_line(
        bytes,
        pattern,
        invert_match,
        need_ranges,
        output_config,
        path_bytes,
        writer,
    )
}

/// Like [`search_file_streaming`] but reuses `read_buf` for small-file reads,
/// avoiding per-file heap allocation. Workers should create one `Vec<u8>` at
/// thread start and pass it here for every file.
pub fn search_file_streaming_reuse(
    path: &Path,
    pattern: &CompiledPattern,
    invert_match: bool,
    output_config: &OutputConfig,
    writer: &mut impl Write,
    read_buf: &mut Vec<u8>,
) -> io::Result<usize> {
    let data = read_file_reuse(path, read_buf)?;
    let bytes: &[u8] = &data;

    if is_binary(bytes) {
        if output_config.count {
            let count = count_matches(bytes, pattern, invert_match);
            if output_config.multi_file {
                let path_bytes = path.as_os_str().as_encoded_bytes();
                if output_config.color {
                    writer.write_all(crate::output::COLOR_FILENAME)?;
                    writer.write_all(path_bytes)?;
                    writer.write_all(crate::output::COLOR_RESET)?;
                    writer.write_all(crate::output::COLOR_SEP)?;
                    writer.write_all(b":")?;
                    writer.write_all(crate::output::COLOR_RESET)?;
                } else {
                    writer.write_all(path_bytes)?;
                    writer.write_all(b":")?;
                }
            }
            let mut itoa_buf = itoa::Buffer::new();
            writer.write_all(itoa_buf.format(count).as_bytes())?;
            writer.write_all(b"\n")?;
            return Ok(count);
        }
        let has_match = if invert_match { true } else { pattern.is_match(bytes) };
        if has_match {
            if output_config.files_with_matches {
                let path_bytes = path.as_os_str().as_encoded_bytes();
                if output_config.color {
                    writer.write_all(crate::output::COLOR_FILENAME)?;
                    writer.write_all(path_bytes)?;
                    writer.write_all(crate::output::COLOR_RESET)?;
                } else {
                    writer.write_all(path_bytes)?;
                }
                writer.write_all(b"\n")?;
            } else {
                eprintln!("grep: {}: binary file matches", path.display());
            }
            return Ok(1);
        }
        return Ok(0);
    }

    if output_config.files_with_matches {
        let has_match = if invert_match {
            !pattern.is_match(bytes) || bytes.contains(&b'\n')
        } else {
            pattern.is_match(bytes)
        };
        if has_match {
            let path_bytes = path.as_os_str().as_encoded_bytes();
            if output_config.color {
                writer.write_all(b"\x1b[35m")?;
                writer.write_all(path_bytes)?;
                writer.write_all(b"\x1b[0m")?;
            } else {
                writer.write_all(path_bytes)?;
            }
            writer.write_all(b"\n")?;
            return Ok(1);
        }
        return Ok(0);
    }

    if output_config.count {
        let count = count_matches(bytes, pattern, invert_match);
        let path_bytes = path.as_os_str().as_encoded_bytes();
        if output_config.multi_file {
            if output_config.color {
                writer.write_all(b"\x1b[35m")?;
                writer.write_all(path_bytes)?;
                writer.write_all(b"\x1b[0m\x1b[36m:\x1b[0m")?;
            } else {
                writer.write_all(path_bytes)?;
                writer.write_all(b":")?;
            }
        }
        let mut itoa_buf = itoa::Buffer::new();
        writer.write_all(itoa_buf.format(count).as_bytes())?;
        writer.write_all(b"\n")?;
        return Ok(count);
    }

    let need_ranges = output_config.color;
    let path_bytes =
        if output_config.multi_file { Some(path.as_os_str().as_encoded_bytes()) } else { None };

    if !invert_match && let Some(finder) = pattern.literal_finder() {
        return stream_literal_whole_buffer(
            bytes,
            finder,
            need_ranges,
            output_config,
            path_bytes,
            writer,
        );
    }

    stream_line_by_line(
        bytes,
        pattern,
        invert_match,
        need_ranges,
        output_config,
        path_bytes,
        writer,
    )
}

/// Streaming literal whole-buffer search — writes output as matches are found.
fn stream_literal_whole_buffer(
    data: &[u8],
    finder: &memchr::memmem::Finder<'_>,
    need_ranges: bool,
    config: &OutputConfig,
    path_bytes: Option<&[u8]>,
    writer: &mut impl Write,
) -> io::Result<usize> {
    let data = strip_line_terminator(data);

    let mut count = 0;
    let mut last_line_start: usize = usize::MAX;
    let mut counted_up_to: usize = 0;
    let mut current_line_no: u32 = 1;
    let needle_len = finder.needle().len();

    // Collect ranges for the current line when coloring
    let mut pending_ranges: Vec<Range<usize>> = Vec::new();
    let mut pending_line: &[u8] = &[];
    let mut pending_line_no: u32 = 0;

    for match_pos in finder.find_iter(data) {
        let line_start = match memrchr(b'\n', &data[..match_pos]) {
            Some(pos) => pos + 1,
            None => 0,
        };

        if line_start == last_line_start {
            continue;
        }

        // Flush previous pending line
        if last_line_start != usize::MAX {
            write_line_match(
                writer,
                config,
                path_bytes,
                pending_line_no,
                pending_line,
                &pending_ranges,
            )?;
            count += 1;
        }

        last_line_start = line_start;

        let line_end = match memchr(b'\n', &data[line_start..]) {
            Some(pos) => line_start + pos,
            None => data.len(),
        };

        if line_start > counted_up_to {
            current_line_no += memchr_iter(b'\n', &data[counted_up_to..line_start]).count() as u32;
        }
        counted_up_to = line_start;

        pending_line = &data[line_start..line_end];
        pending_line_no = current_line_no;
        pending_ranges.clear();
        if need_ranges {
            for pos in finder.find_iter(pending_line) {
                pending_ranges.push(pos..(pos + needle_len));
            }
        }
    }

    // Flush last pending line
    if last_line_start != usize::MAX {
        write_line_match(
            writer,
            config,
            path_bytes,
            pending_line_no,
            pending_line,
            &pending_ranges,
        )?;
        count += 1;
    }

    Ok(count)
}

/// Streaming line-by-line search — writes output as matches are found.
fn stream_line_by_line(
    data: &[u8],
    pattern: &CompiledPattern,
    invert_match: bool,
    need_ranges: bool,
    config: &OutputConfig,
    path_bytes: Option<&[u8]>,
    writer: &mut impl Write,
) -> io::Result<usize> {
    let data = strip_line_terminator(data);

    let mut count = 0;
    let mut line_no: u32 = 1;
    let mut start = 0;

    loop {
        let end = match memchr(b'\n', &data[start..]) {
            Some(pos) => start + pos,
            None => data.len(),
        };

        let line_bytes = &data[start..end];
        let is_match = pattern.is_match(line_bytes);
        let should_include = if invert_match { !is_match } else { is_match };

        if should_include {
            let match_ranges: Vec<Range<usize>> = if need_ranges && !invert_match {
                pattern.regex.find_iter(line_bytes).map(|m| m.start()..m.end()).collect()
            } else {
                Vec::new()
            };

            write_line_match(writer, config, path_bytes, line_no, line_bytes, &match_ranges)?;
            count += 1;
        }

        if end == data.len() {
            break;
        }
        start = end + 1;
        line_no += 1;
    }

    Ok(count)
}
