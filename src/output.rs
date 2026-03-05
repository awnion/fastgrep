use std::io;
use std::io::Write;
use std::ops::Range;

use crate::searcher::FileResult;

pub(crate) const COLOR_FILENAME: &[u8] = b"\x1b[35m";
const COLOR_LINE_NO: &[u8] = b"\x1b[32m";
const COLOR_MATCH: &[u8] = b"\x1b[01;31m";
pub(crate) const COLOR_RESET: &[u8] = b"\x1b[0m";
pub(crate) const COLOR_SEP: &[u8] = b"\x1b[36m";

/// Controls how search results are formatted on output.
#[derive(Clone)]
pub struct OutputConfig {
    pub color: bool,
    pub line_number: bool,
    pub files_with_matches: bool,
    pub count: bool,
    pub multi_file: bool,
    /// Max line length before truncation (0 = no limit).
    pub max_line_len: usize,
}

/// Truncates a line at `max_len` bytes (on a char boundary) if needed.
/// Returns the (possibly truncated) slice and whether truncation occurred.
#[inline]
fn truncate_line(line: &[u8], max_len: usize) -> (&[u8], bool) {
    if max_len == 0 || line.len() <= max_len {
        return (line, false);
    }
    // Find a valid UTF-8 char boundary to avoid splitting multi-byte chars.
    let mut end = max_len;
    while end > 0 && line.get(end).is_some_and(|&b| b & 0xC0 == 0x80) {
        end -= 1;
    }
    (&line[..end], true)
}

const TRUNCATION_MSG: &[u8] = b" [truncated, see grep --help for --max-line-len]";

/// Writes a single file's search results to `writer`.
///
/// The output format mirrors GNU grep:
///
/// * **`-l`** — only the filename is printed when at least one match exists.
/// * **`-c`** — a count of matching lines (prefixed by filename in
///   multi-file mode).
/// * **default** — each matching line, optionally prefixed with the
///   filename and line number.
///
/// ANSI colour codes identical to GNU grep are used when colour is
/// enabled (filename in magenta, line number in green, matches in
/// bold red, separator `:` in cyan).
///
/// # Errors
///
/// Returns [`io::Error`] on write failure.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
///
/// use fastgrep::output::OutputConfig;
/// use fastgrep::output::format_result;
/// use fastgrep::searcher::FileResult;
/// use fastgrep::searcher::LineMatch;
///
/// let result = FileResult {
///     path: PathBuf::from("demo.txt"),
///     matches: vec![LineMatch {
///         line_no: 3,
///         line: b"hello world".to_vec(),
///         match_ranges: vec![0..5],
///         byte_offset: 20,
///         line_len: 11,
///     }],
///     is_binary: false,
/// };
/// let config = OutputConfig {
///     color: false,
///     line_number: true,
///     files_with_matches: false,
///     count: false,
///     multi_file: false,
///     max_line_len: 0,
/// };
/// let mut buf = Vec::new();
/// format_result(&result, &config, &mut buf).unwrap();
/// assert_eq!(String::from_utf8(buf).unwrap(), "3:hello world\n");
/// ```
pub fn format_result(
    result: &FileResult,
    config: &OutputConfig,
    writer: &mut impl Write,
) -> io::Result<()> {
    let path_bytes = result.path.as_os_str().as_encoded_bytes();

    if config.files_with_matches {
        if !result.matches.is_empty() {
            if config.color {
                writer.write_all(COLOR_FILENAME)?;
                writer.write_all(path_bytes)?;
                writer.write_all(COLOR_RESET)?;
            } else {
                writer.write_all(path_bytes)?;
            }
            writer.write_all(b"\n")?;
        }
        return Ok(());
    }

    // Binary file message goes to stderr (matches GNU grep behaviour).
    // The caller sees is_binary on the FileResult and can print separately.
    if result.is_binary && !result.matches.is_empty() {
        eprintln!("grep: {}: binary file matches", result.path.display());
        return Ok(());
    }

    if config.count {
        let count = result.matches.len();
        if config.multi_file {
            if config.color {
                writer.write_all(COLOR_FILENAME)?;
                writer.write_all(path_bytes)?;
                writer.write_all(COLOR_RESET)?;
                writer.write_all(COLOR_SEP)?;
                writer.write_all(b":")?;
                writer.write_all(COLOR_RESET)?;
            } else {
                writer.write_all(path_bytes)?;
                writer.write_all(b":")?;
            }
        }
        let mut itoa_buf = itoa::Buffer::new();
        writer.write_all(itoa_buf.format(count).as_bytes())?;
        writer.write_all(b"\n")?;
        return Ok(());
    }

    let mut itoa_buf = itoa::Buffer::new();

    for m in &result.matches {
        if config.multi_file {
            if config.color {
                writer.write_all(COLOR_FILENAME)?;
                writer.write_all(path_bytes)?;
                writer.write_all(COLOR_RESET)?;
                writer.write_all(COLOR_SEP)?;
                writer.write_all(b":")?;
                writer.write_all(COLOR_RESET)?;
            } else {
                writer.write_all(path_bytes)?;
                writer.write_all(b":")?;
            }
        }

        if config.line_number {
            if config.color {
                writer.write_all(COLOR_LINE_NO)?;
                writer.write_all(itoa_buf.format(m.line_no).as_bytes())?;
                writer.write_all(COLOR_RESET)?;
                writer.write_all(COLOR_SEP)?;
                writer.write_all(b":")?;
                writer.write_all(COLOR_RESET)?;
            } else {
                writer.write_all(itoa_buf.format(m.line_no).as_bytes())?;
                writer.write_all(b":")?;
            }
        }

        let (line, truncated) = truncate_line(&m.line, config.max_line_len);

        if config.color && !m.match_ranges.is_empty() {
            let mut last_end = 0;
            for range in &m.match_ranges {
                if range.start >= line.len() {
                    break;
                }
                let end = range.end.min(line.len());
                writer.write_all(&line[last_end..range.start])?;
                writer.write_all(COLOR_MATCH)?;
                writer.write_all(&line[range.start..end])?;
                writer.write_all(COLOR_RESET)?;
                last_end = end;
            }
            writer.write_all(&line[last_end..])?;
        } else {
            writer.write_all(line)?;
        }
        if truncated {
            writer.write_all(TRUNCATION_MSG)?;
        }
        writer.write_all(b"\n")?;
    }

    Ok(())
}

/// Writes a single matching line with optional filename prefix, line number,
/// and match highlighting. Used by the streaming search path to write
/// directly from the file buffer without copying line content.
#[inline]
pub fn write_line_match(
    writer: &mut impl Write,
    config: &OutputConfig,
    path_bytes: Option<&[u8]>,
    line_no: u32,
    line: &[u8],
    match_ranges: &[Range<usize>],
) -> io::Result<()> {
    let mut itoa_buf = itoa::Buffer::new();

    if let Some(path_bytes) = path_bytes {
        if config.color {
            writer.write_all(COLOR_FILENAME)?;
            writer.write_all(path_bytes)?;
            writer.write_all(COLOR_RESET)?;
            writer.write_all(COLOR_SEP)?;
            writer.write_all(b":")?;
            writer.write_all(COLOR_RESET)?;
        } else {
            writer.write_all(path_bytes)?;
            writer.write_all(b":")?;
        }
    }

    if config.line_number {
        if config.color {
            writer.write_all(b"\x1b[32m")?;
            writer.write_all(itoa_buf.format(line_no).as_bytes())?;
            writer.write_all(b"\x1b[0m\x1b[36m:\x1b[0m")?;
        } else {
            writer.write_all(itoa_buf.format(line_no).as_bytes())?;
            writer.write_all(b":")?;
        }
    }

    let (line, truncated) = truncate_line(line, config.max_line_len);

    if config.color && !match_ranges.is_empty() {
        let mut last_end = 0;
        for range in match_ranges {
            if range.start >= line.len() {
                break;
            }
            let end = range.end.min(line.len());
            writer.write_all(&line[last_end..range.start])?;
            writer.write_all(b"\x1b[01;31m")?;
            writer.write_all(&line[range.start..end])?;
            writer.write_all(b"\x1b[0m")?;
            last_end = end;
        }
        writer.write_all(&line[last_end..])?;
    } else {
        writer.write_all(line)?;
    }
    if truncated {
        writer.write_all(TRUNCATION_MSG)?;
    }

    writer.write_all(b"\n")
}
