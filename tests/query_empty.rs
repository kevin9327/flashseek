//! Everything-class `empty:` is a zero-byte size filter (`size == 0`).
//!
//! Directories are not excluded: a dir whose catalog `size` is 0 also matches.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom, SizeFilter};
use flashseek::{search, Catalog, ContentIndex, FileRecord, Hit};

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
}

fn rec(id: u64, name: &str, path: &str, size: u64, is_dir: bool) -> FileRecord {
    FileRecord {
        id,
        parent_id: None,
        name: name.into(),
        path: PathBuf::from(path),
        size,
        modified: now(),
        is_dir,
        attributes: 0,
    }
}

fn names(hits: &[Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

fn catalog_with_sizes() -> Catalog {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "empty.txt", r"C:\docs\empty.txt", 0, false));
    catalog.insert(rec(2, "tiny.txt", r"C:\docs\tiny.txt", 1, false));
    catalog.insert(rec(3, "report.txt", r"C:\docs\report.txt", 12, false));
    catalog.insert(rec(4, "empty-dir", r"C:\docs\empty-dir", 0, true));
    catalog.insert(rec(5, "full-dir", r"C:\docs\full-dir", 8, true));
    catalog
}

#[test]
fn empty_tokens_parse_as_eq_zero_size_filter() {
    assert_eq!(parse_query("report", now()).size, None);

    for input in ["empty:", "empty:yes", "EMPTY:", "Empty:YES"] {
        let q = parse_query(input, now());
        assert_eq!(q.size, Some(SizeFilter::Eq(0)), "{input}");
        assert!(q.must.is_empty(), "{input}");
        assert!(!q.files_only, "{input}");
        assert!(!q.folders_only, "{input}");
    }

    let mixed = parse_query("report empty:", now());
    assert_eq!(mixed.size, Some(SizeFilter::Eq(0)));
    assert_eq!(mixed.must.len(), 1);
    match &mixed.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let last = parse_query("size:>1mb empty:", now());
    assert_eq!(last.size, Some(SizeFilter::Eq(0)));
    assert!(last.must.is_empty());

    let size_wins = parse_query("empty: size:>1mb", now());
    assert_eq!(size_wins.size, Some(SizeFilter::Gt(1024 * 1024)));

    let invalid = parse_query("empty:no empty:abc", now());
    assert_eq!(invalid.size, None);
    assert!(invalid.must.is_empty());
}

#[test]
fn empty_hits_zero_byte_files() {
    let catalog = catalog_with_sizes();
    let content = ContentIndex::new();
    let hits = search(&catalog, &content, &parse_query("empty:", now()), now());
    let got = names(&hits);
    assert!(got.contains(&"empty.txt".into()), "{got:?}");
    assert!(!got.contains(&"tiny.txt".into()), "{got:?}");
    assert!(!got.contains(&"report.txt".into()), "{got:?}");
    assert!(!got.contains(&"full-dir".into()), "{got:?}");
    // Directories are not excluded; a dir recorded as size 0 matches empty:.
    assert!(got.contains(&"empty-dir".into()), "{got:?}");
    assert_eq!(got.len(), 2, "{got:?}");
}

#[test]
fn empty_yes_matches_size_eq_zero() {
    let catalog = catalog_with_sizes();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("empty:yes", now()),
        now(),
    );
    let got = names(&hits);
    assert!(got.contains(&"empty.txt".into()), "{got:?}");
    assert!(got.contains(&"empty-dir".into()), "{got:?}");
    assert!(!got.contains(&"tiny.txt".into()), "{got:?}");
}

#[test]
fn empty_combines_with_name_terms() {
    let catalog = catalog_with_sizes();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("empty: report", now()),
        now(),
    );
    assert!(names(&hits).is_empty(), "{:?}", names(&hits));

    let hits = search(
        &catalog,
        &content,
        &parse_query("empty: empty", now()),
        now(),
    );
    let got = names(&hits);
    assert!(got.contains(&"empty.txt".into()), "{got:?}");
    assert!(got.contains(&"empty-dir".into()), "{got:?}");
    assert!(!got.contains(&"tiny.txt".into()), "{got:?}");
}

#[test]
fn queries_without_empty_still_match_nonzero() {
    let catalog = catalog_with_sizes();
    let content = ContentIndex::new();
    let hits = search(&catalog, &content, &parse_query("tiny", now()), now());
    assert_eq!(names(&hits), vec!["tiny.txt".to_string()]);
}
