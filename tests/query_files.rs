//! Everything-class `files:` alias of `file:` (files only).

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom};
use flashseek::{search, Catalog, ContentIndex, FileRecord};

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
}

fn rec(id: u64, name: &str, path: &str, is_dir: bool) -> FileRecord {
    FileRecord {
        id,
        parent_id: None,
        name: name.into(),
        path: PathBuf::from(path),
        size: 1,
        modified: now(),
        is_dir,
        attributes: 0,
    }
}

fn names(hits: &[flashseek::Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

#[test]
fn files_is_an_alias_of_file() {
    let file = parse_query("file:", now());
    for input in ["files:", "FILES:", "Files:"] {
        let q = parse_query(input, now());
        assert_eq!(q.files_only, file.files_only, "{input}");
        assert!(q.files_only, "{input}");
        assert!(!q.folders_only, "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn files_attached_rest_matches_file() {
    let file = parse_query("file:work", now());
    for input in ["files:work", "FILES:work", "Files:work"] {
        let q = parse_query(input, now());
        assert!(q.files_only, "{input}");
        assert!(!q.folders_only, "{input}");
        assert_eq!(q.must.len(), file.must.len(), "{input}");
        match (&q.must[0], &file.must[0]) {
            (Atom::Term(got), Atom::Term(want)) => {
                assert_eq!(got.raw, want.raw, "{input}");
            }
            other => panic!("{input}: expected Term, got {other:?}"),
        }
    }

    let q = parse_query("files:work", now());
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "work"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn files_is_a_filter_not_a_term() {
    let q = parse_query("report files:", now());
    assert!(q.files_only);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("report files:work", now());
    assert!(q.files_only);
    assert_eq!(q.must.len(), 2);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
    match &q.must[1] {
        Atom::Term(p) => assert_eq!(p.raw, "work"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn filex_is_not_a_file_filter() {
    let q = parse_query("filex:", now());
    assert!(!q.files_only);
    assert!(!q.folders_only);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "filex:"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("filex:work", now());
    assert!(!q.files_only);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "filex:work"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("filename:work", now());
    assert!(!q.files_only);
    assert!(q.name_only);
}

#[test]
fn files_filters_files_like_file() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "work", r"C:\work", true));
    catalog.insert(rec(2, "work.txt", r"C:\docs\work.txt", false));
    catalog.insert(rec(3, "notes.txt", r"C:\notes.txt", false));
    let content = ContentIndex::new();

    for input in ["files:", "file:", "FILES:"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        let got = names(&hits);
        assert!(got.contains(&"work.txt".into()), "{input}: {got:?}");
        assert!(got.contains(&"notes.txt".into()), "{input}: {got:?}");
        assert!(!got.contains(&"work".into()), "{input}: {got:?}");
    }

    for input in ["files:work", "file:work", "FILES:work"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        assert_eq!(names(&hits), vec!["work.txt".to_string()], "{input}");
    }
}
