//! Everything-class `filename:` alias of `name:` / `n:` (basename only).

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom};
use flashseek::{search, Catalog, ContentIndex, FileRecord};

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
}

fn rec(id: u64, name: &str, path: &str) -> FileRecord {
    FileRecord {
        id,
        parent_id: None,
        name: name.into(),
        path: PathBuf::from(path),
        size: 1,
        modified: now(),
        is_dir: false,
        attributes: 0,
    }
}

fn names(hits: &[flashseek::Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

#[test]
fn filename_is_an_alias_of_name() {
    let name = parse_query("name:alpha", now());
    for input in ["filename:alpha", "FILENAME:alpha", "Filename:alpha", "n:alpha"] {
        let q = parse_query(input, now());
        assert_eq!(q.name_only, name.name_only, "{input}");
        assert!(q.name_only, "{input}");
        assert!(!q.files_only, "{input}");
        assert!(!q.folders_only, "{input}");
        assert_eq!(q.must.len(), 1, "{input}");
        match &q.must[0] {
            Atom::Term(p) => assert_eq!(p.raw, "alpha", "{input}"),
            other => panic!("{input}: expected Term, got {other:?}"),
        }
    }
}

#[test]
fn filename_attached_rest_matches_name() {
    let name = parse_query("name:work", now());
    for input in ["filename:work", "FILENAME:work", "Filename:work"] {
        let q = parse_query(input, now());
        assert!(q.name_only, "{input}");
        assert!(!q.files_only, "{input}");
        assert_eq!(q.must.len(), name.must.len(), "{input}");
        match (&q.must[0], &name.must[0]) {
            (Atom::Term(got), Atom::Term(want)) => {
                assert_eq!(got.raw, want.raw, "{input}");
            }
            other => panic!("{input}: expected Term, got {other:?}"),
        }
    }

    let q = parse_query("filename:work", now());
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "work"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn filename_bare_sets_name_only() {
    let name = parse_query("name:", now());
    for input in ["filename:", "FILENAME:", "n:"] {
        let q = parse_query(input, now());
        assert_eq!(q.name_only, name.name_only, "{input}");
        assert!(q.name_only, "{input}");
        assert!(q.must.is_empty(), "{input}");
        assert!(!q.files_only, "{input}");
    }
}

#[test]
fn filename_is_a_name_filter_not_a_term() {
    let q = parse_query("report filename:", now());
    assert!(q.name_only);
    assert!(!q.files_only);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("report filename:alpha", now());
    assert!(q.name_only);
    assert_eq!(q.must.len(), 2);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
    match &q.must[1] {
        Atom::Term(p) => assert_eq!(p.raw, "alpha"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn filenames_is_not_a_name_filter() {
    let q = parse_query("filenames:", now());
    assert!(!q.name_only);
    assert!(!q.files_only);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "filenames:"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("filenames:work", now());
    assert!(!q.name_only);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "filenames:work"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn filename_matches_basename_only_like_name() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "alpha.txt", r"C:\notes\alpha.txt"));
    catalog.insert(rec(2, "other.txt", r"C:\alpha\other.txt"));
    catalog.insert(rec(3, "body.txt", r"C:\docs\body.txt"));

    let mut content = ContentIndex::new();
    content.index(
        PathBuf::from(r"C:\docs\body.txt"),
        "contains alpha in the body".into(),
    );

    for input in ["n:alpha", "name:alpha", "filename:alpha", "FILENAME:alpha"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        let got = names(&hits);
        assert_eq!(got, vec!["alpha.txt".to_string()], "{input}: {got:?}");
        assert!(hits[0].name_match, "{input}");
        assert!(!hits[0].path_match, "{input}");
        assert!(!hits[0].content_match, "{input}");
    }
}
