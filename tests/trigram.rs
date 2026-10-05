use std::collections::HashSet;
use std::fs;

use fastgrep::trigram::FileSnapshot;
use fastgrep::trigram::TrigramIndex;
use tempfile::TempDir;

#[test]
fn postings_intersection_matches_direct_trigram_checks() {
    let dir = TempDir::new().unwrap();
    let contents: Vec<_> =
        (0..100).map(|i| format!("prefix group-{} item-{i} suffix\n", i % 7)).collect();
    let paths: Vec<_> = contents
        .iter()
        .enumerate()
        .map(|(i, data)| {
            let path = dir.path().join(i.to_string());
            fs::write(&path, data).unwrap();
            path
        })
        .collect();
    let index = TrigramIndex::build(dir.path(), &paths);
    for query in ["prefix", "group-2", "item-37", "absent", "suffix", "prefix group-2"] {
        let mut trigrams: Vec<_> =
            query.as_bytes().windows(3).map(|t| [t[0], t[1], t[2]]).collect();
        trigrams.push(trigrams[0]);
        let expected: HashSet<_> = paths
            .iter()
            .zip(&contents)
            .filter(|(_, data)| trigrams.iter().all(|t| data.as_bytes().windows(3).any(|w| w == t)))
            .map(|(path, _)| std::path::absolute(path).unwrap())
            .collect();
        assert_eq!(index.candidate_files(&trigrams), expected, "{query}");
    }
}

#[test]
fn changed_files_are_not_indexed_from_old_buffers() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("file");
    fs::write(&path, b"old contents").unwrap();
    let snapshot = FileSnapshot::capture(&path).unwrap();
    fs::write(&path, b"new contents with a different size").unwrap();
    assert!(snapshot.extract(b"old contents").is_none());
}

#[test]
fn unknown_and_stale_paths_are_never_excluded() {
    let dir = TempDir::new().unwrap();
    let paths: Vec<_> = (0..20)
        .map(|i| {
            let path = dir.path().join(i.to_string());
            fs::write(&path, b"hay\n").unwrap();
            path
        })
        .collect();
    let index = TrigramIndex::build(dir.path(), &paths);
    fs::write(&paths[0], b"needle\n").unwrap();
    let plan = index.plan(&[*b"nee"], true);
    assert!(!plan.needs_rebuild());
    assert!(!plan.can_skip(&paths[0]));
    assert!(plan.can_skip(&paths[1]));
    assert!(!plan.can_skip(&dir.path().join("new")));
    assert!(!index.plan(&[*b"nee"], false).can_skip(&paths[1]));
}
