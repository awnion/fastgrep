mod common;

use common::*;
use rstest::rstest;

#[rstest]
#[case::basic("123Ok123\nhello\nOk again\n", &["Ok"])]
#[case::no_match("hello\nworld\n", &["zzz"])]
#[case::case_insensitive("Hello World\nhello world\nHELLO\n", &["-i", "hello"])]
#[case::line_numbers("aaa\nbbb\naaa\n", &["-n", "aaa"])]
#[case::count("aaa\nbbb\naaa\nccc\naaa\n", &["-c", "aaa"])]
#[case::invert("aaa\nbbb\naaa\n", &["-v", "aaa"])]
#[case::fixed_string("foo.bar\nfooXbar\n", &["-F", "foo.bar"])]
#[case::word_regexp("error found\nerror_code\nmy error here\n", &["-w", "error"])]
#[case::multiple_patterns("alpha\nbeta\ngamma\n", &["-e", "alpha", "-e", "gamma"])]
#[case::empty_input("", &["pattern"])]
fn stdin(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}
