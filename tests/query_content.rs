//! Everything-class `content:` / `body:` terms match file bodies only.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom};
use flashseek::{search, Catalog, ContentIndex, FileRecord, Hit};

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

fn names(hits: &[Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

#[test]
fn content_and_body_tokens_parse_as_content_only_terms() {
    for input in ["content:zzzz", "body:zzzz", "CONTENT:zzzz", "BODY:zzzz"] {
        let q = parse_query(input, now());
        assert_eq!(q.must.len(), 1, "{input}");
        match &q.must[0] {
            Atom::Term(p) => {
                assert_eq!(p.raw, "zzzz", "{input}");
                assert!(p.content_only, "{input}");
            }
            other => panic!("{input}: expected Term, got {other:?}"),
        }
        assert!(q.must[0].content_only(), "{input}");
        assert!(!q.name_only, "{input}");
    }

    let mixed = parse_query("report content:zzzz", now());
    assert_eq!(mixed.must.len(), 2);
    match &mixed.must[0] {
        Atom::Term(p) => {
            assert_eq!(p.raw, "report");
            assert!(!p.content_only);
        }
        other => panic!("expected Term, got {other:?}"),
    }
    match &mixed.must[1] {
        Atom::Term(p) => {
            assert_eq!(p.raw, "zzzz");
            assert!(p.content_only);
        }
        other => panic!("expected content Term, got {other:?}"),
    }

    let bare = parse_query("content: body:", now());
    assert!(bare.must.is_empty());
}

#[test]
fn content_zzzz_hits_body_content_alpha_does_not_hit_name() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "alpha.txt", r"C:\docs\alpha.txt"));

    let mut content = ContentIndex::new();
    content.index(PathBuf::from(r"C:\docs\alpha.txt"), "zzzz".into());

    let body_hits = search(
        &catalog,
        &content,
        &parse_query("content:zzzz", now()),
        now(),
    );
    assert_eq!(names(&body_hits), vec!["alpha.txt".to_string()]);
    assert!(body_hits[0].content_match);
    assert!(!body_hits[0].name_match);
    assert!(!body_hits[0].path_match);

    let name_hits = search(
        &catalog,
        &content,
        &parse_query("content:alpha", now()),
        now(),
    );
    assert!(
        name_hits.is_empty(),
        "content:alpha must not match the filename: {:?}",
        names(&name_hits)
    );

    let body_alias = search(&catalog, &content, &parse_query("body:zzzz", now()), now());
    assert_eq!(names(&body_alias), vec!["alpha.txt".to_string()]);
}

#[test]
fn queries_without_content_prefix_still_match_name_or_path_or_body() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "alpha.txt", r"C:\docs\alpha.txt"));
    catalog.insert(rec(2, "other.txt", r"C:\alpha\other.txt"));
    catalog.insert(rec(3, "body.txt", r"C:\docs\body.txt"));

    let mut content = ContentIndex::new();
    content.index(PathBuf::from(r"C:\docs\alpha.txt"), "zzzz".into());
    content.index(PathBuf::from(r"C:\docs\body.txt"), "zzzz lives here".into());

    let by_name = search(&catalog, &content, &parse_query("alpha", now()), now());
    let by_name_names = names(&by_name);
    assert!(
        by_name_names.contains(&"alpha.txt".into()),
        "name hit: {by_name_names:?}"
    );
    assert!(
        by_name_names.contains(&"other.txt".into()),
        "path hit: {by_name_names:?}"
    );

    let by_body = search(&catalog, &content, &parse_query("zzzz", now()), now());
    let by_body_names = names(&by_body);
    assert!(
        by_body_names.contains(&"alpha.txt".into()),
        "body hit via name file: {by_body_names:?}"
    );
    assert!(
        by_body_names.contains(&"body.txt".into()),
        "body-only hit: {by_body_names:?}"
    );
    assert!(
        !by_body_names.contains(&"other.txt".into()),
        "{by_body_names:?}"
    );
}
