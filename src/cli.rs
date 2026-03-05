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
#[command(name = "grep", about = "fastgrep - parallel grep with lazy caching")]
pub struct Cli {
    /// Search pattern (positional, unless -e is used)
    #[arg(value_name = "PATTERN", required_unless_present = "patterns")]
    pub pattern: Option<String>,

    /// Files or directories to search
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Specify pattern(s) via -e (can repeat)
    #[arg(short = 'e', long = "regexp", value_name = "PATTERN")]
    pub patterns: Vec<String>,

    /// Recurse into directories
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,

    /// Case-insensitive matching
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// Show line numbers
    #[arg(short = 'n', long = "line-number")]
    pub line_number: bool,

    /// Print only filenames of matching files
    #[arg(short = 'l', long = "files-with-matches")]
    pub files_with_matches: bool,

    /// Print only a count of matching lines per file
    #[arg(short = 'c', long = "count")]
    pub count: bool,

    /// Invert match (select non-matching lines)
    #[arg(short = 'v', long = "invert-match")]
    pub invert_match: bool,

    /// Match whole words only
    #[arg(short = 'w', long = "word-regexp")]
    pub word_regexp: bool,

    /// Extended regex (ERE) — accepted for compatibility, this is the default
    #[arg(short = 'E', long = "extended-regexp")]
    pub extended_regexp: bool,

    /// Fixed string matching (no regex)
    #[arg(short = 'F', long = "fixed-strings")]
    pub fixed_strings: bool,

    /// Colorize output
    #[arg(long = "color", value_enum, default_value = "auto")]
    pub color: ColorMode,

    /// Search only files matching GLOB
    #[arg(long = "include", value_name = "GLOB")]
    pub include: Vec<String>,

    /// Skip files matching GLOB
    #[arg(long = "exclude", value_name = "GLOB")]
    pub exclude: Vec<String>,

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
    pub count: bool,
    pub invert_match: bool,
    pub word_regexp: bool,
    pub fixed_strings: bool,
    pub color: bool,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub threads: usize,
    pub no_index: bool,
    pub multi_file: bool,
    pub stdin: bool,
    pub max_line_len: usize,
    pub max_file_size: u64,
    pub no_limit: bool,
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
            recursive,
            ignore_case,
            line_number,
            files_with_matches,
            count,
            invert_match,
            word_regexp,
            extended_regexp: _,
            fixed_strings,
            color,
            include,
            exclude,
            threads,
            no_index,
            max_line_len,
            max_file_size,
        } = self;

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

        let no_limit = std::env::var("FASTGREP_NO_LIMIT").is_ok_and(|v| v == "1");

        let multi_file = paths.len() > 1 || recursive;

        ResolvedConfig {
            patterns,
            paths,
            recursive,
            ignore_case,
            line_number,
            files_with_matches,
            count,
            invert_match,
            word_regexp,
            fixed_strings,
            color,
            include,
            exclude,
            threads,
            no_index,
            multi_file,
            stdin,
            max_line_len,
            max_file_size,
            no_limit,
        }
    }
}
