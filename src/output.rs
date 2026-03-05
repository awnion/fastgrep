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
    pub files_without_match: bool,
    pub count: bool,
    pub quiet: bool,
    pub max_count: usize,
    pub multi_file: bool,
    /// Max line length before truncation (0 = no limit).
    pub max_line_len: usize,
    /// Print only matched parts of a line (-o).
    pub only_matching: bool,
    /// Lines of context after a match (-A).
    pub after_context: usize,
    /// Lines of context before a match (-B).
    pub before_context: usize,
    /// Print byte offset before each line (-b).
    pub byte_offset: bool,
    /// Ignore binary files (-I).
    pub ignore_binary: bool,
    /// Group separator for context output. `None` = no separator.
    pub group_separator: Option<String>,
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
///     files_without_match: false,
///     count: false,
///     quiet: false,
///     max_count: 0,
///     multi_file: false,
///     max_line_len: 0,
///     only_matching: false,
///     after_context: 0,
///     before_context: 0,
///     byte_offset: false,
///     ignore_binary: false,
///     group_separator: Some("--".to_string()),
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
        let mut count = result.matches.len();
        if config.max_count > 0 && count > config.max_count {
            count = config.max_count;
        }
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

    let max = if config.max_count > 0 { config.max_count } else { usize::MAX };

    // -o mode: print each match part on its own line
    if config.only_matching {
        let path_opt = if config.multi_file { Some(path_bytes) } else { None };
        for m in result.matches.iter().take(max) {
            for range in &m.match_ranges {
                if range.start >= m.line.len() {
                    break;
                }
                let end = range.end.min(m.line.len());
                let matched = &m.line[range.start..end];

                if let Some(pb) = path_opt {
                    if config.color {
                        writer.write_all(COLOR_FILENAME)?;
                        writer.write_all(pb)?;
                        writer.write_all(COLOR_RESET)?;
                        writer.write_all(COLOR_SEP)?;
                        writer.write_all(b":")?;
                        writer.write_all(COLOR_RESET)?;
                    } else {
                        writer.write_all(pb)?;
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

                if config.byte_offset {
                    if config.color {
                        writer.write_all(COLOR_LINE_NO)?;
                        writer.write_all(itoa_buf.format(m.byte_offset).as_bytes())?;
                        writer.write_all(COLOR_RESET)?;
                        writer.write_all(COLOR_SEP)?;
                        writer.write_all(b":")?;
                        writer.write_all(COLOR_RESET)?;
                    } else {
                        writer.write_all(itoa_buf.format(m.byte_offset).as_bytes())?;
                        writer.write_all(b":")?;
                    }
                }

                if config.color {
                    writer.write_all(COLOR_MATCH)?;
                    writer.write_all(matched)?;
                    writer.write_all(COLOR_RESET)?;
                } else {
                    writer.write_all(matched)?;
                }
                writer.write_all(b"\n")?;
            }
        }
        return Ok(());
    }

    for m in result.matches.iter().take(max) {
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

        if config.byte_offset {
            if config.color {
                writer.write_all(COLOR_LINE_NO)?;
                writer.write_all(itoa_buf.format(m.byte_offset).as_bytes())?;
                writer.write_all(COLOR_RESET)?;
                writer.write_all(COLOR_SEP)?;
                writer.write_all(b":")?;
                writer.write_all(COLOR_RESET)?;
            } else {
                writer.write_all(itoa_buf.format(m.byte_offset).as_bytes())?;
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
    byte_off: u64,
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

    if config.byte_offset {
        if config.color {
            writer.write_all(b"\x1b[32m")?;
            writer.write_all(itoa_buf.format(byte_off).as_bytes())?;
            writer.write_all(b"\x1b[0m\x1b[36m:\x1b[0m")?;
        } else {
            writer.write_all(itoa_buf.format(byte_off).as_bytes())?;
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

/// Writes the group separator between non-contiguous context groups.
/// Respects `--group-separator` and `--no-group-separator`.
#[inline]
pub fn write_group_separator(writer: &mut impl Write, config: &OutputConfig) -> io::Result<()> {
    let Some(ref sep) = config.group_separator else {
        return Ok(());
    };
    if config.color {
        writer.write_all(COLOR_SEP)?;
        writer.write_all(sep.as_bytes())?;
        writer.write_all(COLOR_RESET)?;
    } else {
        writer.write_all(sep.as_bytes())?;
    }
    writer.write_all(b"\n")
}

/// Writes a context (non-matching) line. Uses `-` as separator instead of `:`.
#[inline]
pub fn write_context_line(
    writer: &mut impl Write,
    config: &OutputConfig,
    path_bytes: Option<&[u8]>,
    line_no: u32,
    byte_off: u64,
    line: &[u8],
) -> io::Result<()> {
    let mut itoa_buf = itoa::Buffer::new();

    if let Some(path_bytes) = path_bytes {
        if config.color {
            writer.write_all(COLOR_FILENAME)?;
            writer.write_all(path_bytes)?;
            writer.write_all(COLOR_RESET)?;
            writer.write_all(COLOR_SEP)?;
            writer.write_all(b"-")?;
            writer.write_all(COLOR_RESET)?;
        } else {
            writer.write_all(path_bytes)?;
            writer.write_all(b"-")?;
        }
    }

    if config.line_number {
        if config.color {
            writer.write_all(b"\x1b[32m")?;
            writer.write_all(itoa_buf.format(line_no).as_bytes())?;
            writer.write_all(b"\x1b[0m\x1b[36m-\x1b[0m")?;
        } else {
            writer.write_all(itoa_buf.format(line_no).as_bytes())?;
            writer.write_all(b"-")?;
        }
    }

    if config.byte_offset {
        if config.color {
            writer.write_all(b"\x1b[32m")?;
            writer.write_all(itoa_buf.format(byte_off).as_bytes())?;
            writer.write_all(b"\x1b[0m\x1b[36m-\x1b[0m")?;
        } else {
            writer.write_all(itoa_buf.format(byte_off).as_bytes())?;
            writer.write_all(b"-")?;
        }
    }

    let (line, truncated) = truncate_line(line, config.max_line_len);
    writer.write_all(line)?;
    if truncated {
        writer.write_all(TRUNCATION_MSG)?;
    }
    writer.write_all(b"\n")
}

/// Writes only the matched parts of a line, each on its own line.
/// Returns the number of match parts written.
#[inline]
pub fn write_only_matching(
    writer: &mut impl Write,
    config: &OutputConfig,
    path_bytes: Option<&[u8]>,
    line_no: u32,
    byte_off: u64,
    line: &[u8],
    match_ranges: &[Range<usize>],
) -> io::Result<usize> {
    let mut itoa_buf = itoa::Buffer::new();
    let mut count = 0;

    for range in match_ranges {
        if range.start >= line.len() {
            break;
        }
        let end = range.end.min(line.len());
        let matched = &line[range.start..end];

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

        if config.byte_offset {
            if config.color {
                writer.write_all(b"\x1b[32m")?;
                writer.write_all(itoa_buf.format(byte_off).as_bytes())?;
                writer.write_all(b"\x1b[0m\x1b[36m:\x1b[0m")?;
            } else {
                writer.write_all(itoa_buf.format(byte_off).as_bytes())?;
                writer.write_all(b":")?;
            }
        }

        if config.color {
            writer.write_all(b"\x1b[01;31m")?;
            writer.write_all(matched)?;
            writer.write_all(b"\x1b[0m")?;
        } else {
            writer.write_all(matched)?;
        }
        writer.write_all(b"\n")?;
        count += 1;
    }

    Ok(count)
}
