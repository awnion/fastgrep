use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use kanal::Sender;

use crate::cli::ResolvedConfig;

/// A file that was skipped due to size limits.
pub struct SkippedFile {
    pub path: PathBuf,
    pub size: u64,
}

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
/// Files exceeding `max_file_size` (when `no_limit` is false) are
/// collected in `skipped` instead of being sent to the searcher.
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
/// let skipped = std::sync::Mutex::new(Vec::new());
/// walk(&config, tx, 4, &skipped);
/// for path in rx {
///     println!("{}", path.display());
/// }
/// ```
pub fn walk(
    config: &ResolvedConfig,
    tx: Sender<PathBuf>,
    walk_threads: usize,
    skipped: &Mutex<Vec<SkippedFile>>,
) {
    let (dir_tx, dir_rx) = kanal::bounded::<PathBuf>(256);
    let active = AtomicUsize::new(0);

    let max_file_size = if config.no_limit { u64::MAX } else { config.max_file_size };

    // Seed: send files directly, push directories onto work queue
    for path in &config.paths {
        if path.is_file() {
            if let Ok(meta) = std::fs::metadata(path) {
                let size = meta.len();
                if size > max_file_size {
                    if let Ok(mut s) = skipped.lock() {
                        s.push(SkippedFile { path: path.clone(), size });
                    }
                    continue;
                }
            }
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

    let exclude_dir = &config.exclude_dir;

    let ctx = WalkContext {
        include,
        exclude,
        exclude_dir,
        file_tx: &tx,
        dir_tx: &dir_tx,
        active: &active,
        max_file_size,
        skipped,
    };

    std::thread::scope(|s| {
        for _ in 0..walk_threads {
            let dir_rx = &dir_rx;
            let ctx = &ctx;

            s.spawn(move || {
                while let Ok(dir) = dir_rx.recv() {
                    process_directory(&dir, ctx);
                }
            });
        }
    });
}

/// Shared context for directory processing to avoid too many function arguments.
struct WalkContext<'a> {
    include: &'a [String],
    exclude: &'a [String],
    exclude_dir: &'a [String],
    file_tx: &'a Sender<PathBuf>,
    dir_tx: &'a kanal::Sender<PathBuf>,
    active: &'a AtomicUsize,
    max_file_size: u64,
    skipped: &'a Mutex<Vec<SkippedFile>>,
}

/// Process a single directory: send files to `file_tx`, push subdirs to `dir_tx`.
fn process_directory(dir: &PathBuf, ctx: &WalkContext<'_>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            let Ok(entry) = entry else { continue };
            let Ok(ft) = entry.file_type() else { continue };

            if ft.is_dir() {
                if !ctx.exclude_dir.is_empty() {
                    let dir_name = entry.file_name().to_string_lossy().into_owned();
                    if ctx.exclude_dir.iter().any(|g| glob_matches(g, &dir_name)) {
                        continue;
                    }
                }
                ctx.active.fetch_add(1, Ordering::Relaxed);
                let _ = ctx.dir_tx.send(entry.path());
            } else if ft.is_file() {
                let file_name = entry.file_name().to_string_lossy().into_owned();
                if passes_filter(ctx.include, ctx.exclude, &file_name) {
                    if ctx.max_file_size < u64::MAX
                        && let Ok(meta) = entry.metadata()
                    {
                        let size = meta.len();
                        if size > ctx.max_file_size {
                            if let Ok(mut s) = ctx.skipped.lock() {
                                s.push(SkippedFile { path: entry.path(), size });
                            }
                            continue;
                        }
                    }
                    let _ = ctx.file_tx.send(entry.path());
                }
            }
        }
    }
    if ctx.active.fetch_sub(1, Ordering::AcqRel) == 1 {
        let _ = ctx.dir_tx.close();
    }
}
