use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use rstest::rstest;

#[rstest]
#[case(&["-n", "needle"], 0)]
#[case(&["--json", "needle"], 0)]
#[case(&["missing"], 1)]
#[case(&["-E", "["], 2)]
fn binary_names_have_identical_behavior(#[case] args: &[&str], #[case] exit_code: i32) {
    let run = |binary| {
        let mut child = Command::new(binary)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // Invalid patterns may exit before reading stdin.
        let _ = child.stdin.take().unwrap().write_all(b"hay\nneedle\n");
        child.wait_with_output().unwrap()
    };
    let grep = run(env!("CARGO_BIN_EXE_grep"));
    let fastgrep = run(env!("CARGO_BIN_EXE_fastgrep"));
    assert_eq!(grep.status.code(), Some(exit_code));
    assert_eq!(fastgrep.status.code(), Some(exit_code));
    assert_eq!(fastgrep.stdout, grep.stdout);
    assert_eq!(fastgrep.stderr, grep.stderr);
    if args == ["-n", "needle"] {
        assert_eq!(fastgrep.stdout, b"2:needle\n");
    }
}
