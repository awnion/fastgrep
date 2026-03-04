use std::io::Write;
use std::process::Command;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use tempfile::TempDir;

const GNU_GREP: &str = "/opt/homebrew/opt/grep/libexec/gnubin/grep";

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
    // criterion puts bench binaries deeper; try to find the grep binary
    // in the deps parent directory
    path.push("grep");
    if !path.exists() {
        // fallback: search via cargo metadata
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

/// Generate a directory with N files, each containing `lines_per_file` lines.
/// Some lines contain the word "needle" for searching.
fn generate_corpus(num_files: usize, lines_per_file: usize) -> TempDir {
    let dir = TempDir::new().unwrap();
    for i in 0..num_files {
        let path = dir.path().join(format!("file_{i:04}.txt"));
        let mut f = std::fs::File::create(&path).unwrap();
        for j in 0..lines_per_file {
            if j % 100 == 0 {
                writeln!(f, "line {j}: this line contains the needle we search for").unwrap();
            } else if j % 50 == 0 {
                writeln!(f, "line {j}: error handling is important in production code").unwrap();
            } else {
                writeln!(f, "line {j}: lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor").unwrap();
            }
        }
    }
    dir
}

fn bench_single_file(c: &mut Criterion) {
    let dir = generate_corpus(1, 10_000);
    let file_path = dir.path().join("file_0000.txt");
    let file_str = file_path.to_str().unwrap().to_string();
    let fastgrep = fastgrep_bin();

    let mut group = c.benchmark_group("single_file_10k_lines");

    group.bench_function("gnu_grep", |b| {
        b.iter(|| {
            let out = Command::new(GNU_GREP).args(["needle", &file_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep_no_cache", |b| {
        b.iter(|| {
            let out =
                Command::new(&fastgrep).args(["--no-cache", "needle", &file_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep_with_cache", |b| {
        // warm up the cache
        Command::new(&fastgrep).args(["needle", &file_str]).output().unwrap();
        b.iter(|| {
            let out = Command::new(&fastgrep).args(["needle", &file_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

fn bench_recursive(c: &mut Criterion) {
    let dir = generate_corpus(50, 1_000);
    let dir_str = dir.path().to_str().unwrap().to_string();
    let fastgrep = fastgrep_bin();

    let mut group = c.benchmark_group("recursive_50_files_1k_lines");

    group.bench_function("gnu_grep", |b| {
        b.iter(|| {
            let out = Command::new(GNU_GREP).args(["-r", "needle", &dir_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep_no_cache", |b| {
        b.iter(|| {
            let out = Command::new(&fastgrep)
                .args(["--no-cache", "-r", "needle", &dir_str])
                .output()
                .unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep_with_cache", |b| {
        Command::new(&fastgrep).args(["-r", "needle", &dir_str]).output().unwrap();
        b.iter(|| {
            let out = Command::new(&fastgrep).args(["-r", "needle", &dir_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

fn bench_regex_pattern(c: &mut Criterion) {
    let dir = generate_corpus(1, 10_000);
    let file_path = dir.path().join("file_0000.txt");
    let file_str = file_path.to_str().unwrap().to_string();
    let fastgrep = fastgrep_bin();

    let mut group = c.benchmark_group("regex_pattern_10k_lines");

    let pattern = "needle|error";

    group.bench_function("gnu_grep", |b| {
        b.iter(|| {
            let out = Command::new(GNU_GREP).args(["-E", pattern, &file_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep", |b| {
        b.iter(|| {
            let out = Command::new(&fastgrep)
                .args(["--no-cache", "-E", pattern, &file_str])
                .output()
                .unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

fn bench_count_mode(c: &mut Criterion) {
    let dir = generate_corpus(50, 1_000);
    let dir_str = dir.path().to_str().unwrap().to_string();
    let fastgrep = fastgrep_bin();

    let mut group = c.benchmark_group("count_mode_50_files");

    group.bench_function("gnu_grep", |b| {
        b.iter(|| {
            let out = Command::new(GNU_GREP).args(["-rc", "needle", &dir_str]).output().unwrap();
            assert!(out.status.success());
        });
    });

    group.bench_function("fastgrep", |b| {
        b.iter(|| {
            let out = Command::new(&fastgrep)
                .args(["--no-cache", "-rc", "needle", &dir_str])
                .output()
                .unwrap();
            assert!(out.status.success());
        });
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let fastgrep = fastgrep_bin();
    let mut group = c.benchmark_group("scaling_by_file_count");
    group.sample_size(10);

    for num_files in [10, 50, 100] {
        let dir = generate_corpus(num_files, 1_000);
        let dir_str = dir.path().to_str().unwrap().to_string();

        group.bench_with_input(BenchmarkId::new("gnu_grep", num_files), &dir_str, |b, dir_str| {
            b.iter(|| {
                Command::new(GNU_GREP).args(["-r", "needle", dir_str]).output().unwrap();
            });
        });

        group.bench_with_input(BenchmarkId::new("fastgrep", num_files), &dir_str, |b, dir_str| {
            b.iter(|| {
                Command::new(&fastgrep)
                    .args(["--no-cache", "-r", "needle", dir_str])
                    .output()
                    .unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_file,
    bench_recursive,
    bench_regex_pattern,
    bench_count_mode,
    bench_scaling,
);
criterion_main!(benches);
