mod common;

use common::*;

// ============================================================
// Files with matches (-l)
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
// Multi-file and recursive
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
// Files without match (-L)
// ============================================================

#[test]
fn files_without_match_has_match() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-L", "error", p]);
}

#[test]
fn files_without_match_no_match() {
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["-L", "zzz_no_match_zzz", p]);
}

#[test]
fn files_without_match_multi_file() {
    let dir = generate_test_dir();
    let f1 = dir.path().join("file1.txt");
    let f2 = dir.path().join("file2.txt");
    let p1 = f1.to_str().unwrap();
    let p2 = f2.to_str().unwrap();
    // "delta" only in file1, so -L should print file2
    assert_same_lines(&["-L", "delta", p1, p2]);
}

#[test]
fn files_without_match_recursive() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    // "delta" only in file1.txt, not in file2.txt or sub/file3.txt
    assert_same_lines(&["-rL", "delta", p]);
}

// ============================================================
// Exclude dir (--exclude-dir)
// ============================================================

#[test]
fn exclude_dir_basic() {
    let dir = generate_test_dir();
    let p = dir.path().to_str().unwrap();
    // Without --exclude-dir=sub, "nested alpha line" in sub/file3.txt would match
    assert_same_lines(&["-r", "--exclude-dir=sub", "alpha", p]);
}

#[test]
fn exclude_dir_multiple() {
    let dir = generate_test_dir();
    // Create another subdir
    let sub2 = dir.path().join("other");
    std::fs::create_dir(&sub2).unwrap();
    let mut f = std::fs::File::create(sub2.join("file4.txt")).unwrap();
    std::io::Write::write_all(&mut f, b"alpha in other\n").unwrap();

    let p = dir.path().to_str().unwrap();
    assert_same_lines(&["-r", "--exclude-dir=sub", "--exclude-dir=other", "alpha", p]);
}

#[test]
fn exclude_dir_no_effect_without_recursive() {
    // --exclude-dir is only meaningful with -r
    let f = generate_test_file();
    let p = f.path().to_str().unwrap();
    assert_same_output(&["--exclude-dir=sub", "error", p]);
}

// ============================================================
// Known divergences: count zero-match files
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
