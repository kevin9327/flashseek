//! Everything-class `parent:` and `path:` location filters.

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
fn parent_token_sets_parent_and_is_not_a_must_term() {
    let q = parse_query(r"parent:C:\docs", now());
    assert_eq!(q.parent.as_deref(), Some(r"C:\docs"));
    assert!(q.must.is_empty());
    assert!(q.path_contains.is_none());

    let attached = parse_query(r"report parent:C:\docs", now());
    assert_eq!(attached.parent.as_deref(), Some(r"C:\docs"));
    assert_eq!(attached.must.len(), 1);
    match &attached.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn path_token_sets_path_contains_and_is_not_a_must_term() {
    let q = parse_query("path:docs", now());
    assert_eq!(q.path_contains.as_deref(), Some("docs"));
    assert!(q.must.is_empty());
    assert!(q.parent.is_none());

    let mixed = parse_query("path:docs report", now());
    assert_eq!(mixed.path_contains.as_deref(), Some("docs"));
    assert_eq!(mixed.must.len(), 1);
    match &mixed.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn parent_and_path_prefixes_are_case_insensitive() {
    let p = parse_query(r"PARENT:C:\docs", now());
    assert_eq!(p.parent.as_deref(), Some(r"C:\docs"));

    let path = parse_query("PATH:Docs", now());
    assert_eq!(path.path_contains.as_deref(), Some("Docs"));

    let quoted = parse_query(r#"parent:"C:\Program Files""#, now());
    assert_eq!(quoted.parent.as_deref(), Some(r"C:\Program Files"));
    assert!(quoted.must.is_empty());
}

#[test]
fn parent_matches_immediate_parent_only() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "plain.txt", r"C:\docs\plain.txt", false));
    catalog.insert(rec(2, "nested.txt", r"C:\docs\sub\nested.txt", false));
    catalog.insert(rec(3, "docs", r"C:\docs", true));
    catalog.insert(rec(4, "other.txt", r"C:\work\other.txt", false));
    catalog.insert(rec(5, "sub", r"C:\docs\sub", true));
    let content = ContentIndex::new();

    let hits = search(&catalog, &content, &parse_query(r"parent:C:\docs", now()), now());
    let hit_names = names(&hits);
    assert!(hit_names.contains(&"plain.txt".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"sub".into()), "{hit_names:?}");
    assert!(!hit_names.contains(&"nested.txt".into()), "{hit_names:?}");
    assert!(!hit_names.contains(&"docs".into()), "{hit_names:?}");
    assert!(!hit_names.contains(&"other.txt".into()), "{hit_names:?}");
}

#[test]
fn parent_normalizes_slashes_and_case() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "plain.txt", r"C:\docs\plain.txt", false));
    catalog.insert(rec(2, "other.txt", r"C:\work\other.txt", false));
    let content = ContentIndex::new();

    for input in [
        r"parent:C:\docs",
        "parent:C:/docs",
        r"parent:c:\DOCS",
        r"parent:C:\docs\",
        r"PARENT:C:\Docs",
    ] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        let hit_names = names(&hits);
        assert_eq!(hit_names, vec!["plain.txt".to_string()], "{input}: {hit_names:?}");
    }
}

#[test]
fn path_contains_is_a_filter_not_a_must_term() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "plain.txt", r"C:\docs\plain.txt", false));
    catalog.insert(rec(2, "nested.txt", r"C:\docs\sub\nested.txt", false));
    catalog.insert(rec(3, "docs.txt", r"C:\work\docs.txt", false));
    catalog.insert(rec(4, "readme.txt", r"C:\work\readme.txt", false));

    let mut content = ContentIndex::new();
    content.index(
        PathBuf::from(r"C:\work\readme.txt"),
        "the body mentions docs only here".into(),
    );

    let hits = search(&catalog, &content, &parse_query("path:docs", now()), now());
    let hit_names = names(&hits);
    assert!(hit_names.contains(&"plain.txt".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"nested.txt".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"docs.txt".into()), "{hit_names:?}");
    assert!(
        !hit_names.contains(&"readme.txt".into()),
        "body-only must not satisfy path:: {hit_names:?}"
    );
}

#[test]
fn path_contains_normalizes_slashes_and_case() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "nested.txt", r"C:\docs\sub\nested.txt", false));
    catalog.insert(rec(2, "plain.txt", r"C:\docs\plain.txt", false));
    catalog.insert(rec(3, "other.txt", r"C:\work\other.txt", false));
    let content = ContentIndex::new();

    let ci = search(&catalog, &content, &parse_query("path:DOCS", now()), now());
    let ci_names = names(&ci);
    assert!(ci_names.contains(&"nested.txt".into()), "{ci_names:?}");
    assert!(ci_names.contains(&"plain.txt".into()), "{ci_names:?}");
    assert!(!ci_names.contains(&"other.txt".into()), "{ci_names:?}");

    let slash = search(
        &catalog,
        &content,
        &parse_query("path:docs/sub", now()),
        now(),
    );
    assert_eq!(names(&slash), vec!["nested.txt".to_string()]);
}

#[test]
fn parent_and_path_combine_with_other_terms() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "report.txt", r"C:\docs\report.txt", false));
    catalog.insert(rec(2, "notes.txt", r"C:\docs\notes.txt", false));
    catalog.insert(rec(3, "report.txt", r"C:\work\report.txt", false));
    let content = ContentIndex::new();

    let hits = search(
        &catalog,
        &content,
        &parse_query(r"parent:C:\docs report", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["report.txt".to_string()]);
    assert_eq!(hits[0].record.path, PathBuf::from(r"C:\docs\report.txt"));

    let path_hits = search(
        &catalog,
        &content,
        &parse_query("path:work report", now()),
        now(),
    );
    assert_eq!(names(&path_hits), vec!["report.txt".to_string()]);
    assert_eq!(
        path_hits[0].record.path,
        PathBuf::from(r"C:\work\report.txt")
    );
}
