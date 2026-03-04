use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;
use crossbeam_channel::bounded;
use fastgrep::cache::CacheIndex;
use fastgrep::cli::Cli;
use fastgrep::output::OutputConfig;
use fastgrep::output::format_result;
use fastgrep::pattern::CompiledPattern;
use fastgrep::searcher::FileResult;
use fastgrep::searcher::search_file;
use fastgrep::searcher::search_reader;
use fastgrep::threadpool::ThreadPool;
use fastgrep::walker::walk;

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
    };

    if config.stdin {
        return run_stdin(&pattern, &output_config, config.invert_match);
    }

    run_files(config, pattern, output_config)
}

fn run_stdin(
    pattern: &CompiledPattern,
    output_config: &OutputConfig,
    invert_match: bool,
) -> ExitCode {
    let mut stdin = std::io::stdin().lock();
    let result = match search_reader(&mut stdin, pattern, invert_match) {
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
    let cache = if config.no_cache { None } else { CacheIndex::load(&pattern.cache_key) };
    let cache = Arc::new(cache);

    let invert_match = config.invert_match;
    let files_with_matches = config.files_with_matches;
    let no_cache = config.no_cache;
    let threads = config.threads;

    let (path_tx, path_rx) = bounded::<PathBuf>(256);
    let (result_tx, result_rx) = bounded::<FileResult>(256);

    let walker_handle = std::thread::Builder::new()
        .name("fg-walker".into())
        .spawn(move || {
            walk(&config, path_tx);
        })
        .expect("failed to spawn walker thread");

    let pool = ThreadPool::new(threads, "fg-search", {
        let pattern = Arc::clone(&pattern);
        let cache = Arc::clone(&cache);
        let result_tx = result_tx.clone();
        move || {
            let pattern = Arc::clone(&pattern);
            let cache = Arc::clone(&cache);
            let result_tx = result_tx.clone();
            while let Ok(path) = path_rx.recv() {
                if let Some(ref idx) = *cache
                    && let Some(cached) = idx.lookup(&path)
                    && files_with_matches
                {
                    if !cached.line_nos.is_empty() {
                        let _ = result_tx.send(FileResult {
                            path,
                            matches: vec![fastgrep::searcher::LineMatch {
                                line_no: 0,
                                line: Vec::new(),
                                match_ranges: Vec::new(),
                                byte_offset: 0,
                                line_len: 0,
                            }],
                            is_binary: false,
                        });
                    }
                    continue;
                }

                match search_file(&path, &pattern, invert_match) {
                    Ok(result) => {
                        let _ = result_tx.send(result);
                    }
                    Err(e) => {
                        eprintln!("grep: {}: {e}", path.display());
                    }
                }
            }
        }
    });
    drop(result_tx);

    let stdout = std::io::stdout().lock();
    let mut writer = BufWriter::new(stdout);
    let mut found_match = false;

    let mut new_entries: Vec<(PathBuf, fastgrep::cache::CacheEntry)> = Vec::new();

    for result in result_rx {
        if !result.matches.is_empty() {
            found_match = true;
        }

        if !no_cache && !result.is_binary {
            new_entries.push((result.path.clone(), result.to_cache_entry()));
        }

        if let Err(e) = format_result(&result, &output_config, &mut writer) {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                break;
            }
            eprintln!("grep: write error: {e}");
        }
    }

    let _ = writer.flush();

    walker_handle.join().ok();
    pool.join();

    if !no_cache && !new_entries.is_empty() {
        if let Some(ref idx) = *cache {
            for (path, entry) in &new_entries {
                let _ = idx.append(path, entry);
            }
        } else if let Some(idx) = CacheIndex::load(&pattern.cache_key) {
            for (path, entry) in &new_entries {
                let _ = idx.append(path, entry);
            }
        } else {
            let key = &pattern.cache_key;
            if let Ok(home) = std::env::var("HOME") {
                let cache_dir = std::path::PathBuf::from(home)
                    .join(".cache")
                    .join("fastgrep")
                    .join("v1")
                    .join(key);
                let _ = std::fs::create_dir_all(&cache_dir);
                if let Some(idx) = CacheIndex::load(key) {
                    for (path, entry) in &new_entries {
                        let _ = idx.append(path, entry);
                    }
                }
            }
        }
    }

    if found_match { ExitCode::SUCCESS } else { ExitCode::from(1) }
}
