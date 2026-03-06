mod common;

use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use common::*;
use serde_json::Value;
use tempfile::NamedTempFile;
use tempfile::TempDir;

fn parse_json_lines(bytes: &[u8]) -> Vec<Value> {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("expected valid JSON line"))
        .collect()
}

#[test]
fn json_outputs_match_objects_for_file_search() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();

    let output = Command::new(fastgrep_bin())
        .args(["--no-index", "--json", "-n", "error", p])
        .output()
        .expect("failed to run fastgrep");

    assert!(output.status.success(), "expected match success");
    let lines = parse_json_lines(&output.stdout);
    assert_eq!(lines.len(), 2);

    assert_eq!(lines[0]["type"], "match");
    assert_eq!(lines[0]["path"], p);
    assert_eq!(lines[0]["line_number"], 6);
    assert_eq!(lines[0]["lines"]["text"], "line with error message");
    assert_eq!(lines[0]["submatches"][0]["match"]["text"], "error");
    assert_eq!(lines[0]["submatches"][0]["start"], 10);
    assert_eq!(lines[0]["submatches"][0]["end"], 15);

    assert_eq!(lines[1]["type"], "match");
    assert_eq!(lines[1]["line_number"], 7);
    assert_eq!(lines[1]["lines"]["text"], "another error here");
}

#[test]
fn json_outputs_match_objects_for_stdin_with_label() {
    let input = "hello world\nno match\nhello again\n";
    let mut child = Command::new(fastgrep_bin())
        .args(["--no-index", "--json", "-Hn", "--label", "stdin.txt", "hello"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn fastgrep");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success(), "expected match success");
    let lines = parse_json_lines(&output.stdout);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["path"], "stdin.txt");
    assert_eq!(lines[0]["line_number"], 1);
    assert_eq!(lines[1]["path"], "stdin.txt");
    assert_eq!(lines[1]["line_number"], 3);
}

#[test]
fn json_emits_structured_size_limit_warning_on_stderr() {
    let dir = TempDir::new().unwrap();

    let mut small = std::fs::File::create(dir.path().join("small.txt")).unwrap();
    writeln!(small, "needle in small file").unwrap();

    let mut big = NamedTempFile::new_in(dir.path()).unwrap();
    write!(big, "{}", "x".repeat(64)).unwrap();
    big.flush().unwrap();

    let output = Command::new(fastgrep_bin())
        .args([
            "--no-index",
            "--json",
            "--max-file-size=32",
            "-r",
            "needle",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run fastgrep");

    assert!(output.status.success(), "small file should still match");
    let stdout = parse_json_lines(&output.stdout);
    assert_eq!(stdout.len(), 1);

    let stderr = parse_json_lines(&output.stderr);
    assert_eq!(stderr.len(), 1);
    assert_eq!(stderr[0]["type"], "warning");
    assert_eq!(stderr[0]["kind"], "size_limit");
    assert_eq!(stderr[0]["path"], big.path().to_str().unwrap());
    assert_eq!(stderr[0]["size_bytes"], 64);
    assert_eq!(stderr[0]["max_file_size"], 32);
}
