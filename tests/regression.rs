mod common;

use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use common::*;
use tempfile::NamedTempFile;

// ============================================================
// Regression tests
// ============================================================

#[test]
fn multiple_matches_same_line_color() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "test foo test bar test baz test qux test end").unwrap();
    writeln!(f, "no match here").unwrap();
    writeln!(f, "another test line with test in it").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();

    let output = Command::new(fastgrep_bin())
        .args(["--no-cache", "--color=always", "-n", "test", p])
        .output()
        .expect("failed to run fastgrep");

    assert!(output.status.success(), "fastgrep crashed on multiple matches per line");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 2, "expected 2 matching lines, got: {stdout}");
}

#[test]
fn multiple_matches_same_line_color_stdin() {
    let input = "test foo test bar test baz test qux test end\nno match\nanother test line\n";
    let mut child = Command::new(fastgrep_bin())
        .args(["--no-cache", "--color=always", "test"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn fastgrep");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success(), "fastgrep crashed on multiple matches per line via stdin");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 2, "expected 2 matching lines, got: {stdout}");
}

// ============================================================
// Line truncation (fastgrep-specific)
// ============================================================

#[test]
fn truncate_long_lines() {
    let long_line = "x".repeat(500) + "MATCH" + &"y".repeat(600);
    let input = format!("{long_line}\nshort MATCH line\n");

    let mut child = Command::new(fastgrep_bin())
        .args(["--no-cache", "--color=never", "--max-line-len=100", "MATCH"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn fastgrep");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("[truncated"), "expected truncation message, got: {}", lines[0]);
    assert!(lines[0].len() < long_line.len(), "line was not truncated");
    assert!(!lines[1].contains("[truncated"), "short line should not be truncated");
    assert_eq!(lines[1], "short MATCH line");
}

#[test]
fn truncate_disabled_with_zero() {
    let long_line = "x".repeat(2000) + "MATCH";
    let input = format!("{long_line}\n");

    let mut child = Command::new(fastgrep_bin())
        .args(["--no-cache", "--color=never", "--max-line-len=0", "MATCH"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn fastgrep");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().unwrap();
    assert!(!line.contains("[truncated"), "should not truncate with max-line-len=0");
    assert_eq!(line.len(), 2005);
}

#[test]
fn truncate_via_env() {
    let long_line = "x".repeat(200) + "MATCH";
    let input = format!("{long_line}\n");

    let mut child = Command::new(fastgrep_bin())
        .args(["--no-cache", "--color=never", "MATCH"])
        .env("FASTGREP_MAX_LINE_LEN", "50")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn fastgrep");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().unwrap();
    assert!(line.contains("[truncated"), "expected truncation via env var");
}
