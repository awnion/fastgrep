use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use kanal::Sender;

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

/// Returns `true` if `file_name` passes include/exclude glob filters.
fn passes_filter(include: &[String], exclude: &[String], file_name: &str) -> bool {
    if !include.is_empty() && !include.iter().any(|g| glob_matches(g, file_name)) {
        return false;
    }
    if exclude.iter().any(|g| glob_matches(g, file_name)) {
        return false;
    }
    true
}

/// Traverses the paths listed in `config` and sends each regular file
/// into `tx`.
///
/// Plain files are sent directly. Directories are walked recursively
/// (when `-r` is active) using `walk_threads` parallel threads that
/// cooperatively traverse the directory tree via a shared work queue.
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
/// walk(&config, tx, 4);
/// for path in rx {
///     println!("{}", path.display());
/// }
/// ```
pub fn walk(config: &ResolvedConfig, tx: Sender<PathBuf>, walk_threads: usize) {
    let (dir_tx, dir_rx) = kanal::bounded::<PathBuf>(256);
    let active = AtomicUsize::new(0);

    // Seed: send files directly, push directories onto work queue
    for path in &config.paths {
        if path.is_file() {
            let _ = tx.send(path.clone());
            continue;
        }

        if config.recursive {
            active.fetch_add(1, Ordering::SeqCst);
            let _ = dir_tx.send(path.clone());
        }
    }

    // If nothing to walk, return early
    if active.load(Ordering::SeqCst) == 0 {
        return;
    }

    let include = &config.include;
    let exclude = &config.exclude;

    std::thread::scope(|s| {
        for _ in 0..walk_threads {
            let dir_rx = &dir_rx;
            let dir_tx = &dir_tx;
            let tx = &tx;
            let active = &active;

            s.spawn(move || {
                loop {
                    match dir_rx.try_recv() {
                        Ok(Some(dir)) => {
                            process_directory(&dir, include, exclude, tx, dir_tx, active);
                        }
                        _ => {
                            if active.load(Ordering::SeqCst) == 0 {
                                break;
                            }
                            std::thread::yield_now();
                        }
                    }
                }
            });
        }
    });
}

/// Process a single directory: send files to `file_tx`, push subdirs to `dir_tx`.
fn process_directory(
    dir: &PathBuf,
    include: &[String],
    exclude: &[String],
    file_tx: &Sender<PathBuf>,
    dir_tx: &kanal::Sender<PathBuf>,
    active: &AtomicUsize,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            let Ok(entry) = entry else { continue };
            let Ok(ft) = entry.file_type() else { continue };

            if ft.is_dir() {
                active.fetch_add(1, Ordering::SeqCst);
                let _ = dir_tx.send(entry.path());
            } else if ft.is_file() {
                let file_name = entry.file_name().to_string_lossy().into_owned();
                if passes_filter(include, exclude, &file_name) {
                    let _ = file_tx.send(entry.path());
                }
            }
        }
    }
    active.fetch_sub(1, Ordering::SeqCst);
}
