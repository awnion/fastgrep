mod corpus;

use std::process::Command;

use corpus::generate_corpus;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;

fn fastgrep_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    path.push("grep");
    if !path.exists() {
        let output =
            Command::new("cargo").args(["metadata", "--format-version", "1"]).output().unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let target_dir = meta["target_directory"].as_str().unwrap();
        path = std::path::PathBuf::from(target_dir).join("release").join("grep");
        if !path.exists() {
            path = std::path::PathBuf::from(target_dir).join("debug").join("grep");
        }
    }
    path
}

/// Build the trigram index for a directory by running fastgrep once.
fn build_index(fastgrep: &std::path::Path, dir: &str) {
    Command::new(fastgrep).args(["-rl", ".", dir]).output().unwrap();
}

/// Clear the trigram index cache.
fn clear_index() {
    if let Some(cache) = dirs::cache_dir() {
        let _ = std::fs::remove_dir_all(cache.join("fastgrep").join("trigram"));
    }
}

// ---------------------------------------------------------------------------
// Benchmark groups
// ---------------------------------------------------------------------------

/// -rn literal sparse pattern ("fn main")
fn bench_rn_literal_sparse(c: &mut Criterion) {
    let dir = generate_corpus(200, 5_000);
    let dir_str = dir.path().to_str().unwrap().to_string();
    let fg = fastgrep_bin();

    clear_index();
    build_index(&fg, &dir_str);

    let mut group = c.benchmark_group("rn_literal_sparse");

    group.bench_function("fastgrep_index", |b| {
        b.iter(|| {
            let out = Command::new(&fg).args(["-rn", "fn main", &dir_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep_no_index", |b| {
        b.iter(|| {
            let out = Command::new(&fg)
                .args(["--no-index", "-rn", "fn main", &dir_str])
                .output()
                .unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

/// -rl literal pattern ("fn main")
fn bench_rl_literal(c: &mut Criterion) {
    let dir = generate_corpus(200, 5_000);
    let dir_str = dir.path().to_str().unwrap().to_string();
    let fg = fastgrep_bin();

    clear_index();
    build_index(&fg, &dir_str);

    let mut group = c.benchmark_group("rl_literal");

    group.bench_function("fastgrep_index", |b| {
        b.iter(|| {
            let out = Command::new(&fg).args(["-rl", "fn main", &dir_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep_no_index", |b| {
        b.iter(|| {
            let out = Command::new(&fg)
                .args(["--no-index", "-rl", "fn main", &dir_str])
                .output()
                .unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

/// -rc dense pattern ("use ")
fn bench_rc_dense(c: &mut Criterion) {
    let dir = generate_corpus(200, 5_000);
    let dir_str = dir.path().to_str().unwrap().to_string();
    let fg = fastgrep_bin();

    clear_index();
    build_index(&fg, &dir_str);

    let mut group = c.benchmark_group("rc_dense");

    group.bench_function("fastgrep_index", |b| {
        b.iter(|| {
            let out = Command::new(&fg).args(["-rc", "use ", &dir_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep_no_index", |b| {
        b.iter(|| {
            let out =
                Command::new(&fg).args(["--no-index", "-rc", "use ", &dir_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

/// -rni case-insensitive ("error")
fn bench_rni_case_insensitive(c: &mut Criterion) {
    let dir = generate_corpus(200, 5_000);
    let dir_str = dir.path().to_str().unwrap().to_string();
    let fg = fastgrep_bin();

    let mut group = c.benchmark_group("rni_case_insensitive");

    group.bench_function("fastgrep", |b| {
        b.iter(|| {
            let out = Command::new(&fg).args(["-rni", "error", &dir_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

/// -rn regex with extractable prefix ("impl\s+Drop")
fn bench_rn_regex_prefix(c: &mut Criterion) {
    let dir = generate_corpus(200, 5_000);
    let dir_str = dir.path().to_str().unwrap().to_string();
    let fg = fastgrep_bin();

    clear_index();
    build_index(&fg, &dir_str);

    let mut group = c.benchmark_group("rn_regex_prefix");

    group.bench_function("fastgrep_index", |b| {
        b.iter(|| {
            let out = Command::new(&fg).args(["-rn", r"impl\s+Drop", &dir_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep_no_index", |b| {
        b.iter(|| {
            let out = Command::new(&fg)
                .args(["--no-index", "-rn", r"impl\s+Drop", &dir_str])
                .output()
                .unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

/// -rn very sparse literal ("SubscriptionManager")
fn bench_rn_very_sparse(c: &mut Criterion) {
    let dir = generate_corpus(200, 5_000);
    let dir_str = dir.path().to_str().unwrap().to_string();
    let fg = fastgrep_bin();

    clear_index();
    build_index(&fg, &dir_str);

    let mut group = c.benchmark_group("rn_very_sparse");

    group.bench_function("fastgrep_index", |b| {
        b.iter(|| {
            let out =
                Command::new(&fg).args(["-rn", "SubscriptionManager", &dir_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep_no_index", |b| {
        b.iter(|| {
            let out = Command::new(&fg)
                .args(["--no-index", "-rn", "SubscriptionManager", &dir_str])
                .output()
                .unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

/// Single file search (no index applicable, raw speed comparison)
fn bench_single_file(c: &mut Criterion) {
    let dir = generate_corpus(1, 100_000);
    let file_path = dir.path().join("src").join("module_0000.rs");
    let file_str = file_path.to_str().unwrap().to_string();
    let fg = fastgrep_bin();

    let mut group = c.benchmark_group("single_file_100k_lines");

    group.bench_function("fastgrep", |b| {
        b.iter(|| {
            let out = Command::new(&fg).args(["-n", "fn main", &file_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

/// Scaling: increasing file count with index
fn bench_scaling(c: &mut Criterion) {
    let fg = fastgrep_bin();
    let mut group = c.benchmark_group("scaling_with_index");
    group.sample_size(10);

    for num_files in [50, 200, 500] {
        let dir = generate_corpus(num_files, 2_000);
        let dir_str = dir.path().to_str().unwrap().to_string();

        clear_index();
        build_index(&fg, &dir_str);

        group.bench_with_input(
            BenchmarkId::new("fastgrep_index", num_files),
            &dir_str,
            |b, dir_str| {
                b.iter(|| {
                    Command::new(&fg).args(["-rn", "fn main", dir_str]).output().unwrap();
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fastgrep_no_index", num_files),
            &dir_str,
            |b, dir_str| {
                b.iter(|| {
                    Command::new(&fg)
                        .args(["--no-index", "-rn", "fn main", dir_str])
                        .output()
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_rn_literal_sparse,
    bench_rl_literal,
    bench_rc_dense,
    bench_rni_case_insensitive,
    bench_rn_regex_prefix,
    bench_rn_very_sparse,
    bench_single_file,
    bench_scaling,
);
criterion_main!(benches);
