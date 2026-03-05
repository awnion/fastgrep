use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use clap::Parser;
use fastgrep::cli::Cli;
use fastgrep::output::OutputConfig;
use fastgrep::output::format_result;
use fastgrep::pattern::CompiledPattern;
use fastgrep::searcher::search_file_streaming;
use fastgrep::searcher::search_file_streaming_reuse;
use fastgrep::searcher::search_reader;
use fastgrep::threadpool::ThreadPool;
use fastgrep::trigram::TrigramIndex;
use fastgrep::trigram::evict_if_needed;
use fastgrep::walker::walk;
use kanal::bounded;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let config = cli.resolve();

    let pattern = match CompiledPattern::compile(&config) {
        Ok(p) => Arc::new(p),
        Err(e) => {
            eprintln!("grep: {e}");
            return ExitCode::from(2);
        }
    };

    let output_config = OutputConfig {
        color: config.color,
        line_number: config.line_number,
        files_with_matches: config.files_with_matches,
        count: config.count,
        multi_file: config.multi_file,
        max_line_len: config.max_line_len,
    };

    if config.stdin {
        return run_stdin(&pattern, &output_config, config.invert_match);
    }

    // Fast path: single file, no recursion — skip thread pool/channel overhead
    if config.paths.len() == 1 && !config.recursive {
        let path = &config.paths[0];
        if path.is_file() {
            return run_single_file(path, &pattern, &output_config, config.invert_match);
        }
    }

    run_files(config, pattern, output_config)
}

fn run_single_file(
    path: &std::path::Path,
    pattern: &CompiledPattern,
    output_config: &OutputConfig,
    invert_match: bool,
) -> ExitCode {
    let stdout = std::io::stdout().lock();
    let mut writer = BufWriter::new(stdout);

    let count = match search_file_streaming(path, pattern, invert_match, output_config, &mut writer)
    {
        Ok(c) => c,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                eprintln!("grep: {}: {e}", path.display());
            }
            return ExitCode::from(2);
        }
    };

    let _ = writer.flush();
    if count > 0 { ExitCode::SUCCESS } else { ExitCode::from(1) }
}

fn run_stdin(
    pattern: &CompiledPattern,
    output_config: &OutputConfig,
    invert_match: bool,
) -> ExitCode {
    let mut stdin = std::io::stdin().lock();
    let need_ranges =
        output_config.color && !output_config.files_with_matches && !output_config.count;
    let count_only = output_config.count || output_config.files_with_matches;
    let result = match search_reader(&mut stdin, pattern, invert_match, need_ranges, count_only) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("grep: (stdin): {e}");
            return ExitCode::from(2);
        }
    };

    let found_match = !result.matches.is_empty();

    let stdout = std::io::stdout().lock();
    let mut writer = BufWriter::new(stdout);
    if let Err(e) = format_result(&result, output_config, &mut writer)
        && e.kind() != std::io::ErrorKind::BrokenPipe
    {
        eprintln!("grep: write error: {e}");
    }
    let _ = writer.flush();

    if found_match { ExitCode::SUCCESS } else { ExitCode::from(1) }
}

fn run_files(
    config: fastgrep::cli::ResolvedConfig,
    pattern: Arc<CompiledPattern>,
    output_config: OutputConfig,
) -> ExitCode {
    let no_index = config.no_index;
    let invert_match = config.invert_match;
    let threads = config.threads;

    // --- Trigram index: load and compute candidate filter ---
    let search_root = config
        .paths
        .first()
        .and_then(|p| if config.recursive { std::fs::canonicalize(p).ok() } else { None });

    let (candidate_filter, index_loaded) = if !no_index && let Some(ref root) = search_root {
        let trigrams = pattern.required_trigrams();
        if let Some(index) = TrigramIndex::load(root) {
            if !trigrams.is_empty() && !index.needs_rebuild() {
                let mut candidates = index.candidate_files(&trigrams);
                let total = index.file_count();
                // Skip filtering when trigrams are too common (>= 90% of files match)
                if total > 0 && candidates.len() * 10 >= total * 9 {
                    (None, true)
                } else {
                    // Include stale files so they get searched normally
                    for stale in index.stale_files() {
                        candidates.insert(stale);
                    }
                    (Some(candidates), true)
                }
            } else {
                (None, true)
            }
        } else {
            (None, false)
        }
    } else {
        (None, false)
    };

    let candidate_filter = candidate_filter.map(Arc::new);
    let should_build_index = !no_index && search_root.is_some() && !index_loaded;

    let (path_tx, path_rx) = bounded::<PathBuf>(256);

    // Channel to collect walked paths for index building on first run
    let (walked_send, walked_recv) = if should_build_index {
        let (s, r) = kanal::unbounded::<PathBuf>();
        (Some(s), Some(r))
    } else {
        (None, None)
    };

    let filter_for_walker = candidate_filter.clone();
    let walker_handle = std::thread::Builder::new()
        .name("fg-walker".into())
        .spawn(move || {
            let (tx_inner, rx_inner) = bounded::<PathBuf>(256);
            std::thread::scope(|s| {
                let config_ref = &config;
                s.spawn(|| {
                    let walk_threads = (config_ref.threads / 4).clamp(2, 4);
                    walk(config_ref, tx_inner, walk_threads);
                });
                for p in rx_inner {
                    if let Some(ref filter) = filter_for_walker
                        && !filter.contains(&p)
                    {
                        continue;
                    }
                    if let Some(ref wtx) = walked_send {
                        let _ = wtx.send(p.clone());
                    }
                    let _ = path_tx.send(p);
                }
            });
            // Drop walked_send to close the channel
            drop(walked_send);
        })
        .expect("failed to spawn walker thread");

    // Shared stdout writer behind a mutex — workers flush per-file buffers here.
    let shared_writer = Arc::new(Mutex::new(BufWriter::new(std::io::stdout())));
    let found_match = Arc::new(AtomicBool::new(false));

    let pool = ThreadPool::new(threads, "fg-search", {
        let pattern = Arc::clone(&pattern);
        let shared_writer = Arc::clone(&shared_writer);
        let found_match = Arc::clone(&found_match);
        let output_config = output_config.clone();
        move || {
            let pattern = Arc::clone(&pattern);
            let shared_writer = Arc::clone(&shared_writer);
            let found_match = Arc::clone(&found_match);
            let output_config = output_config.clone();
            // Per-thread buffers: reusable read buffer + output buffer
            let mut read_buf = Vec::with_capacity(256 * 1024);
            let mut out_buf: Vec<u8> = Vec::with_capacity(64 * 1024);
            while let Ok(path) = path_rx.recv() {
                out_buf.clear();
                match search_file_streaming_reuse(
                    &path,
                    &pattern,
                    invert_match,
                    &output_config,
                    &mut out_buf,
                    &mut read_buf,
                ) {
                    Ok(count) => {
                        if count > 0 {
                            found_match.store(true, Ordering::Relaxed);
                        }
                        // For -c mode, always flush (to show file:0).
                        // For other modes, only flush when there are matches.
                        if (count > 0 || output_config.count)
                            && let Ok(mut w) = shared_writer.lock()
                            && let Err(e) = w.write_all(&out_buf)
                            && e.kind() == std::io::ErrorKind::BrokenPipe
                        {
                            break;
                        }
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::BrokenPipe {
                            eprintln!("grep: {}: {e}", path.display());
                        }
                    }
                }
            }
        }
    });

    pool.join();
    if let Ok(mut w) = shared_writer.lock() {
        let _ = w.flush();
    }

    walker_handle.join().ok();

    // Build trigram index after first run
    if should_build_index
        && let Some(ref root) = search_root
        && let Some(rx) = walked_recv
    {
        let paths: Vec<PathBuf> = std::iter::from_fn(|| rx.try_recv().ok().flatten()).collect();
        if !paths.is_empty() {
            let index = TrigramIndex::build(root, &paths);
            let _ = index.save();
            evict_if_needed();
        }
    }

    if found_match.load(Ordering::Relaxed) { ExitCode::SUCCESS } else { ExitCode::from(1) }
}
