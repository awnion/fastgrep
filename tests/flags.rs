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
// Quiet (-q)
// ============================================================

#[test]
fn quiet_match_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-q", "error", p]);
}

#[test]
fn quiet_no_match_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-q", "zzz_no_match_zzz", p]);
}

#[rstest]
#[case::match_found("hello world\n", &["-q", "hello"])]
#[case::no_match("hello world\n", &["-q", "xyz"])]
#[case::case_insensitive("Hello World\n", &["-qi", "hello"])]
#[case::invert("aaa\nbbb\n", &["-qv", "aaa"])]
fn quiet(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

#[test]
fn quiet_recursive() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    let (gnu, fast, gnu_exit, fast_exit) = run_both(&["-rq", "alpha", p]);
    assert_eq!(gnu_exit, fast_exit);
    assert!(gnu.is_empty());
    assert!(fast.is_empty());
}

// ============================================================
// No filename (-h) / With filename (-H)
// ============================================================

#[test]
fn no_filename_multi_file() {
    let dir = generate_test_dir();
    let f1 = dir.path().join("file1.txt");
    let f2 = dir.path().join("file2.txt");
    let p1 = f1.to_str().unwrap();
    let p2 = f2.to_str().unwrap();
    assert_same_lines(&["-h", "alpha", p1, p2]);
}

#[test]
fn with_filename_single_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-H", "error", p]);
}

#[test]
fn no_filename_recursive() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    assert_same_lines(&["-rh", "alpha", p]);
}

#[test]
fn no_filename_with_line_numbers() {
    let dir = generate_test_dir();
    let f1 = dir.path().join("file1.txt");
    let f2 = dir.path().join("file2.txt");
    let p1 = f1.to_str().unwrap();
    let p2 = f2.to_str().unwrap();
    assert_same_lines(&["-hn", "alpha", p1, p2]);
}

#[test]
fn with_filename_line_numbers() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-Hn", "error", p]);
}

#[test]
fn no_filename_count() {
    let dir = generate_test_dir();
    let f1 = dir.path().join("file1.txt");
    let f2 = dir.path().join("file2.txt");
    let p1 = f1.to_str().unwrap();
    let p2 = f2.to_str().unwrap();
    assert_same_lines(&["-hc", "alpha", p1, p2]);
}

// ============================================================
// Max count (-m)
// ============================================================

#[test]
fn max_count_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-m1", "error", p]);
}

#[rstest]
#[case::one("a\na\na\n", &["-m1", "a"])]
#[case::two("a\na\na\n", &["-m2", "a"])]
#[case::more_than_matches("a\nb\n", &["-m5", "a"])]
#[case::with_line_numbers("a\nb\na\nb\na\n", &["-n", "-m2", "a"])]
#[case::with_count("a\na\na\na\n", &["-c", "-m2", "a"])]
#[case::with_invert("a\nb\na\nb\n", &["-v", "-m1", "a"])]
fn max_count(#[case] input: &str, #[case] args: &[&str]) {
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

// ============================================================
// Colour alias (--colour)
// ============================================================

#[test]
fn colour_alias_same_as_color() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    let (_, color_out, _, _) = run_both(&["--color=always", "error", p]);
    let (_, colour_out, _, _) = run_both(&["--colour=always", "error", p]);
    assert_eq!(color_out, colour_out, "--colour should produce same output as --color");
}

#[test]
fn colour_alias_never() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    let (_, color_out, _, _) = run_both(&["--color=never", "error", p]);
    let (_, colour_out, _, _) = run_both(&["--colour=never", "error", p]);
    assert_eq!(color_out, colour_out);
}

// ============================================================
// No messages (-s)
// ============================================================

#[test]
fn no_messages_nonexistent_file() {
    let output = std::process::Command::new(fastgrep_bin())
        .args(["--no-cache", "-s", "pattern", "/nonexistent/path/file.txt"])
        .output()
        .expect("failed to run fastgrep");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "-s should suppress error messages, got: {stderr}");
}

#[test]
fn no_messages_exit_code() {
    // -s should not change exit codes
    let (_, _, gnu_exit, fast_exit) = run_both(&["-s", "pattern", "/nonexistent/path/file.txt"]);
    assert_eq!(gnu_exit, fast_exit, "exit codes should match with -s");
}

#[test]
fn without_no_messages_shows_error() {
    let output = std::process::Command::new(fastgrep_bin())
        .args(["--no-cache", "pattern", "/nonexistent/path/file.txt"])
        .output()
        .expect("failed to run fastgrep");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty(), "without -s, errors should appear on stderr");
}

// ============================================================
// Byte offset (-b)
// ============================================================

#[rstest]
#[case::basic("aaa\nbbb\naaa\n", &["-b", "aaa"])]
#[case::with_line_numbers("aaa\nbbb\naaa\n", &["-bn", "aaa"])]
#[case::with_count("aaa\nbbb\naaa\n", &["-bc", "aaa"])]
#[case::single_line("hello\n", &["-b", "hello"])]
#[case::no_trailing_newline("aaa\nbbb", &["-b", "bbb"])]
fn byte_offset(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

#[test]
fn byte_offset_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-b", "error", p]);
}

#[test]
fn byte_offset_with_line_numbers_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-bn", "error", p]);
}
