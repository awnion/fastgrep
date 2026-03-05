mod common;

use std::io::Write;

use common::*;
use rstest::rstest;
use tempfile::NamedTempFile;

// ============================================================
// Exit codes
// ============================================================

#[test]
fn exit_code_match() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    let (_, _, gnu_exit, fast_exit) = run_both(&["error", p]);
    assert_eq!(gnu_exit, 0);
    assert_eq!(fast_exit, 0);
}

#[test]
fn exit_code_no_match() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    let (_, _, gnu_exit, fast_exit) = run_both(&["zzz_no_match_zzz", p]);
    assert_eq!(gnu_exit, 1);
    assert_eq!(fast_exit, 1);
}

#[test]
fn exit_code_no_match_stdin() {
    let (_, _, gnu_exit, fast_exit) = run_both_stdin("hello\n", &["xyz"]);
    assert_eq!(gnu_exit, 1);
    assert_eq!(fast_exit, 1);
}

// ============================================================
// Empty files and edge cases
// ============================================================

#[test]
fn empty_file() {
    let mut f = NamedTempFile::new().unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();
    let (_, _, gnu_exit, fast_exit) = run_both(&["pattern", p]);
    assert_eq!(gnu_exit, 1);
    assert_eq!(fast_exit, 1);
}

#[rstest]
#[case::no_trailing_newline("hello", &["hello"])]
#[case::no_trailing_newline_multi("aaa\nbbb", &["bbb"])]
#[case::only_newlines("\n\n\n", &["-E", "^$"])]
#[case::single_empty_line("\n", &["-c", "-E", "^$"])]
#[case::only_spaces("  \nhello\n  \n", &["-E", "^\\s+$"])]
fn edge_cases(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}
