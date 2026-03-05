mod common;

use common::*;
use rstest::rstest;

#[rstest]
#[case::literal("hello world\ngoodbye\nhello again\n", &["-o", "hello"])]
#[case::regex("foo123bar\nbaz456qux\n", &["-o", "-E", "[0-9]+"])]
#[case::multiple_per_line("abcabc\n", &["-o", "abc"])]
#[case::with_line_numbers("hello world\ngoodbye\nhello again\n", &["-on", "hello"])]
#[case::no_match("hello world\n", &["-o", "xyz"])]
fn only_matching_stdin(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

#[test]
fn only_matching_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-o", "error", p]);
}

#[test]
fn only_matching_file_with_line_numbers() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-on", "error", p]);
}
