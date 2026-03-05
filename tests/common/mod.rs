use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use tempfile::NamedTempFile;
use tempfile::TempDir;

pub const GNU_GREP: &str = "/opt/homebrew/opt/grep/libexec/gnubin/grep";

pub fn fastgrep_bin() -> std::path::PathBuf {
    // cargo test sets this env var pointing to the built binary directory
    let mut path =
        std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().to_path_buf();
    path.push("grep");
    path
}

pub fn generate_test_file() -> NamedTempFile {
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

pub fn generate_test_dir() -> TempDir {
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
pub fn run_both(args: &[&str]) -> (String, String, i32, i32) {
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

pub fn assert_same_output(args: &[&str]) {
    let (gnu, fast, gnu_exit, fast_exit) = run_both(args);
    assert_eq!(
        gnu_exit, fast_exit,
        "exit codes differ for args {args:?}: gnu={gnu_exit}, fast={fast_exit}\ngnu_stdout: {gnu}\nfast_stdout: {fast}"
    );
    assert_eq!(gnu, fast, "stdout differs for args {args:?}");
}

pub fn assert_same_lines(args: &[&str]) {
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

pub fn run_both_stdin(input: &str, args: &[&str]) -> (String, String, i32, i32) {
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

pub fn assert_same_stdin(input: &str, args: &[&str]) {
    let (gnu, fast, gnu_exit, fast_exit) = run_both_stdin(input, args);
    assert_eq!(
        gnu_exit, fast_exit,
        "exit codes differ for stdin args {args:?}: gnu={gnu_exit}, fast={fast_exit}\ngnu: {gnu}\nfast: {fast}"
    );
    assert_eq!(gnu, fast, "stdout differs for stdin args {args:?}");
}
