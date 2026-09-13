//! Everything-class `dir:` alias of `folder:` (directories only).

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
fn dir_is_an_alias_of_folder() {
    let folder = parse_query("folder:", now());
    for input in ["dir:", "DIR:", "Dir:"] {
        let q = parse_query(input, now());
        assert_eq!(q.folders_only, folder.folders_only, "{input}");
        assert!(q.folders_only, "{input}");
        assert!(!q.files_only, "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn dir_attached_rest_matches_folder() {
    let folder = parse_query("folder:work", now());
    for input in ["dir:work", "DIR:work", "Dir:work"] {
        let q = parse_query(input, now());
        assert!(q.folders_only, "{input}");
        assert!(!q.files_only, "{input}");
        assert_eq!(q.must.len(), folder.must.len(), "{input}");
        match (&q.must[0], &folder.must[0]) {
            (Atom::Term(got), Atom::Term(want)) => {
                assert_eq!(got.raw, want.raw, "{input}");
            }
            other => panic!("{input}: expected Term, got {other:?}"),
        }
    }

    let q = parse_query("dir:work", now());
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "work"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn dir_is_a_filter_not_a_term() {
    let q = parse_query("report dir:", now());
    assert!(q.folders_only);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("report dir:work", now());
    assert!(q.folders_only);
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
fn directory_is_not_a_folder_filter() {
    let q = parse_query("directory:", now());
    assert!(!q.folders_only);
    assert!(!q.files_only);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "directory:"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("directory:work", now());
    assert!(!q.folders_only);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "directory:work"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn dir_filters_directories_like_folder() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "work", r"C:\work", true));
    catalog.insert(rec(2, "work.txt", r"C:\docs\work.txt", false));
    catalog.insert(rec(3, "notes", r"C:\notes", true));
    let content = ContentIndex::new();

    for input in ["dir:", "folder:", "DIR:"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        let got = names(&hits);
        assert!(got.contains(&"work".into()), "{input}: {got:?}");
        assert!(got.contains(&"notes".into()), "{input}: {got:?}");
        assert!(!got.contains(&"work.txt".into()), "{input}: {got:?}");
    }

    for input in ["dir:work", "folder:work", "DIR:work"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        assert_eq!(names(&hits), vec!["work".to_string()], "{input}");
    }
}
