//! Fastgrep — a parallel grep implementation with lazy caching.
//!
//! Provides a GNU grep-compatible interface that runs searches across
//! all available CPU threads and persists results in
//! `~/.cache/fastgrep/` so that repeated queries are near-instant.
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

pub mod cache;
pub mod cli;
pub mod output;
pub mod pattern;
pub mod searcher;
pub mod threadpool;
pub mod walker;
