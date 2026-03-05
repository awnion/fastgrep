mod common;

use common::*;
use rstest::rstest;

#[rstest]
#[case::after("a\nb\nc\nd\ne\n", &["-A1", "c"])]
#[case::before("a\nb\nc\nd\ne\n", &["-B1", "c"])]
#[case::both("a\nb\nc\nd\ne\n", &["-C1", "c"])]
#[case::group_separator("a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n", &["-C1", "-E", "c|h"])]
#[case::overlapping("a\nb\nc\nd\ne\n", &["-C1", "-E", "b|d"])]
#[case::with_line_numbers("a\nb\nc\nd\ne\n", &["-n", "-C1", "c"])]
#[case::at_start("match\na\nb\n", &["-A1", "match"])]
#[case::at_end("a\nb\nmatch\n", &["-B1", "match"])]
#[case::no_match("a\nb\nc\n", &["-C1", "xyz"])]
#[case::with_invert("a\nb\nc\nd\ne\n", &["-v", "-C1", "c"])]
#[case::large("a\nb\nc\nd\ne\nf\ng\n", &["-A3", "c"])]
#[case::a_and_b_separate("a\nb\nc\nd\ne\n", &["-A1", "-B1", "c"])]
#[case::o_ignores_after("hello world\nfoo\nbar\n", &["-o", "-A1", "hello"])]
#[case::o_ignores_before("foo\nbar\nhello world\n", &["-o", "-B1", "hello"])]
fn context(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

#[test]
fn after_context_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-n", "-A1", "error", p]);
}

#[test]
fn before_context_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-n", "-B1", "error", p]);
}

#[test]
fn context_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-n", "-C1", "error", p]);
}

// ============================================================
// Group separator (--group-separator / --no-group-separator)
// ============================================================

#[rstest]
#[case::custom_separator("a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n", &["-C1", "--group-separator=##", "-E", "c|h"])]
#[case::no_group_separator("a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n", &["-C1", "--no-group-separator", "-E", "c|h"])]
#[case::empty_separator("a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n", &["-C1", "--group-separator=", "-E", "c|h"])]
fn group_separator(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

#[test]
fn group_separator_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-C1", "--group-separator=##", "error", p]);
}

#[test]
fn no_group_separator_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-C1", "--no-group-separator", "error", p]);
}
