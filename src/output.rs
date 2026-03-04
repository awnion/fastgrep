use std::io;
use std::io::Write;

use crate::searcher::FileResult;

const COLOR_FILENAME: &str = "\x1b[35m";
const COLOR_LINE_NO: &str = "\x1b[32m";
const COLOR_MATCH: &str = "\x1b[01;31m";
const COLOR_RESET: &str = "\x1b[0m";
const COLOR_SEP: &str = "\x1b[36m";

/// Controls how search results are formatted on output.
pub struct OutputConfig {
    pub color: bool,
    pub line_number: bool,
    pub files_with_matches: bool,
    pub count: bool,
    pub multi_file: bool,
}

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
    let path_str = result.path.to_string_lossy();

    if result.is_binary && !result.matches.is_empty() {
        writeln!(writer, "Binary file {path_str} matches")?;
        return Ok(());
    }

    if config.files_with_matches {
        if !result.matches.is_empty() {
            if config.color {
                writeln!(writer, "{COLOR_FILENAME}{path_str}{COLOR_RESET}")?;
            } else {
                writeln!(writer, "{path_str}")?;
            }
        }
        return Ok(());
    }

    if config.count {
        let count = result.matches.len();
        if config.multi_file {
            if config.color {
                writeln!(
                    writer,
                    "{COLOR_FILENAME}{path_str}{COLOR_RESET}{COLOR_SEP}:{COLOR_RESET}{count}"
                )?;
            } else {
                writeln!(writer, "{path_str}:{count}")?;
            }
        } else {
            writeln!(writer, "{count}")?;
        }
        return Ok(());
    }

    for m in &result.matches {
        let line_str = String::from_utf8_lossy(&m.line);

        if config.multi_file {
            if config.color {
                write!(writer, "{COLOR_FILENAME}{path_str}{COLOR_RESET}{COLOR_SEP}:{COLOR_RESET}")?;
            } else {
                write!(writer, "{path_str}:")?;
            }
        }

        if config.line_number {
            if config.color {
                write!(
                    writer,
                    "{COLOR_LINE_NO}{}{COLOR_RESET}{COLOR_SEP}:{COLOR_RESET}",
                    m.line_no
                )?;
            } else {
                write!(writer, "{}:", m.line_no)?;
            }
        }

        if config.color && !m.match_ranges.is_empty() {
            let mut last_end = 0;
            for range in &m.match_ranges {
                write!(writer, "{}", &line_str[last_end..range.start])?;
                write!(writer, "{COLOR_MATCH}{}{COLOR_RESET}", &line_str[range.start..range.end])?;
                last_end = range.end;
            }
            writeln!(writer, "{}", &line_str[last_end..])?;
        } else {
            writeln!(writer, "{line_str}")?;
        }
    }

    Ok(())
}
