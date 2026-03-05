mod common;

use std::io::Write;
use std::process::Command;

use common::*;
use tempfile::NamedTempFile;

// ============================================================
// Binary file handling
// ============================================================

#[test]
fn binary_file_detected() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"hello world\n").unwrap();
    f.write_all(b"some \x00 binary data\n").unwrap();
    f.write_all(b"hello again\n").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();

    let output = Command::new(fastgrep_bin())
        .args(["--no-index", "hello", p])
        .output()
        .expect("failed to run fastgrep");

    assert!(output.stdout.is_empty(), "binary file should not produce stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("binary file matches"),
        "expected binary file message on stderr, got: {stderr}"
    );
    assert!(output.status.success());
}

#[test]
fn binary_file_with_files_with_matches() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"hello world\n").unwrap();
    f.write_all(b"\x00 binary\n").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();

    let output = Command::new(fastgrep_bin())
        .args(["--no-index", "-l", "hello", p])
        .output()
        .expect("failed to run fastgrep");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(p),
        "binary file with -l should print filename to stdout, got: {stdout}"
    );
    assert!(!stdout.contains("Binary"), "-l should not print 'Binary file' message");
    assert!(output.status.success());
}

#[test]
fn binary_file_with_count() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"hello world\n\x00binary\nhello again\n").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();

    let output = Command::new(fastgrep_bin())
        .args(["--no-index", "-c", "hello", p])
        .output()
        .expect("failed to run fastgrep");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(!stdout.is_empty(), "count mode should produce output for binary file");
}

#[test]
fn text_file_not_detected_as_binary() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "hello world").unwrap();
    writeln!(f, "hello again").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();

    let output = Command::new(fastgrep_bin())
        .args(["--no-index", "hello", p])
        .output()
        .expect("failed to run fastgrep");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "text file should not produce stderr, got: {stderr}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 2);
}

#[test]
fn binary_detection_nul_at_end() {
    let mut f = NamedTempFile::new().unwrap();
    for _ in 0..1000 {
        writeln!(f, "hello world this is a normal text line").unwrap();
    }
    f.write_all(b"line with \x00 nul\n").unwrap();
    writeln!(f, "hello after nul").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();

    let output = Command::new(fastgrep_bin())
        .args(["--no-index", "hello", p])
        .output()
        .expect("failed to run fastgrep");

    assert!(output.stdout.is_empty(), "should detect binary even with NUL deep in file");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("binary file matches"),
        "expected binary detection with NUL at end, got stderr: {stderr}"
    );
}

#[test]
fn binary_file_no_match() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"hello\x00world\n").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();

    let output = Command::new(fastgrep_bin())
        .args(["--no-index", "xyz_no_match", p])
        .output()
        .expect("failed to run fastgrep");

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("binary file matches"),
        "no match in binary file should not produce binary message"
    );
    assert!(!output.status.success());
}

// ============================================================
// Known divergences: binary file count
// ============================================================

#[test]
fn binary_file_count_actual_lines() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"hello\nhello\n\x00binary\nhello\n").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-c", "hello", p]);
}

// ============================================================
// Ignore binary files (-I)
// ============================================================

#[test]
fn ignore_binary_suppresses_output() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"hello world\n").unwrap();
    f.write_all(b"some \x00 binary data\n").unwrap();
    f.write_all(b"hello again\n").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();

    let output = Command::new(fastgrep_bin())
        .args(["--no-index", "-I", "hello", p])
        .output()
        .expect("failed to run fastgrep");

    assert!(output.stdout.is_empty(), "-I should suppress binary file stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("binary file matches"),
        "-I should suppress binary file stderr message, got: {stderr}"
    );
    assert!(!output.status.success(), "-I on binary should exit 1");
}

#[test]
fn ignore_binary_with_count() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"hello\n\x00binary\nhello\n").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();

    assert_same_output(&["-Ic", "hello", p]);
}

#[test]
fn ignore_binary_exit_code() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"hello\x00world\n").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();

    let (_, _, gnu_exit, fast_exit) = run_both(&["-I", "hello", p]);
    assert_eq!(gnu_exit, fast_exit, "-I exit codes should match");
}

#[test]
fn invert_files_with_matches_all_match() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "a").unwrap();
    writeln!(f, "a").unwrap();
    writeln!(f, "a").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-l", "-v", "a", p]);
}
