use std::io;
use std::io::Write;
use std::ops::Range;

use serde::Serialize;

use crate::cli::OutputMode;
use crate::searcher::FileResult;

pub(crate) const COLOR_FILENAME: &[u8] = b"\x1b[35m";
const COLOR_LINE_NO: &[u8] = b"\x1b[32m";
const COLOR_MATCH: &[u8] = b"\x1b[01;31m";
pub(crate) const COLOR_RESET: &[u8] = b"\x1b[0m";
pub(crate) const COLOR_SEP: &[u8] = b"\x1b[36m";

/// Controls how search results are formatted on output.
#[derive(Clone)]
pub struct OutputConfig {
    pub mode: OutputMode,
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
    /// Insert tab after prefix (-T).
    pub initial_tab: bool,
    /// Print NUL byte after filename (-Z).
    pub null: bool,
    /// Treat binary files as text (-a).
    pub text: bool,
}

impl OutputConfig {
    #[inline]
    pub fn is_json(&self) -> bool {
        matches!(self.mode, OutputMode::Json)
    }

    #[inline]
    pub fn requires_match_ranges(&self) -> bool {
        self.color || self.only_matching || self.is_json()
    }
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

#[derive(Serialize)]
struct JsonText {
    text: String,
}

#[derive(Serialize)]
struct JsonSubmatch {
    #[serde(rename = "match")]
    matched: JsonText,
    start: usize,
    end: usize,
}

#[derive(Serialize)]
struct JsonMatchRecord {
    #[serde(rename = "type")]
    record_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    line_number: u32,
    absolute_offset: u64,
    lines: JsonText,
    submatches: Vec<JsonSubmatch>,
    truncated: bool,
}

#[derive(Serialize)]
struct JsonContextRecord {
    #[serde(rename = "type")]
    record_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    line_number: u32,
    absolute_offset: u64,
    lines: JsonText,
    truncated: bool,
}

#[derive(Serialize)]
struct JsonPathRecord {
    #[serde(rename = "type")]
    record_type: &'static str,
    path: String,
    matched: bool,
}

#[derive(Serialize)]
struct JsonSummaryRecord {
    #[serde(rename = "type")]
    record_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    count: usize,
}

#[derive(Serialize)]
struct JsonWarningRecord {
    #[serde(rename = "type")]
    record_type: &'static str,
    kind: &'static str,
    path: String,
    size_bytes: u64,
    max_file_size: u64,
}

#[inline]
fn path_bytes_to_string(path_bytes: &[u8]) -> String {
    String::from_utf8_lossy(path_bytes).into_owned()
}

#[inline]
fn path_to_string(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[inline]
fn json_submatches(line: &[u8], match_ranges: &[Range<usize>]) -> Vec<JsonSubmatch> {
    match_ranges
        .iter()
        .filter(|range| range.start < line.len())
        .map(|range| {
            let end = range.end.min(line.len());
            JsonSubmatch {
                matched: JsonText {
                    text: String::from_utf8_lossy(&line[range.start..end]).into_owned(),
                },
                start: range.start,
                end,
            }
        })
        .collect()
}

#[inline]
fn write_json_record(writer: &mut impl Write, record: &impl Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, record).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

#[inline]
pub fn write_json_match(
    writer: &mut impl Write,
    path_bytes: Option<&[u8]>,
    line_no: u32,
    byte_off: u64,
    line: &[u8],
    match_ranges: &[Range<usize>],
    max_line_len: usize,
) -> io::Result<()> {
    let (line, truncated) = truncate_line(line, max_line_len);
    let record = JsonMatchRecord {
        record_type: "match",
        path: path_bytes.map(path_bytes_to_string),
        line_number: line_no,
        absolute_offset: byte_off,
        lines: JsonText { text: String::from_utf8_lossy(line).into_owned() },
        submatches: json_submatches(line, match_ranges),
        truncated,
    };
    write_json_record(writer, &record)
}

#[inline]
pub fn write_json_only_matching(
    writer: &mut impl Write,
    path_bytes: Option<&[u8]>,
    line_no: u32,
    byte_off: u64,
    matched: &[u8],
    max_line_len: usize,
) -> io::Result<()> {
    let (matched, truncated) = truncate_line(matched, max_line_len);
    let text = String::from_utf8_lossy(matched).into_owned();
    let record = JsonMatchRecord {
        record_type: "match",
        path: path_bytes.map(path_bytes_to_string),
        line_number: line_no,
        absolute_offset: byte_off,
        lines: JsonText { text: text.clone() },
        submatches: vec![JsonSubmatch { matched: JsonText { text }, start: 0, end: matched.len() }],
        truncated,
    };
    write_json_record(writer, &record)
}

#[inline]
pub fn write_json_context(
    writer: &mut impl Write,
    path_bytes: Option<&[u8]>,
    line_no: u32,
    byte_off: u64,
    line: &[u8],
    max_line_len: usize,
) -> io::Result<()> {
    let (line, truncated) = truncate_line(line, max_line_len);
    let record = JsonContextRecord {
        record_type: "context",
        path: path_bytes.map(path_bytes_to_string),
        line_number: line_no,
        absolute_offset: byte_off,
        lines: JsonText { text: String::from_utf8_lossy(line).into_owned() },
        truncated,
    };
    write_json_record(writer, &record)
}

#[inline]
pub fn write_json_path(
    writer: &mut impl Write,
    path: &std::path::Path,
    matched: bool,
) -> io::Result<()> {
    let record = JsonPathRecord { record_type: "path", path: path_to_string(path), matched };
    write_json_record(writer, &record)
}

#[inline]
pub fn write_json_summary(
    writer: &mut impl Write,
    path: Option<&std::path::Path>,
    count: usize,
) -> io::Result<()> {
    let record =
        JsonSummaryRecord { record_type: "summary", path: path.map(path_to_string), count };
    write_json_record(writer, &record)
}

#[inline]
pub fn write_json_size_limit_warning(
    writer: &mut impl Write,
    path: &std::path::Path,
    size_bytes: u64,
    max_file_size: u64,
) -> io::Result<()> {
    let record = JsonWarningRecord {
        record_type: "warning",
        kind: "size_limit",
        path: path_to_string(path),
        size_bytes,
        max_file_size,
    };
    write_json_record(writer, &record)
}

/// Default line-number field width used for stdin when -T is enabled.
/// Equals `digits(i64::MAX)` = 19, matching GNU grep's behavior for
/// unseekable inputs where the file size is unknown.
pub const TAB_FIELD_WIDTH_STDIN: usize = 19;

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
/// use fastgrep::cli::OutputMode;
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
///     mode: OutputMode::Text,
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
///     initial_tab: false,
///     null: false,
///     text: false,
/// };
/// let mut buf = Vec::new();
/// format_result(&result, &config, &mut buf, 0).unwrap();
/// assert_eq!(String::from_utf8(buf).unwrap(), "3:hello world\n");
/// ```
/// Writes the filename separator: NUL when -Z is active, otherwise `sep` (`:` or `-`).
#[inline]
fn write_filename_sep(writer: &mut impl Write, config: &OutputConfig, sep: u8) -> io::Result<()> {
    if config.null {
        writer.write_all(b"\0")
    } else if config.color {
        writer.write_all(COLOR_SEP)?;
        writer.write_all(&[sep])?;
        writer.write_all(COLOR_RESET)
    } else {
        writer.write_all(&[sep])
    }
}

/// Writes a numeric field (line number or byte offset) with separator.
/// When -T is active, right-aligns the number within `number_width` characters
/// (matching GNU grep's behavior: `digits(file_size)` for files, 19 for stdin).
#[inline]
fn write_numeric_field(
    writer: &mut impl Write,
    config: &OutputConfig,
    itoa_buf: &mut itoa::Buffer,
    value: impl itoa::Integer + Copy,
    sep: u8,
    is_last_field: bool,
    number_width: usize,
) -> io::Result<()> {
    let num_str = itoa_buf.format(value);

    if config.initial_tab && number_width > num_str.len() {
        let pad = number_width - num_str.len();
        for _ in 0..pad {
            writer.write_all(b" ")?;
        }
    }

    if config.color {
        writer.write_all(COLOR_LINE_NO)?;
        writer.write_all(num_str.as_bytes())?;
        writer.write_all(COLOR_RESET)?;
        writer.write_all(COLOR_SEP)?;
        writer.write_all(&[sep])?;
        writer.write_all(COLOR_RESET)?;
    } else {
        writer.write_all(num_str.as_bytes())?;
        writer.write_all(&[sep])?;
    }

    if config.initial_tab && is_last_field {
        writer.write_all(b"\t")?;
    }

    Ok(())
}

pub fn format_result(
    result: &FileResult,
    config: &OutputConfig,
    writer: &mut impl Write,
    number_width: usize,
) -> io::Result<()> {
    let path_bytes = result.path.as_os_str().as_encoded_bytes();
    let json_path = (!result.path.as_os_str().is_empty()).then_some(path_bytes);

    if config.files_with_matches {
        if !result.matches.is_empty() {
            if config.is_json() {
                write_json_path(writer, &result.path, true)?;
                return Ok(());
            }
            if config.color {
                writer.write_all(COLOR_FILENAME)?;
                writer.write_all(path_bytes)?;
                writer.write_all(COLOR_RESET)?;
            } else {
                writer.write_all(path_bytes)?;
            }
            if config.null {
                writer.write_all(b"\0")?;
            } else {
                writer.write_all(b"\n")?;
            }
        }
        return Ok(());
    }

    // Binary file message goes to stderr (matches GNU grep behaviour).
    // The caller sees is_binary on the FileResult and can print separately.
    if result.is_binary && !config.text && !result.matches.is_empty() {
        eprintln!("grep: {}: binary file matches", result.path.display());
        return Ok(());
    }

    if config.count {
        let mut count = result.matches.len();
        if config.max_count > 0 && count > config.max_count {
            count = config.max_count;
        }
        if config.is_json() {
            write_json_summary(writer, json_path.map(|_| result.path.as_path()), count)?;
            return Ok(());
        }
        if config.multi_file {
            if config.color {
                writer.write_all(COLOR_FILENAME)?;
                writer.write_all(path_bytes)?;
                writer.write_all(COLOR_RESET)?;
            } else {
                writer.write_all(path_bytes)?;
            }
            write_filename_sep(writer, config, b':')?;
        }
        let mut itoa_buf = itoa::Buffer::new();
        writer.write_all(itoa_buf.format(count).as_bytes())?;
        writer.write_all(b"\n")?;
        return Ok(());
    }

    let mut itoa_buf = itoa::Buffer::new();

    let max = if config.max_count > 0 { config.max_count } else { usize::MAX };

    let has_line_no = config.line_number;
    let has_byte_off = config.byte_offset;

    // -o mode: print each match part on its own line
    if config.only_matching {
        let path_opt = if config.is_json() || config.multi_file { Some(path_bytes) } else { None };
        for m in result.matches.iter().take(max) {
            for range in &m.match_ranges {
                if range.start >= m.line.len() {
                    break;
                }
                let end = range.end.min(m.line.len());
                let matched = &m.line[range.start..end];

                if config.is_json() {
                    write_json_only_matching(
                        writer,
                        path_opt,
                        m.line_no,
                        m.byte_offset + range.start as u64,
                        matched,
                        config.max_line_len,
                    )?;
                    continue;
                }

                if let Some(pb) = path_opt {
                    if config.color {
                        writer.write_all(COLOR_FILENAME)?;
                        writer.write_all(pb)?;
                        writer.write_all(COLOR_RESET)?;
                    } else {
                        writer.write_all(pb)?;
                    }
                    write_filename_sep(writer, config, b':')?;
                }

                let is_last_ln = !has_byte_off;
                if has_line_no {
                    write_numeric_field(
                        writer,
                        config,
                        &mut itoa_buf,
                        m.line_no,
                        b':',
                        is_last_ln,
                        number_width,
                    )?;
                }
                if has_byte_off {
                    write_numeric_field(
                        writer,
                        config,
                        &mut itoa_buf,
                        m.byte_offset,
                        b':',
                        true,
                        number_width,
                    )?;
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
        let path_opt = if config.is_json() || config.multi_file { Some(path_bytes) } else { None };
        if config.is_json() {
            write_json_match(
                writer,
                path_opt,
                m.line_no,
                m.byte_offset,
                &m.line,
                &m.match_ranges,
                config.max_line_len,
            )?;
            continue;
        }

        if config.multi_file {
            if config.color {
                writer.write_all(COLOR_FILENAME)?;
                writer.write_all(path_bytes)?;
                writer.write_all(COLOR_RESET)?;
            } else {
                writer.write_all(path_bytes)?;
            }
            write_filename_sep(writer, config, b':')?;
        }

        let is_last_ln = !has_byte_off;
        if has_line_no {
            write_numeric_field(
                writer,
                config,
                &mut itoa_buf,
                m.line_no,
                b':',
                is_last_ln,
                number_width,
            )?;
        }
        if has_byte_off {
            write_numeric_field(
                writer,
                config,
                &mut itoa_buf,
                m.byte_offset,
                b':',
                true,
                number_width,
            )?;
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
#[allow(clippy::too_many_arguments)]
pub fn write_line_match(
    writer: &mut impl Write,
    config: &OutputConfig,
    path_bytes: Option<&[u8]>,
    line_no: u32,
    byte_off: u64,
    line: &[u8],
    match_ranges: &[Range<usize>],
    number_width: usize,
) -> io::Result<()> {
    if config.is_json() {
        return write_json_match(
            writer,
            path_bytes,
            line_no,
            byte_off,
            line,
            match_ranges,
            config.max_line_len,
        );
    }

    let mut itoa_buf = itoa::Buffer::new();

    if let Some(path_bytes) = path_bytes {
        if config.color {
            writer.write_all(COLOR_FILENAME)?;
            writer.write_all(path_bytes)?;
            writer.write_all(COLOR_RESET)?;
        } else {
            writer.write_all(path_bytes)?;
        }
        write_filename_sep(writer, config, b':')?;
    }

    let has_line_no = config.line_number;
    let has_byte_off = config.byte_offset;
    let is_last_ln = !has_byte_off;

    if has_line_no {
        write_numeric_field(
            writer,
            config,
            &mut itoa_buf,
            line_no,
            b':',
            is_last_ln,
            number_width,
        )?;
    }
    if has_byte_off {
        write_numeric_field(writer, config, &mut itoa_buf, byte_off, b':', true, number_width)?;
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
    if config.is_json() {
        return Ok(());
    }
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
    number_width: usize,
) -> io::Result<()> {
    if config.is_json() {
        return write_json_context(
            writer,
            path_bytes,
            line_no,
            byte_off,
            line,
            config.max_line_len,
        );
    }

    let mut itoa_buf = itoa::Buffer::new();

    if let Some(path_bytes) = path_bytes {
        if config.color {
            writer.write_all(COLOR_FILENAME)?;
            writer.write_all(path_bytes)?;
            writer.write_all(COLOR_RESET)?;
        } else {
            writer.write_all(path_bytes)?;
        }
        write_filename_sep(writer, config, b'-')?;
    }

    let has_line_no = config.line_number;
    let has_byte_off = config.byte_offset;
    let is_last_ln = !has_byte_off;

    if has_line_no {
        write_numeric_field(
            writer,
            config,
            &mut itoa_buf,
            line_no,
            b'-',
            is_last_ln,
            number_width,
        )?;
    }
    if has_byte_off {
        write_numeric_field(writer, config, &mut itoa_buf, byte_off, b'-', true, number_width)?;
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
#[allow(clippy::too_many_arguments)]
pub fn write_only_matching(
    writer: &mut impl Write,
    config: &OutputConfig,
    path_bytes: Option<&[u8]>,
    line_no: u32,
    byte_off: u64,
    line: &[u8],
    match_ranges: &[Range<usize>],
    number_width: usize,
) -> io::Result<usize> {
    if config.is_json() {
        let mut count = 0;
        for range in match_ranges {
            if range.start >= line.len() {
                break;
            }
            let end = range.end.min(line.len());
            let matched = &line[range.start..end];
            write_json_only_matching(
                writer,
                path_bytes,
                line_no,
                byte_off + range.start as u64,
                matched,
                config.max_line_len,
            )?;
            count += 1;
        }
        return Ok(count);
    }

    let mut itoa_buf = itoa::Buffer::new();
    let mut count = 0;

    let has_line_no = config.line_number;
    let has_byte_off = config.byte_offset;

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
            } else {
                writer.write_all(path_bytes)?;
            }
            write_filename_sep(writer, config, b':')?;
        }

        let is_last_ln = !has_byte_off;
        if has_line_no {
            write_numeric_field(
                writer,
                config,
                &mut itoa_buf,
                line_no,
                b':',
                is_last_ln,
                number_width,
            )?;
        }
        if has_byte_off {
            write_numeric_field(writer, config, &mut itoa_buf, byte_off, b':', true, number_width)?;
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
