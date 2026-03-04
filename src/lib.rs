//! Fastgrep — a parallel grep implementation with trigram indexing.
//!
//! Provides a GNU grep-compatible interface that runs searches across
//! all available CPU threads. Builds a trigram content index on first
//! run to accelerate subsequent searches for any pattern.
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
