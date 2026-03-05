mod common;

use common::*;
use rstest::rstest;

// ============================================================
// Basic pattern matching
// ============================================================

#[test]
fn basic_match() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["error", p]);
}

#[test]
fn no_match() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["nonexistent_pattern_xyz", p]);
}

#[rstest]
#[case::dot_matches_any_char("abc\ndef\n", &["-E", "a.c"])]
#[case::dot_star_matches_all("hello\nworld\n", &["-E", ".*"])]
#[case::anchor_caret("hello\n  hello\n", &["-E", "^hello"])]
#[case::anchor_dollar("hello\nhello world\n", &["-E", "hello$"])]
#[case::anchor_caret_dollar_empty("hello\n\nworld\n", &["-E", "^$"])]
#[case::alternation("alpha\nbeta\ngamma\n", &["-E", "alpha|gamma"])]
#[case::character_class("cat\ncut\ncot\nczt\n", &["-E", "c[aou]t"])]
#[case::negated_character_class("cat\ncut\ncot\nczt\n", &["-E", "c[^aou]t"])]
#[case::quantifier_plus("ab\naab\naaab\nb\n", &["-E", "a+b"])]
#[case::quantifier_question("ab\naab\nb\n", &["-E", "a?b"])]
#[case::quantifier_exact("ab\naab\naaab\n", &["-E", "a{2}b"])]
#[case::pattern_at_line_boundaries("abc\nxabc\nabcx\nxabcx\n", &["-E", "^abc$"])]
#[case::grouping("abc\nabc abc\nab\n", &["-E", "(abc)+"])]
#[case::pipe_alternation("cat\ndog\nbird\n", &["-E", "cat|dog"])]
fn regex_patterns(#[case] input: &str, #[case] args: &[&str]) {
    assert_same_stdin(input, args);
}

#[test]
fn extended_regexp_file() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-E", "foo|error", p]);
}
