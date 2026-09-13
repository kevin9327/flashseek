//! Everything-class `root:` path-prefix location filter.

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
fn root_token_sets_root_and_is_not_a_must_term() {
    let q = parse_query(r"root:C:\Users", now());
    assert_eq!(q.root.as_deref(), Some(r"C:\Users"));
    assert!(q.must.is_empty());
    assert!(q.parent.is_none());
    assert!(q.path_contains.is_none());

    let attached = parse_query(r"report root:C:\Users", now());
    assert_eq!(attached.root.as_deref(), Some(r"C:\Users"));
    assert_eq!(attached.must.len(), 1);
    match &attached.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn root_prefix_is_case_insensitive() {
    let p = parse_query(r"ROOT:C:\Users", now());
    assert_eq!(p.root.as_deref(), Some(r"C:\Users"));

    let quoted = parse_query(r#"root:"C:\Program Files""#, now());
    assert_eq!(quoted.root.as_deref(), Some(r"C:\Program Files"));
    assert!(quoted.must.is_empty());

    let bare = parse_query("root:", now());
    assert!(bare.root.is_none());
    assert!(bare.must.is_empty());
}

#[test]
fn root_matches_path_prefix_including_nested() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "plain.txt", r"C:\Users\plain.txt", false));
    catalog.insert(rec(2, "nested.txt", r"C:\Users\swsz9\nested.txt", false));
    catalog.insert(rec(3, "Users", r"C:\Users", true));
    catalog.insert(rec(4, "other.txt", r"C:\Windows\other.txt", false));
    catalog.insert(rec(5, "swsz9", r"C:\Users\swsz9", true));
    let content = ContentIndex::new();

    let hits = search(
        &catalog,
        &content,
        &parse_query(r"root:C:\Users", now()),
        now(),
    );
    let hit_names = names(&hits);
    assert!(hit_names.contains(&"plain.txt".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"nested.txt".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"Users".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"swsz9".into()), "{hit_names:?}");
    assert!(!hit_names.contains(&"other.txt".into()), "{hit_names:?}");
}

#[test]
fn root_is_prefix_not_contains_or_immediate_parent() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "plain.txt", r"C:\Users\plain.txt", false));
    catalog.insert(rec(2, "nested.txt", r"C:\Users\swsz9\nested.txt", false));
    catalog.insert(rec(3, "users.txt", r"C:\Windows\users.txt", false));
    catalog.insert(rec(4, "other.txt", r"C:\Windows\other.txt", false));
    let content = ContentIndex::new();

    let root_hits = search(
        &catalog,
        &content,
        &parse_query(r"root:C:\Users", now()),
        now(),
    );
    let root_names = names(&root_hits);
    assert!(root_names.contains(&"plain.txt".into()), "{root_names:?}");
    assert!(root_names.contains(&"nested.txt".into()), "{root_names:?}");
    assert!(
        !root_names.contains(&"users.txt".into()),
        "path-contains must not satisfy root:: {root_names:?}"
    );
    assert!(!root_names.contains(&"other.txt".into()), "{root_names:?}");

    let parent_hits = search(
        &catalog,
        &content,
        &parse_query(r"parent:C:\Users", now()),
        now(),
    );
    let parent_names = names(&parent_hits);
    assert!(parent_names.contains(&"plain.txt".into()), "{parent_names:?}");
    assert!(
        !parent_names.contains(&"nested.txt".into()),
        "nested must not satisfy parent:: {parent_names:?}"
    );

    let path_hits = search(
        &catalog,
        &content,
        &parse_query("path:Users", now()),
        now(),
    );
    let path_names = names(&path_hits);
    assert!(path_names.contains(&"plain.txt".into()), "{path_names:?}");
    assert!(path_names.contains(&"nested.txt".into()), "{path_names:?}");
    assert!(
        path_names.contains(&"users.txt".into()),
        "path: is contains, not prefix: {path_names:?}"
    );
}

#[test]
fn root_normalizes_slashes_and_case() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "plain.txt", r"C:\Users\plain.txt", false));
    catalog.insert(rec(2, "nested.txt", r"C:\Users\swsz9\nested.txt", false));
    catalog.insert(rec(3, "other.txt", r"C:\Windows\other.txt", false));
    let content = ContentIndex::new();

    for input in [
        r"root:C:\Users",
        "root:C:/Users",
        r"root:c:\USERS",
        r"root:C:\Users\",
        r"ROOT:C:\users",
    ] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        let hit_names = names(&hits);
        assert!(hit_names.contains(&"plain.txt".into()), "{input}: {hit_names:?}");
        assert!(hit_names.contains(&"nested.txt".into()), "{input}: {hit_names:?}");
        assert!(!hit_names.contains(&"other.txt".into()), "{input}: {hit_names:?}");
    }
}

#[test]
fn root_combines_with_other_terms() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "report.txt", r"C:\Users\report.txt", false));
    catalog.insert(rec(2, "notes.txt", r"C:\Users\notes.txt", false));
    catalog.insert(rec(3, "report.txt", r"C:\Windows\report.txt", false));
    catalog.insert(rec(4, "nested.txt", r"C:\Users\swsz9\report.txt", false));
    let content = ContentIndex::new();

    let hits = search(
        &catalog,
        &content,
        &parse_query(r"root:C:\Users report", now()),
        now(),
    );
    let hit_names = names(&hits);
    assert_eq!(hit_names.len(), 2, "{hit_names:?}");
    assert!(hit_names.contains(&"report.txt".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"nested.txt".into()), "{hit_names:?}");
    assert!(!hit_names.contains(&"notes.txt".into()), "{hit_names:?}");
    assert!(
        hits.iter()
            .all(|h| h.record.path.to_string_lossy().to_ascii_lowercase().starts_with(r"c:\users")),
        "{hits:?}"
    );
}
