use std::path::PathBuf;

use kanal::Sender;
use walkdir::WalkDir;

use crate::cli::ResolvedConfig;

/// Returns `true` if `name` matches a simple glob pattern (e.g. `*.rs`, `Makefile`).
fn glob_matches(pattern: &str, name: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix('*') {
        name.ends_with(suffix)
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        name.starts_with(prefix)
    } else {
        name == pattern
    }
}

/// Traverses the paths listed in `config` and sends each regular file
/// into `tx`.
///
/// Plain files are sent directly. Directories are walked recursively
/// (when `-r` is active) using [`walkdir`] with no filtering (to match
/// GNU grep behaviour), applying only `--include` / `--exclude` globs.
///
/// The function returns once every path has been visited; the caller
/// typically runs it in a dedicated thread so that searcher threads
/// can consume paths concurrently.
///
/// # Example
///
/// ```no_run
/// use clap::Parser;
/// use fastgrep::cli::Cli;
/// use fastgrep::walker::walk;
///
/// let cli = Cli::parse_from(["grep", "-r", "pattern", "src/"]);
/// let config = cli.resolve();
/// let (tx, rx) = kanal::bounded(64);
/// walk(&config, tx);
/// for path in rx {
///     println!("{}", path.display());
/// }
/// ```
pub fn walk(config: &ResolvedConfig, tx: Sender<PathBuf>) {
    for path in &config.paths {
        if path.is_file() {
            let _ = tx.send(path.clone());
            continue;
        }

        if !config.recursive {
            continue;
        }

        for entry in WalkDir::new(path) {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy();

            // --include: if specified, only matching files pass
            if !config.include.is_empty()
                && !config.include.iter().any(|g| glob_matches(g, &file_name))
            {
                continue;
            }

            // --exclude: skip matching files
            if config
                .exclude
                .iter()
                .any(|g| glob_matches(g, &file_name))
            {
                continue;
            }

            let _ = tx.send(entry.into_path());
        }
    }
}
