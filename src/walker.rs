use std::path::PathBuf;

use crossbeam_channel::Sender;
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;

use crate::cli::ResolvedConfig;

/// Traverses the paths listed in `config` and sends each regular file
/// into `tx`.
///
/// Plain files are sent directly. Directories are walked recursively
/// (when `-r` is active) using the [`ignore`] crate, which honours
/// `.gitignore` rules and applies `--include` / `--exclude` globs.
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
/// let (tx, rx) = crossbeam_channel::bounded(64);
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

        let mut builder = WalkBuilder::new(path);
        builder.hidden(false).git_ignore(true);

        let mut overrides = OverrideBuilder::new(path);
        for glob in &config.include {
            let _ = overrides.add(glob);
        }
        for glob in &config.exclude {
            let _ = overrides.add(&format!("!{glob}"));
        }
        if let Ok(ov) = overrides.build() {
            builder.overrides(ov);
        }

        for entry in builder.build() {
            let Ok(entry) = entry else { continue };
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                let _ = tx.send(entry.into_path());
            }
        }
    }
}
