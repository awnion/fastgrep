use std::io::Write;
use std::process::Command;

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

// --- Single file tests ---

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
fn case_insensitive() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-i", "hello", p]);
}

#[test]
fn line_numbers() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-n", "error", p]);
}

#[test]
fn count_mode() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-c", "error", p]);
}

#[test]
fn invert_match() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-v", "error", p]);
}

#[test]
fn fixed_string() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-F", "foo bar", p]);
}

#[test]
fn word_regexp() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-w", "error", p]);
}

#[test]
fn extended_regexp() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-E", "foo|error", p]);
}

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
fn multiple_patterns_e() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-e", "hello", "-e", "error", p]);
}

// --- Multi-file and recursive tests ---

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
    // Recursive output order may differ, compare sorted lines
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

// --- Exit codes ---

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

// --- Files with matches (-l) ---

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
