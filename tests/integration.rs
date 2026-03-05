use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use tempfile::NamedTempFile;
use tempfile::TempDir;

const GNU_GREP: &str = "/opt/homebrew/opt/grep/libexec/gnubin/grep";

fn fastgrep_bin() -> std::path::PathBuf {
    // cargo test sets this env var pointing to the built binary directory
    let mut path =
        std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().to_path_buf();
    path.push("grep");
    path
}

fn generate_test_file() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "Hello World").unwrap();
    writeln!(f, "hello world").unwrap();
    writeln!(f, "HELLO WORLD").unwrap();
    writeln!(f, "foo bar baz").unwrap();
    writeln!(f, "foo BAR baz").unwrap();
    writeln!(f, "line with error message").unwrap();
    writeln!(f, "another error here").unwrap();
    writeln!(f, "no match on this line").unwrap();
    writeln!(f, "Error at the start").unwrap();
    writeln!(f, "end of file").unwrap();
    f.flush().unwrap();
    f
}

fn generate_test_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    let mut f1 = std::fs::File::create(dir.path().join("file1.txt")).unwrap();
    writeln!(f1, "alpha beta gamma").unwrap();
    writeln!(f1, "delta epsilon zeta").unwrap();
    writeln!(f1, "alpha again").unwrap();

    let mut f2 = std::fs::File::create(dir.path().join("file2.txt")).unwrap();
    writeln!(f2, "one two three").unwrap();
    writeln!(f2, "alpha in file2").unwrap();
    writeln!(f2, "four five six").unwrap();

    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    let mut f3 = std::fs::File::create(sub.join("file3.txt")).unwrap();
    writeln!(f3, "nested alpha line").unwrap();
    writeln!(f3, "nothing here").unwrap();

    dir
}

/// Run GNU grep and fastgrep with the same args, compare stdout.
/// Returns (gnu_stdout, fast_stdout, gnu_exit, fast_exit).
fn run_both(args: &[&str]) -> (String, String, i32, i32) {
    let gnu = Command::new(GNU_GREP).args(args).output().expect("failed to run GNU grep");

    let fast = Command::new(fastgrep_bin())
        .args(["--no-cache"])
        .args(args)
        .output()
        .expect("failed to run fastgrep");

    let gnu_stdout = String::from_utf8_lossy(&gnu.stdout).to_string();
    let fast_stdout = String::from_utf8_lossy(&fast.stdout).to_string();
    let gnu_exit = gnu.status.code().unwrap_or(-1);
    let fast_exit = fast.status.code().unwrap_or(-1);

    (gnu_stdout, fast_stdout, gnu_exit, fast_exit)
}

fn assert_same_output(args: &[&str]) {
    let (gnu, fast, gnu_exit, fast_exit) = run_both(args);
    assert_eq!(
        gnu_exit, fast_exit,
        "exit codes differ for args {args:?}: gnu={gnu_exit}, fast={fast_exit}\ngnu_stdout: {gnu}\nfast_stdout: {fast}"
    );
    assert_eq!(gnu, fast, "stdout differs for args {args:?}");
}

fn assert_same_lines(args: &[&str]) {
    let (gnu, fast, gnu_exit, fast_exit) = run_both(args);
    assert_eq!(
        gnu_exit, fast_exit,
        "exit codes differ for args {args:?}: gnu={gnu_exit}, fast={fast_exit}"
    );
    let mut gnu_lines: Vec<&str> = gnu.lines().collect();
    let mut fast_lines: Vec<&str> = fast.lines().collect();
    gnu_lines.sort();
    fast_lines.sort();
    assert_eq!(gnu_lines, fast_lines, "output lines differ for args {args:?}");
}

fn run_both_stdin(input: &str, args: &[&str]) -> (String, String, i32, i32) {
    let mut gnu = Command::new(GNU_GREP)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn GNU grep");
    gnu.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let gnu_out = gnu.wait_with_output().unwrap();

    let mut fast = Command::new(fastgrep_bin())
        .args(["--no-cache"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn fastgrep");
    fast.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let fast_out = fast.wait_with_output().unwrap();

    (
        String::from_utf8_lossy(&gnu_out.stdout).to_string(),
        String::from_utf8_lossy(&fast_out.stdout).to_string(),
        gnu_out.status.code().unwrap_or(-1),
        fast_out.status.code().unwrap_or(-1),
    )
}

fn assert_same_stdin(input: &str, args: &[&str]) {
    let (gnu, fast, gnu_exit, fast_exit) = run_both_stdin(input, args);
    assert_eq!(
        gnu_exit, fast_exit,
        "exit codes differ for stdin args {args:?}: gnu={gnu_exit}, fast={fast_exit}\ngnu: {gnu}\nfast: {fast}"
    );
    assert_eq!(gnu, fast, "stdout differs for stdin args {args:?}");
}

// ============================================================
// 1. Basic pattern matching
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

#[test]
fn dot_matches_any_char() {
    assert_same_stdin("abc\ndef\n", &["-E", "a.c"]);
}

#[test]
fn dot_star_matches_all_lines() {
    assert_same_stdin("hello\nworld\n", &["-E", ".*"]);
}

#[test]
fn anchor_caret() {
    assert_same_stdin("hello\n  hello\n", &["-E", "^hello"]);
}

#[test]
fn anchor_dollar() {
    assert_same_stdin("hello\nhello world\n", &["-E", "hello$"]);
}

#[test]
fn anchor_caret_dollar_empty_line() {
    assert_same_stdin("hello\n\nworld\n", &["-E", "^$"]);
}

#[test]
fn alternation() {
    assert_same_stdin("alpha\nbeta\ngamma\n", &["-E", "alpha|gamma"]);
}

#[test]
fn character_class() {
    assert_same_stdin("cat\ncut\ncot\nczt\n", &["-E", "c[aou]t"]);
}

#[test]
fn negated_character_class() {
    assert_same_stdin("cat\ncut\ncot\nczt\n", &["-E", "c[^aou]t"]);
}

#[test]
fn quantifier_plus() {
    assert_same_stdin("ab\naab\naaab\nb\n", &["-E", "a+b"]);
}

#[test]
fn quantifier_question() {
    assert_same_stdin("ab\naab\nb\n", &["-E", "a?b"]);
}

#[test]
fn quantifier_exact() {
    assert_same_stdin("ab\naab\naaab\n", &["-E", "a{2}b"]);
}

#[test]
fn pattern_at_line_boundaries() {
    assert_same_stdin("abc\nxabc\nabcx\nxabcx\n", &["-E", "^abc$"]);
}

// ============================================================
// 2. Case insensitive (-i)
// ============================================================

#[test]
fn case_insensitive() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-i", "hello", p]);
}

#[test]
fn case_insensitive_stdin() {
    assert_same_stdin("Hello World\nhello world\nHELLO\n", &["-i", "hello"]);
}

#[test]
fn case_insensitive_with_fixed_string() {
    assert_same_stdin("Hello\nhello\nHELLO\nhelp\n", &["-i", "-F", "HELLO"]);
}

#[test]
fn case_insensitive_with_word() {
    assert_same_stdin("HELLO world\nhelloworld\nHello there\n", &["-i", "-w", "hello"]);
}

// ============================================================
// 3. Line numbers (-n)
// ============================================================

#[test]
fn line_numbers() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-n", "error", p]);
}

#[test]
fn line_numbers_stdin() {
    assert_same_stdin("aaa\nbbb\naaa\n", &["-n", "aaa"]);
}

#[test]
fn line_numbers_start_at_one() {
    assert_same_stdin("match\n", &["-n", "match"]);
}

#[test]
fn line_numbers_no_trailing_newline() {
    assert_same_stdin("hello\nworld", &["-n", "world"]);
}

// ============================================================
// 4. Count (-c)
// ============================================================

#[test]
fn count_mode() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-c", "error", p]);
}

#[test]
fn count_no_match() {
    assert_same_stdin("hello\n", &["-c", "xyz"]);
}

#[test]
fn count_multiple_matches_same_line() {
    // -c counts lines, not occurrences
    assert_same_stdin("aaa bbb aaa\n", &["-c", "aaa"]);
}

#[test]
fn count_empty_file() {
    let mut f = NamedTempFile::new().unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-c", "pattern", p]);
}

#[test]
fn count_stdin() {
    assert_same_stdin("aaa\nbbb\naaa\nccc\naaa\n", &["-c", "aaa"]);
}

#[test]
fn count_with_invert() {
    assert_same_stdin("a\nb\na\n", &["-c", "-v", "a"]);
}

#[test]
fn count_ignores_line_numbers() {
    // -n should be ignored when -c is present
    assert_same_stdin("a\nb\na\n", &["-c", "-n", "a"]);
}

// ============================================================
// 5. Invert match (-v)
// ============================================================

#[test]
fn invert_match() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-v", "error", p]);
}

#[test]
fn invert_match_stdin() {
    assert_same_stdin("aaa\nbbb\naaa\n", &["-v", "aaa"]);
}

#[test]
fn invert_all_lines_match() {
    assert_same_stdin("a\na\na\n", &["-v", "a"]);
}

#[test]
fn invert_no_lines_match() {
    assert_same_stdin("a\nb\nc\n", &["-v", "xyz"]);
}

#[test]
fn invert_with_line_numbers() {
    assert_same_stdin("a\nb\nc\n", &["-v", "-n", "b"]);
}

// ============================================================
// 6. Fixed strings (-F)
// ============================================================

#[test]
fn fixed_string() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-F", "foo bar", p]);
}

#[test]
fn fixed_string_dot_not_regex() {
    assert_same_stdin("foo.bar\nfooXbar\n", &["-F", "foo.bar"]);
}

#[test]
fn fixed_string_special_chars() {
    assert_same_stdin("a+b\na++b\nab\n", &["-F", "a+b"]);
}

#[test]
fn fixed_string_brackets() {
    assert_same_stdin("[test]\ntest\n", &["-F", "[test]"]);
}

#[test]
fn fixed_string_backslash() {
    assert_same_stdin("a\\b\nab\n", &["-F", "a\\b"]);
}

// ============================================================
// 7. Word match (-w)
// ============================================================

#[test]
fn word_regexp() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-w", "error", p]);
}

#[test]
fn word_not_embedded() {
    assert_same_stdin("helloworld\nhello world\nhello\n", &["-w", "hello"]);
}

#[test]
fn word_with_punctuation() {
    assert_same_stdin("hello, world\nhello.world\nhello_world\n", &["-w", "hello"]);
}

#[test]
fn word_at_start_and_end() {
    assert_same_stdin("test\ntest line\nline test\n", &["-w", "test"]);
}

#[test]
fn word_with_fixed_string() {
    assert_same_stdin("a.b c\na.bx\nxa.b\n", &["-w", "-F", "a.b"]);
}

#[test]
fn word_stdin() {
    assert_same_stdin("error found\nerror_code\nmy error here\n", &["-w", "error"]);
}

// ============================================================
// 8. Extended regexp (-E)
// ============================================================

#[test]
fn extended_regexp() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-E", "foo|error", p]);
}

#[test]
fn extended_grouping() {
    assert_same_stdin("abc\nabc abc\nab\n", &["-E", "(abc)+"]);
}

#[test]
fn extended_pipe_alternation() {
    assert_same_stdin("cat\ndog\nbird\n", &["-E", "cat|dog"]);
}

// ============================================================
// 9. Multiple patterns (-e)
// ============================================================

#[test]
fn multiple_patterns_e() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-e", "hello", "-e", "error", p]);
}

#[test]
fn multiple_patterns_stdin() {
    assert_same_stdin("alpha\nbeta\ngamma\n", &["-e", "alpha", "-e", "gamma"]);
}

#[test]
fn multiple_patterns_with_fixed() {
    assert_same_stdin("a.b\nc.d\naxb\n", &["-F", "-e", "a.b", "-e", "c.d"]);
}

#[test]
fn multiple_patterns_overlapping() {
    assert_same_stdin("hello\n", &["-e", "hel", "-e", "ello"]);
}

// ============================================================
// 10. Combined flags
// ============================================================

#[test]
fn combined_flags_in() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-i", "-n", "hello", p]);
}

#[test]
fn combined_flags_inv() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-v", "-c", "error", p]);
}

#[test]
fn combined_count_invert() {
    assert_same_stdin("a\nb\nc\n", &["-c", "-v", "a"]);
}

#[test]
fn combined_insensitive_word_fixed() {
    assert_same_stdin("Hello World\nhello_world\n", &["-i", "-w", "-F", "hello"]);
}

// ============================================================
// 11. Files with matches (-l)
// ============================================================

#[test]
fn files_with_matches_single() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-l", "error", p]);
}

#[test]
fn files_with_matches_no_match() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-l", "zzz_no_match_zzz", p]);
}

#[test]
fn files_with_matches_ignores_line_numbers() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    // -n should be ignored when -l is present
    assert_same_output(&["-l", "-n", "error", p]);
}

// ============================================================
// 12. Multi-file and recursive
// ============================================================

#[test]
fn multi_file_match() {
    let dir = generate_test_dir();
    let f1 = dir.path().join("file1.txt");
    let f2 = dir.path().join("file2.txt");
    let p1 = f1.to_str().unwrap();
    let p2 = f2.to_str().unwrap();
    assert_same_lines(&["alpha", p1, p2]);
}

#[test]
fn multi_file_count() {
    let dir = generate_test_dir();
    let f1 = dir.path().join("file1.txt");
    let f2 = dir.path().join("file2.txt");
    let p1 = f1.to_str().unwrap();
    let p2 = f2.to_str().unwrap();
    assert_same_lines(&["-c", "alpha", p1, p2]);
}

#[test]
fn recursive_search() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    assert_same_lines(&["-r", "alpha", p]);
}

#[test]
fn recursive_files_with_matches() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    assert_same_lines(&["-rl", "alpha", p]);
}

#[test]
fn recursive_count() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    assert_same_lines(&["-rc", "alpha", p]);
}

#[test]
fn recursive_line_numbers() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    assert_same_lines(&["-rn", "alpha", p]);
}

#[test]
fn recursive_no_match() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    let (_, _, gnu_exit, fast_exit) = run_both(&["-r", "zzz_no_match_zzz", p]);
    assert_eq!(gnu_exit, 1);
    assert_eq!(fast_exit, 1);
}

#[test]
fn recursive_case_insensitive() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    assert_same_lines(&["-ri", "ALPHA", p]);
}

#[test]
fn recursive_invert_count() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    assert_same_lines(&["-rvc", "alpha", p]);
}

// ============================================================
// 13. Exit codes
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
// 14. Stdin tests
// ============================================================

#[test]
fn stdin_basic() {
    assert_same_stdin("123Ok123\nhello\nOk again\n", &["Ok"]);
}

#[test]
fn stdin_no_match() {
    assert_same_stdin("hello\nworld\n", &["zzz"]);
}

#[test]
fn stdin_case_insensitive() {
    assert_same_stdin("Hello World\nhello world\nHELLO\n", &["-i", "hello"]);
}

#[test]
fn stdin_line_numbers() {
    assert_same_stdin("aaa\nbbb\naaa\n", &["-n", "aaa"]);
}

#[test]
fn stdin_count() {
    assert_same_stdin("aaa\nbbb\naaa\nccc\naaa\n", &["-c", "aaa"]);
}

#[test]
fn stdin_invert() {
    assert_same_stdin("aaa\nbbb\naaa\n", &["-v", "aaa"]);
}

#[test]
fn stdin_fixed_string() {
    assert_same_stdin("foo.bar\nfooXbar\n", &["-F", "foo.bar"]);
}

#[test]
fn stdin_word_regexp() {
    assert_same_stdin("error found\nerror_code\nmy error here\n", &["-w", "error"]);
}

#[test]
fn stdin_multiple_patterns() {
    assert_same_stdin("alpha\nbeta\ngamma\n", &["-e", "alpha", "-e", "gamma"]);
}

#[test]
fn stdin_empty_input() {
    assert_same_stdin("", &["pattern"]);
}

// ============================================================
// 15. Empty files and edge cases
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

#[test]
fn file_no_trailing_newline() {
    assert_same_stdin("hello", &["hello"]);
}

#[test]
fn file_no_trailing_newline_multi_line() {
    assert_same_stdin("aaa\nbbb", &["bbb"]);
}

#[test]
fn file_only_newlines() {
    assert_same_stdin("\n\n\n", &["-E", "^$"]);
}

#[test]
fn single_empty_line() {
    assert_same_stdin("\n", &["-c", "-E", "^$"]);
}

#[test]
fn line_with_only_spaces() {
    assert_same_stdin("  \nhello\n  \n", &["-E", "^\\s+$"]);
}

// ============================================================
// 16. Binary file handling
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
// 17. Regression tests
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
// 18. Line truncation (fastgrep-specific)
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

// ============================================================
// 19. Known divergences from GNU grep (ignored — fix targets)
// ============================================================

#[test]
fn count_zero_match_files_shown() {
    let dir = generate_test_dir();
    let f1 = dir.path().join("file1.txt");
    let f2 = dir.path().join("file2.txt");
    let p1 = f1.to_str().unwrap();
    let p2 = f2.to_str().unwrap();
    // "delta" only in file1, file2 should show :0
    assert_same_lines(&["-c", "delta", p1, p2]);
}

#[test]
fn binary_file_count_actual_lines() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"hello\nhello\n\x00binary\nhello\n").unwrap();
    f.flush().unwrap();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-c", "hello", p]);
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
