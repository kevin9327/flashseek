//! Everything-class `datemodified:` / `modified:` aliases of `dm:`.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom};
use flashseek::{search, Catalog, ContentIndex, FileRecord};

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
}

fn rec(id: u64, name: &str, path: &str, modified: SystemTime) -> FileRecord {
    FileRecord {
        id,
        parent_id: None,
        name: name.into(),
        path: PathBuf::from(path),
        size: 1,
        modified,
        is_dir: false,
        attributes: 0,
    }
}

fn names(hits: &[flashseek::Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

#[test]
fn datemodified_and_modified_are_dm_aliases() {
    let dm = parse_query("dm:lastweek", now());
    for input in [
        "datemodified:lastweek",
        "DATEMODIFIED:lastweek",
        "modified:lastweek",
        "Modified:lastweek",
        "DM:lastweek",
    ] {
        let q = parse_query(input, now());
        assert_eq!(q.modified_after, dm.modified_after, "{input}");
        assert_eq!(q.modified_before, dm.modified_before, "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn date_modified_aliases_are_filters_not_terms() {
    let q = parse_query("report datemodified:lastweek", now());
    assert!(q.modified_after.is_some());
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("report modified:today", now());
    assert!(q.modified_after.is_some());
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn datecreated_is_not_a_modified_filter() {
    let q = parse_query("datecreated:lastweek", now());
    assert!(q.modified_after.is_none());
    assert!(q.modified_before.is_none());
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "datecreated:lastweek"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn datemodified_lastweek_filters_like_dm() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(
        1,
        "recent.txt",
        r"C:\docs\recent.txt",
        now() - Duration::from_secs(2 * 86400),
    ));
    catalog.insert(rec(
        2,
        "old.txt",
        r"C:\docs\old.txt",
        now() - Duration::from_secs(30 * 86400),
    ));
    let content = ContentIndex::new();

    for input in ["dm:lastweek", "datemodified:lastweek", "modified:lastweek"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        assert_eq!(names(&hits), vec!["recent.txt".to_string()], "{input}");
    }
}
