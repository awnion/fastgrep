#![allow(dead_code)]

use std::io::Write;

use tempfile::TempDir;

/// Generates a realistic source-code-like corpus.
///
/// - `num_files` files, each with `lines_per_file` lines
/// - ~1% lines contain "fn main" (sparse literal)
/// - ~5% lines contain "use " imports (dense literal)
/// - ~2% lines contain "error" (medium density)
/// - ~1% lines contain "SubscriptionManager" (very sparse)
/// - ~3% lines contain function defs matching `fn\s+\w+_test` (regex target)
/// - Rest is filler code-like text
pub fn generate_corpus(num_files: usize, lines_per_file: usize) -> TempDir {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    for i in 0..num_files {
        let path = src.join(format!("module_{i:04}.rs"));
        let mut f = std::fs::File::create(&path).unwrap();
        for j in 0..lines_per_file {
            let line = match j % 100 {
                0 => format!("fn main() {{ println!(\"entry point {i}/{j}\"); }}"),
                3 | 7 | 15 | 33 | 67 => {
                    format!("use std::collections::HashMap; // import line {j}")
                }
                10 => format!("    eprintln!(\"error: failed to process item {j}\");"),
                20 => {
                    if i % 10 == 0 {
                        format!("impl SubscriptionManager {{ fn handle_{j}(&self) {{ }} }}")
                    } else {
                        format!("    let result_{j} = compute_value({j});")
                    }
                }
                30 | 60 | 90 => format!(
                    "fn process_test_{j}() -> Result<(), Box<dyn std::error::Error>> {{ Ok(()) }}"
                ),
                50 => format!(
                    "impl Drop for Resource_{i} {{ fn drop(&mut self) {{ cleanup({j}); }} }}"
                ),
                _ => format!(
                    "    let var_{j} = data.iter().map(|x| x * 2).filter(|x| *x > {j}).collect::<Vec<_>>();"
                ),
            };
            writeln!(f, "{line}").unwrap();
        }
    }
    dir
}
