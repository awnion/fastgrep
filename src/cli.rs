use std::io::IsTerminal;
use std::path::PathBuf;

use clap::Parser;
use clap::ValueEnum;

/// Controls when colored output is used.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

/// Controls how matches are emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Text,
    Json,
}

/// Command-line arguments accepted by the `grep` binary.
///
/// Mirrors a subset of the GNU grep interface with additional
/// flags for thread count and caching control.
///
/// # Example
///
/// ```no_run
/// use clap::Parser;
/// use fastgrep::cli::Cli;
///
/// let cli = Cli::parse();
/// let config = cli.resolve();
/// assert!(!config.patterns.is_empty());
/// ```
#[derive(Debug, Parser)]
#[command(
    name = "grep",
    about = "fastgrep - fast parallel grep optimized for AI agents and large codebases\nhttps://crates.io/crates/fastgrep",
    long_about = "fastgrep - fast parallel grep optimized for AI agents and large codebases\n\n\
        Designed for AI coding agents: faster search means fewer tokens spent waiting\n\
        and more time for reasoning. Drop-in grep replacement with SIMD-accelerated\n\
        search and trigram indexing.\n\n\
        https://crates.io/crates/fastgrep",
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_SHA"), ")"),
    disable_version_flag = true,
    disable_help_flag = true,
)]
pub struct Cli {
    /// Search pattern (positional, unless -e or -f is used)
    #[arg(value_name = "PATTERN", required_unless_present_any = ["patterns", "pattern_file", "version"])]
    pub pattern: Option<String>,

    /// Files or directories to search
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Specify pattern(s) via -e (can repeat)
    #[arg(short = 'e', long = "regexp", value_name = "PATTERN")]
    pub patterns: Vec<String>,

    /// Read patterns from file (one per line)
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub pattern_file: Vec<PathBuf>,

    /// Recurse into directories
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,

    /// Case-insensitive matching
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// Undo the effect of -i (case sensitive)
    #[arg(long = "no-ignore-case")]
    pub no_ignore_case: bool,

    /// Show line numbers
    #[arg(short = 'n', long = "line-number")]
    pub line_number: bool,

    /// Print only filenames of matching files
    #[arg(short = 'l', long = "files-with-matches")]
    pub files_with_matches: bool,

    /// Print only names of files with no matches
    #[arg(short = 'L', long = "files-without-match")]
    pub files_without_match: bool,

    /// Print only a count of matching lines per file
    #[arg(short = 'c', long = "count")]
    pub count: bool,

    /// Suppress all normal output
    #[arg(short = 'q', long = "quiet", alias = "silent")]
    pub quiet: bool,

    /// Stop after NUM matches per file
    #[arg(short = 'm', long = "max-count", value_name = "NUM")]
    pub max_count: Option<usize>,

    /// Invert match (select non-matching lines)
    #[arg(short = 'v', long = "invert-match")]
    pub invert_match: bool,

    /// Match whole words only
    #[arg(short = 'w', long = "word-regexp")]
    pub word_regexp: bool,

    /// Match whole lines only
    #[arg(short = 'x', long = "line-regexp")]
    pub line_regexp: bool,

    /// Extended regex (ERE) — accepted for compatibility, this is the default
    #[arg(short = 'E', long = "extended-regexp")]
    pub extended_regexp: bool,

    /// Print only the matched parts of a matching line
    #[arg(short = 'o', long = "only-matching")]
    pub only_matching: bool,

    /// Fixed string matching (no regex)
    #[arg(short = 'F', long = "fixed-strings")]
    pub fixed_strings: bool,

    /// Print NUM lines of trailing context after matches
    #[arg(short = 'A', long = "after-context", value_name = "NUM")]
    pub after_context: Option<usize>,

    /// Print NUM lines of leading context before matches
    #[arg(short = 'B', long = "before-context", value_name = "NUM")]
    pub before_context: Option<usize>,

    /// Print NUM lines of output context
    #[arg(short = 'C', long = "context", value_name = "NUM")]
    pub context: Option<usize>,

    /// Colorize output
    #[arg(long = "color", alias = "colour", value_enum, default_value = "auto")]
    pub color: ColorMode,

    /// Search only files matching GLOB
    #[arg(long = "include", value_name = "GLOB")]
    pub include: Vec<String>,

    /// Skip files matching GLOB
    #[arg(long = "exclude", value_name = "GLOB")]
    pub exclude: Vec<String>,

    /// Skip directories matching GLOB
    #[arg(long = "exclude-dir", value_name = "GLOB")]
    pub exclude_dir: Vec<String>,

    /// Suppress the file name prefix on output
    #[arg(short = 'h', long = "no-filename")]
    pub no_filename: bool,

    /// Print file name with output lines
    #[arg(short = 'H', long = "with-filename")]
    pub with_filename: bool,

    /// Suppress error messages about nonexistent or unreadable files
    #[arg(short = 's', long = "no-messages")]
    pub no_messages: bool,

    /// Print the byte offset within the input file before each line
    #[arg(short = 'b', long = "byte-offset")]
    pub byte_offset: bool,

    /// Treat binary files as if they do not contain matching data
    #[arg(short = 'I')]
    pub ignore_binary: bool,

    /// Custom separator between context groups (default: --)
    #[arg(long = "group-separator", value_name = "SEP")]
    pub group_separator: Option<String>,

    /// Suppress the separator between context groups
    #[arg(long = "no-group-separator")]
    pub no_group_separator: bool,

    /// Label for stdin in output (default: "(standard input)")
    #[arg(long = "label", value_name = "LABEL")]
    pub label: Option<String>,

    /// Align tabs in output (insert tab after prefix)
    #[arg(short = 'T', long = "initial-tab")]
    pub initial_tab: bool,

    /// Print NUL byte after filename instead of newline/colon
    #[arg(short = 'Z', long = "null")]
    pub null: bool,

    /// Read exclude patterns from file (one per line)
    #[arg(long = "exclude-from", value_name = "FILE")]
    pub exclude_from: Vec<PathBuf>,

    /// Treat binary files as text
    #[arg(short = 'a', long = "text")]
    pub text: bool,

    /// Do not strip CR at EOL (no-op on Unix)
    #[arg(short = 'U', long = "binary")]
    pub binary: bool,

    /// Number of threads (0 = all CPUs)
    #[arg(short = 'j', long = "threads", default_value = "0")]
    pub threads: usize,

    /// Disable trigram index
    #[arg(long = "no-index", alias = "no-cache")]
    pub no_index: bool,

    /// Max line length before truncation (0 = no limit)
    #[arg(long = "max-line-len", default_value = "15000", env = "FASTGREP_MAX_LINE_LEN")]
    pub max_line_len: usize,

    /// Max file size in bytes to search (skip larger files). Default 100 MiB.
    /// Set FASTGREP_NO_LIMIT=1 to disable this protection.
    #[arg(long = "max-file-size", default_value = "104857600", env = "FASTGREP_MAX_FILE_SIZE")]
    pub max_file_size: u64,

    /// Emit JSON Lines output for machine parsing.
    #[arg(long = "json")]
    pub json: bool,

    /// Print version
    #[arg(long = "version")]
    pub version: bool,

    /// Print help
    #[arg(long = "help", action = clap::ArgAction::Help)]
    pub help: Option<bool>,
}

/// Fully resolved configuration derived from [`Cli`] arguments.
///
/// All ambiguities (default paths, thread count, color mode) are
/// resolved at construction time so downstream code can use the
/// values directly.
pub struct ResolvedConfig {
    pub patterns: Vec<String>,
    pub paths: Vec<PathBuf>,
    pub recursive: bool,
    pub ignore_case: bool,
    pub line_number: bool,
    pub files_with_matches: bool,
    pub files_without_match: bool,
    pub count: bool,
    pub quiet: bool,
    pub max_count: usize,
    pub invert_match: bool,
    pub word_regexp: bool,
    pub line_regexp: bool,
    pub fixed_strings: bool,
    pub only_matching: bool,
    pub after_context: usize,
    pub before_context: usize,
    pub color: bool,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub exclude_dir: Vec<String>,
    pub no_messages: bool,
    pub byte_offset: bool,
    pub ignore_binary: bool,
    /// Group separator for context output. `None` = no separator, `Some(s)` = use `s`.
    pub group_separator: Option<String>,
    /// Label for stdin in output.
    pub label: Option<String>,
    /// Insert tab after prefix in output.
    pub initial_tab: bool,
    /// Print NUL after filename.
    pub null: bool,
    /// Treat binary files as text.
    pub text: bool,
    pub threads: usize,
    pub no_index: bool,
    pub multi_file: bool,
    pub stdin: bool,
    pub max_line_len: usize,
    pub max_file_size: u64,
    pub no_limit: bool,
    pub output_mode: OutputMode,
}

impl Cli {
    /// Consumes the parsed CLI and produces a [`ResolvedConfig`].
    ///
    /// Handles the positional-vs-`-e` pattern ambiguity, defaults the
    /// search path to `"."`, resolves thread count from
    /// `available_parallelism`, and evaluates the color mode against
    /// the current terminal state.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use clap::Parser;
    /// use fastgrep::cli::Cli;
    ///
    /// let cli = Cli::parse_from(["grep", "-rn", "TODO", "src/"]);
    /// let config = cli.resolve();
    /// assert_eq!(config.patterns, vec!["TODO"]);
    /// assert!(config.recursive);
    /// assert!(config.line_number);
    /// ```
    pub fn resolve(self) -> ResolvedConfig {
        let Cli {
            pattern,
            paths: cli_paths,
            patterns: cli_patterns,
            pattern_file,
            recursive,
            ignore_case,
            no_ignore_case,
            line_number,
            files_with_matches,
            files_without_match,
            count,
            quiet,
            max_count,
            invert_match,
            word_regexp,
            line_regexp,
            only_matching,
            extended_regexp: _,
            fixed_strings,
            after_context,
            before_context,
            context,
            color,
            include,
            mut exclude,
            exclude_dir,
            no_filename,
            with_filename,
            no_messages,
            byte_offset,
            ignore_binary,
            group_separator,
            no_group_separator,
            label,
            initial_tab,
            null,
            exclude_from,
            text,
            binary: _,
            threads,
            no_index,
            max_line_len,
            max_file_size,
            json,
            version: _,
            help: _,
        } = self;

        // Resolve --no-ignore-case: if both -i and --no-ignore-case are given,
        // the last one wins. Since clap doesn't track order, --no-ignore-case always wins.
        let ignore_case = if no_ignore_case { false } else { ignore_case };

        // Load patterns from -f FILE
        // Empty lines are kept: in GNU grep an empty pattern matches every line.
        let mut cli_patterns = cli_patterns;
        for pf in &pattern_file {
            if let Ok(content) = std::fs::read_to_string(pf) {
                for line in content.lines() {
                    cli_patterns.push(line.to_string());
                }
            }
        }

        // Load --exclude-from=FILE
        for ef in &exclude_from {
            if let Ok(content) = std::fs::read_to_string(ef) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() {
                        exclude.push(line.to_string());
                    }
                }
            }
        }

        let is_stdin_pipe = !std::io::stdin().is_terminal();

        let (patterns, paths) = match pattern {
            Some(p) if cli_patterns.is_empty() => {
                let mut patterns = cli_patterns;
                patterns.push(p);
                let paths = if cli_paths.is_empty() && !is_stdin_pipe {
                    vec![PathBuf::from(".")]
                } else {
                    cli_paths
                };
                (patterns, paths)
            }
            Some(p) => {
                let mut paths = vec![PathBuf::from(p)];
                paths.extend(cli_paths);
                (cli_patterns, paths)
            }
            None => {
                let paths = if cli_paths.is_empty() && !is_stdin_pipe {
                    vec![PathBuf::from(".")]
                } else {
                    cli_paths
                };
                (cli_patterns, paths)
            }
        };

        let stdin = paths.is_empty() && is_stdin_pipe;

        let threads = if threads == 0 {
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
        } else {
            threads
        };

        let color = match color {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => std::io::stdout().is_terminal(),
        };

        let after_context = if only_matching { 0 } else { after_context.or(context).unwrap_or(0) };
        let before_context =
            if only_matching { 0 } else { before_context.or(context).unwrap_or(0) };

        let no_limit = std::env::var("FASTGREP_NO_LIMIT").is_ok_and(|v| v == "1");

        let multi_file = if no_filename {
            false
        } else if with_filename {
            true
        } else {
            paths.len() > 1 || recursive
        };

        let max_count = max_count.unwrap_or(0);

        let group_separator = if no_group_separator {
            None
        } else {
            Some(group_separator.unwrap_or_else(|| "--".to_string()))
        };

        ResolvedConfig {
            patterns,
            paths,
            recursive,
            ignore_case,
            line_number,
            files_with_matches,
            files_without_match,
            count,
            quiet,
            max_count,
            invert_match,
            word_regexp,
            line_regexp,
            fixed_strings,
            only_matching,
            after_context,
            before_context,
            color,
            include,
            exclude,
            exclude_dir,
            no_messages,
            byte_offset,
            ignore_binary,
            group_separator,
            label,
            initial_tab,
            null,
            text,
            threads,
            no_index,
            multi_file,
            stdin,
            max_line_len,
            max_file_size,
            no_limit,
            output_mode: if json { OutputMode::Json } else { OutputMode::Text },
        }
    }
}
