//! Fastgrep — a parallel grep implementation with trigram indexing.
//!
//! Provides a GNU grep-compatible interface that runs searches across
//! all available CPU threads. Builds a trigram content index on first
//! run to accelerate subsequent searches for selective patterns.
//!
//! # Benchmarks: fastgrep versus GNU grep
//!
//! Last benchmark run: **2026-10-05**. Measured on Apple M2 Max (12 logical CPUs), 32 GB RAM,
//! macOS 27.0.1. fastgrep 0.1.9 at commit `5094623`, built with Rust 1.99.0
//! and the repository's release profile; GNU grep 3.12 (Homebrew `ggrep`).
//! Both suites use `LC_ALL=C` and default thread counts.
//!
//! The generated Rust-like corpus contains 200 files with 5,000 lines each.
//! The single-file workload contains 100,000 lines.
//!
//! ```text
//! +---------------------------------------+-----------+------------+----------+---------+
//! | Workload                              | fastgrep* | --no-index | GNU grep | GNU/fg* |
//! +---------------------------------------+-----------+------------+----------+---------+
//! | -rn "fn main" (sparse)                |   8.82 ms |    7.31 ms | 32.99 ms |   3.74x |
//! | -rl "fn main"                         |   7.50 ms |    5.88 ms |  7.12 ms |   0.95x |
//! | -rc "use " (dense)                    |   8.55 ms |    7.64 ms | 84.75 ms |   9.91x |
//! | -rni "error"                          |  37.49 ms |          - | 82.57 ms |   2.20x |
//! | -rn impl\s+Drop                       |  45.24 ms |   35.31 ms | 73.43 ms |   1.62x |
//! | -rn SubscriptionManager (very sparse) |   5.41 ms |    6.30 ms | 39.35 ms |   7.28x |
//! | Single file (100k lines)              |   3.70 ms |          - |  5.21 ms |   1.41x |
//! +---------------------------------------+-----------+------------+----------+---------+
//! ```
//!
//! Scaling (2,000 lines per file):
//!
//! ```text
//! +-------+-----------+------------+----------+---------+
//! | Files | fastgrep* | --no-index | GNU grep | GNU/fg* |
//! +-------+-----------+------------+----------+---------+
//! | 50    |   4.17 ms |    3.43 ms |  6.17 ms |   1.48x |
//! | 200   |   7.02 ms |    5.70 ms | 17.07 ms |   2.43x |
//! | 500   |  12.50 ms |    9.65 ms | 38.49 ms |   3.08x |
//! +-------+-----------+------------+----------+---------+
//! ```
//!
//! `fastgrep*` is the default search with a warm index where applicable.
//! `--no-index` is fastgrep with indexing disabled. Case-insensitive searches
//! bypass the index; single-file searches do not use it. A dash means the
//! harness does not measure a separate `--no-index` variant for that workload.
//! `GNU/fg*` is GNU grep time divided by default fastgrep time; below 1 means
//! GNU grep is faster. Ratios use unrounded estimates.
//!
//! Timings are Criterion 0.8.2 central estimates, including process startup
//! and captured output (`Command::output()`). Each workload has a 3-second
//! warmup and a 5-second target measurement period, with 100 samples for
//! main workloads and 10 for scaling. Filesystem pages are warm. Initial
//! index construction is excluded, and the benchmark uses an isolated cache.
//! GNU grep's basic regex query `impl\s\+Drop` is equivalent to fastgrep's
//! `impl\s+Drop`. Sorted output was checked for all seven query types.
//!
//! On this corpus, every file contains `fn main`, `use `, and `impl Drop`;
//! only every tenth file contains `SubscriptionManager`. The index can add
//! metadata overhead when it cannot skip files. These measurements do not
//! establish a general speedup for other corpora, hardware, or cache states.
//!
//! See the [full report, confidence intervals, and reproduction commands](https://github.com/awnion/fastgrep/blob/main/docs/benchmarks/README.md).
//!
//! # Example
//!
//! ```no_run
//! use clap::Parser;
//! use fastgrep::cli::Cli;
//! use fastgrep::pattern::CompiledPattern;
//! use fastgrep::searcher::search_file;
//!
//! let cli = Cli::parse();
//! let config = cli.resolve();
//! let pattern = CompiledPattern::compile(&config).unwrap();
//! let result = search_file(config.paths[0].as_path(), &pattern, false, true, false).unwrap();
//! println!("found {} matches", result.matches.len());
//! ```

pub mod cli;
pub mod output;
pub mod pattern;
pub mod searcher;
pub mod threadpool;
pub mod trigram;
pub mod walker;
