use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use rstest::rstest;
use tempfile::TempDir;

struct Fixture {
    temp: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("data")).unwrap();
        fs::write(temp.path().join("data/a"), b"needle\nneedle\n").unwrap();
        fs::write(temp.path().join("data/b"), b"hay\n").unwrap();
        Self { temp }
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_fastgrep"))
            .args(args)
            .current_dir(self.temp.path())
            .env("HOME", self.temp.path())
            .env("XDG_CACHE_HOME", self.temp.path().join("cache"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if child.try_wait().unwrap().is_some() {
                return child.wait_with_output().unwrap();
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("search hung: {args:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn assert_cached_matches_scan(&self, args: &[&str]) {
        let cached = self.run(args);
        let mut uncached_args = vec!["--no-index"];
        uncached_args.extend_from_slice(args);
        let uncached = self.run(&uncached_args);
        assert_eq!(cached.status.code(), uncached.status.code());
        let sorted = |bytes: &[u8]| {
            let mut lines: Vec<_> =
                String::from_utf8_lossy(bytes).lines().map(str::to_owned).collect();
            lines.sort();
            lines
        };
        assert_eq!(sorted(&cached.stdout), sorted(&uncached.stdout));
        assert_eq!(sorted(&cached.stderr), sorted(&uncached.stderr));
    }
}

#[rstest]
#[case("-rn")]
#[case("-rv")]
#[case("-rc")]
#[case("-rL")]
fn cached_search_preserves_output_modes(#[case] flags: &str) {
    let f = Fixture::new();
    assert!(f.run(&["-r", "needle", "data"]).status.success());
    f.assert_cached_matches_scan(&[flags, "needle", "data"]);
}

#[test]
fn index_does_not_hide_new_files_or_other_roots() {
    let f = Fixture::new();
    f.run(&["-r", "needle", "data"]);
    fs::write(f.temp.path().join("data/new"), b"needle\n").unwrap();
    fs::create_dir(f.temp.path().join("other")).unwrap();
    fs::write(f.temp.path().join("other/match"), b"needle\n").unwrap();
    f.assert_cached_matches_scan(&["-rn", "needle", "data", "other"]);
}

#[test]
fn index_does_not_hide_binary_files_in_text_mode() {
    let f = Fixture::new();
    fs::write(f.temp.path().join("data/binary"), b"\0needle\n").unwrap();
    f.run(&["-r", "needle", "data"]);
    f.assert_cached_matches_scan(&["-ra", "needle", "data"]);
}

#[test]
fn index_does_not_hide_previously_excluded_files() {
    let f = Fixture::new();
    f.run(&["-r", "--include=b", "needle", "data"]);
    f.assert_cached_matches_scan(&["-r", "needle", "data"]);
}

fn index_bytes(dir: &Path) -> Option<Vec<u8>> {
    for entry in fs::read_dir(dir).ok()? {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if let Some(bytes) = index_bytes(&path) {
                return Some(bytes);
            }
        } else if path.file_name().unwrap() == "index.bin" {
            return Some(fs::read(path).unwrap());
        }
    }
    None
}

#[test]
fn stale_index_is_rebuilt() {
    let f = Fixture::new();
    f.run(&["-r", "needle", "data"]);
    let before = index_bytes(f.temp.path()).expect("index must be created");
    fs::write(f.temp.path().join("data/a"), b"changed\n").unwrap();
    f.assert_cached_matches_scan(&["-r", "changed", "data"]);
    let after = index_bytes(f.temp.path()).unwrap();
    assert!(before != after, "stale index must be replaced");
    f.assert_cached_matches_scan(&["-r", "needle", "./data"]);
}

#[test]
fn many_root_directories_do_not_deadlock() {
    let f = Fixture::new();
    let paths: Vec<_> = (0..257).map(|i| format!("root-{i}")).collect();
    for path in &paths {
        fs::create_dir(f.temp.path().join(path)).unwrap();
    }
    let mut args = vec!["--no-index", "-r", "needle"];
    args.extend(paths.iter().map(String::as_str));
    assert_eq!(f.run(&args).status.code(), Some(1));
}

#[rstest]
#[case("data/a")]
#[case("data")]
fn count_limit_is_consistent(#[case] path: &str) {
    let f = Fixture::new();
    let flags = if path == "data" { "-rhcm1" } else { "-hcm1" };
    let output = f.run(&["--no-index", flags, "needle", path]);
    assert!(output.status.success());
    assert!(!output.stdout.contains(&b'2'));
    assert!(output.stdout.contains(&b'1'));
}

#[test]
fn recursive_inverted_file_list_excludes_all_matching_files() {
    let f = Fixture::new();
    let output = f.run(&["--no-index", "-rvl", "needle", "data"]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"data/b\n");
}

#[test]
fn single_file_json_count_is_a_summary() {
    let f = Fixture::new();
    let output = f.run(&["--json", "-cm1", "needle", "data/a"]);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["type"], "summary");
    assert_eq!(value["path"], "data/a");
}

#[test]
fn wide_directory_tree_does_not_deadlock() {
    let f = Fixture::new();
    for branch in 0..4 {
        for leaf in 0..300 {
            fs::create_dir_all(f.temp.path().join(format!("data/{branch}/{leaf}"))).unwrap();
        }
    }
    assert!(f.run(&["--no-index", "-rj1", "needle", "data"]).status.success());
}

#[test]
fn a_small_number_of_changed_files_are_searched() {
    let f = Fixture::new();
    for i in 0..20 {
        fs::write(f.temp.path().join(format!("data/extra-{i}")), b"hay\n").unwrap();
    }
    f.run(&["-r", "needle", "data"]);
    fs::write(f.temp.path().join("data/extra-0"), b"needle\n").unwrap();
    f.assert_cached_matches_scan(&["-rn", "needle", "data"]);
}

#[test]
fn cached_paths_work_with_another_spelling() {
    let f = Fixture::new();
    f.run(&["-r", "needle", "data"]);
    f.assert_cached_matches_scan(&["-rn", "needle", "./data"]);
    let absolute = f.temp.path().join("data");
    f.assert_cached_matches_scan(&["-rn", "needle", absolute.to_str().unwrap()]);
}

#[test]
fn large_file_respects_match_limit() {
    let f = Fixture::new();
    fs::write(f.temp.path().join("data/a"), b"needle\n".repeat(700_000)).unwrap();
    let output = f.run(&["--no-index", "-m1", "needle", "data/a"]);
    assert_eq!(output.stdout, b"needle\n");
}

#[rstest]
#[case("-qv")]
#[case("-vl")]
fn empty_files_have_no_inverted_matches(#[case] flag: &str) {
    let f = Fixture::new();
    fs::write(f.temp.path().join("data/a"), b"").unwrap();
    let output = f.run(&["--no-index", flag, "needle", "data/a"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}
