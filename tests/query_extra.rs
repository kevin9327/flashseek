//! Extra Everything-class operators: quoted phrases, n:/name:, file:, folder:.

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
fn quoted_phrase_stays_one_atom() {
    let q = parse_query("\"foo bar\"", now());
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "foo bar"),
        _ => panic!("quoted phrase must be one Atom::Term, got {:?}", q.must[0]),
    }
    assert!(!q.name_only && !q.files_only && !q.folders_only);
}

#[test]
fn n_and_name_set_name_only_and_strip_prefix() {
    for input in ["n:alpha", "name:alpha", "N:alpha", "NAME:alpha"] {
        let q = parse_query(input, now());
        assert!(q.name_only, "{input}");
        assert_eq!(q.must.len(), 1, "{input}");
        match &q.must[0] {
            Atom::Term(p) => assert_eq!(p.raw, "alpha", "{input}"),
            other => panic!("{input}: expected Term, got {other:?}"),
        }
    }
}

#[test]
fn file_and_folder_tokens_set_kind_flags() {
    let files = parse_query("file:", now());
    assert!(files.files_only);
    assert!(!files.folders_only);
    assert!(files.must.is_empty());

    let folders = parse_query("folder:", now());
    assert!(folders.folders_only);
    assert!(!folders.files_only);
    assert!(folders.must.is_empty());

    let attached = parse_query("file:report folder:", now());
    assert!(attached.files_only);
    assert!(attached.folders_only);
    match &attached.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn existing_operators_still_parse() {
    let q = parse_query("foo bar|baz !tmp *.pdf ext:txt size:>1mb dm:today", now());
    assert_eq!(q.must.len(), 3);
    match &q.must[1] {
        Atom::Or(ps) => {
            assert_eq!(ps[0].raw, "bar");
            assert_eq!(ps[1].raw, "baz");
        }
        other => panic!("expected OR, got {other:?}"),
    }
    assert_eq!(q.must_not.len(), 1);
    assert_eq!(q.extensions, vec!["txt".to_string()]);
    assert!(q.size.is_some());
    assert!(q.modified_after.is_some());
    assert!(!q.name_only && !q.files_only && !q.folders_only);
}

#[test]
fn name_only_ignores_path_and_body() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "alpha.txt", r"C:\notes\alpha.txt", false));
    catalog.insert(rec(2, "other.txt", r"C:\alpha\other.txt", false));
    catalog.insert(rec(3, "body.txt", r"C:\docs\body.txt", false));

    let mut content = ContentIndex::new();
    content.index(PathBuf::from(r"C:\docs\body.txt"), "contains alpha in the body".into());

    let plain = search(&catalog, &content, &parse_query("alpha", now()), now());
    let plain_names = names(&plain);
    assert!(plain_names.contains(&"alpha.txt".into()));
    assert!(plain_names.contains(&"other.txt".into()), "path-only hit: {plain_names:?}");
    assert!(plain_names.contains(&"body.txt".into()), "body-only hit: {plain_names:?}");

    let named = search(&catalog, &content, &parse_query("n:alpha", now()), now());
    let named_names = names(&named);
    assert_eq!(named_names, vec!["alpha.txt".to_string()]);
    assert!(named[0].name_match);
    assert!(!named[0].path_match);
    assert!(!named[0].content_match);

    let named2 = search(&catalog, &content, &parse_query("name:alpha", now()), now());
    assert_eq!(names(&named2), vec!["alpha.txt".to_string()]);
}

#[test]
fn file_and_folder_filter_by_is_dir() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "work", r"C:\work", true));
    catalog.insert(rec(2, "work.txt", r"C:\docs\work.txt", false));

    let content = ContentIndex::new();

    let files = search(&catalog, &content, &parse_query("file:work", now()), now());
    assert_eq!(names(&files), vec!["work.txt".to_string()]);

    let folders = search(&catalog, &content, &parse_query("folder:work", now()), now());
    assert_eq!(names(&folders), vec!["work".to_string()]);
}

#[test]
fn quoted_phrase_matches_contiguous_text() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "a.txt", r"C:\a.txt", false));
    catalog.insert(rec(2, "b.txt", r"C:\b.txt", false));
    catalog.insert(rec(3, "foo bar.txt", r"C:\foo bar.txt", false));

    let mut content = ContentIndex::new();
    content.index(PathBuf::from(r"C:\a.txt"), "the foo bar baz".into());
    content.index(PathBuf::from(r"C:\b.txt"), "foo and then bar".into());

    let hits = search(&catalog, &content, &parse_query("\"foo bar\"", now()), now());
    let hit_names = names(&hits);
    assert!(hit_names.contains(&"a.txt".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"foo bar.txt".into()), "{hit_names:?}");
    assert!(!hit_names.contains(&"b.txt".into()), "{hit_names:?}");
}
