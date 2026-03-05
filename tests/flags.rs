mod common;

use std::io::Write;

use common::*;
use rstest::rstest;
use tempfile::NamedTempFile;

// ============================================================
// Case insensitive (-i)
// ============================================================

#[test]
fn case_insensitive_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-i", "hello", p]);
}

#[rstest]
#[case::basic("Hello World\nhello world\nHELLO\n", &["-i", "hello"])]
#[case::with_fixed("Hello\nhello\nHELLO\nhelp\n", &["-i", "-F", "HELLO"])]
#[case::with_word("HELLO world\nhelloworld\nHello there\n", &["-i", "-w", "hello"])]
fn case_insensitive(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

// ============================================================
// Line numbers (-n)
// ============================================================

#[test]
fn line_numbers_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-n", "error", p]);
}

#[rstest]
#[case::basic("aaa\nbbb\naaa\n", &["-n", "aaa"])]
#[case::start_at_one("match\n", &["-n", "match"])]
#[case::no_trailing_newline("hello\nworld", &["-n", "world"])]
fn line_numbers(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

// ============================================================
// Count (-c)
// ============================================================

#[test]
fn count_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-c", "error", p]);
}

#[test]
fn count_empty_file() {
    let mut f = NamedTempFile::new().unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-c", "pattern", p]);
}

#[rstest]
#[case::no_match("hello\n", &["-c", "xyz"])]
#[case::counts_lines_not_occurrences("aaa bbb aaa\n", &["-c", "aaa"])]
#[case::basic("aaa\nbbb\naaa\nccc\naaa\n", &["-c", "aaa"])]
#[case::with_invert("a\nb\na\n", &["-c", "-v", "a"])]
#[case::ignores_line_numbers("a\nb\na\n", &["-c", "-n", "a"])]
fn count(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

// ============================================================
// Invert match (-v)
// ============================================================

#[test]
fn invert_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-v", "error", p]);
}

#[rstest]
#[case::basic("aaa\nbbb\naaa\n", &["-v", "aaa"])]
#[case::all_lines_match("a\na\na\n", &["-v", "a"])]
#[case::no_lines_match("a\nb\nc\n", &["-v", "xyz"])]
#[case::with_line_numbers("a\nb\nc\n", &["-v", "-n", "b"])]
fn invert_match(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

// ============================================================
// Fixed strings (-F)
// ============================================================

#[test]
fn fixed_string_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-F", "foo bar", p]);
}

#[rstest]
#[case::dot_not_regex("foo.bar\nfooXbar\n", &["-F", "foo.bar"])]
#[case::special_chars("a+b\na++b\nab\n", &["-F", "a+b"])]
#[case::brackets("[test]\ntest\n", &["-F", "[test]"])]
#[case::backslash("a\\b\nab\n", &["-F", "a\\b"])]
fn fixed_string(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

// ============================================================
// Word match (-w)
// ============================================================

#[test]
fn word_regexp_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-w", "error", p]);
}

#[rstest]
#[case::not_embedded("helloworld\nhello world\nhello\n", &["-w", "hello"])]
#[case::with_punctuation("hello, world\nhello.world\nhello_world\n", &["-w", "hello"])]
#[case::at_start_and_end("test\ntest line\nline test\n", &["-w", "test"])]
#[case::with_fixed("a.b c\na.bx\nxa.b\n", &["-w", "-F", "a.b"])]
#[case::stdin("error found\nerror_code\nmy error here\n", &["-w", "error"])]
fn word_match(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

// ============================================================
// Multiple patterns (-e)
// ============================================================

#[test]
fn multiple_patterns_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-e", "hello", "-e", "error", p]);
}

#[rstest]
#[case::basic("alpha\nbeta\ngamma\n", &["-e", "alpha", "-e", "gamma"])]
#[case::with_fixed("a.b\nc.d\naxb\n", &["-F", "-e", "a.b", "-e", "c.d"])]
#[case::overlapping("hello\n", &["-e", "hel", "-e", "ello"])]
fn multiple_patterns(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

// ============================================================
// Combined flags
// ============================================================

#[test]
fn combined_in_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-i", "-n", "hello", p]);
}

#[test]
fn combined_vc_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-v", "-c", "error", p]);
}

#[rstest]
#[case::count_invert("a\nb\nc\n", &["-c", "-v", "a"])]
#[case::insensitive_word_fixed("Hello World\nhello_world\n", &["-i", "-w", "-F", "hello"])]
fn combined_flags(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}
