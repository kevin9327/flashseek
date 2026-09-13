//! Everything-class `extlen:` filters by extension character length.
//!
//! Length is the unicode scalars after the last `.` in the basename, not including the dot.
//! `foo.jpeg` is 4; names with no `.` are 0.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom, SizeFilter};
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

fn catalog_with_exts() -> Catalog {
    let mut catalog = Catalog::new();
    // extlen 0 — no dot
    catalog.insert(rec(1, "README", r"C:\docs\README"));
    // extlen 2
    catalog.insert(rec(2, "lib.rs", r"C:\src\lib.rs"));
    // extlen 3
    catalog.insert(rec(3, "notes.txt", r"C:\docs\notes.txt"));
    catalog.insert(rec(4, "archive.tar.gz", r"C:\docs\archive.tar.gz"));
    // extlen 4 (`jpeg`, `pptx`)
    catalog.insert(rec(5, "foo.jpeg", r"C:\pics\foo.jpeg"));
    catalog.insert(rec(6, "photo.jpeg", r"C:\pics\photo.jpeg"));
    catalog.insert(rec(7, "slide.pptx", r"C:\docs\slide.pptx"));
    // extlen 5
    catalog.insert(rec(8, "app.swift", r"C:\src\app.swift"));
    catalog
}

#[test]
fn jpeg_basename_has_extlen_4() {
    assert_eq!("foo.jpeg".rsplit_once('.').unwrap().1.chars().count(), 4);
}

#[test]
fn extlen_tokens_parse_as_size_filters() {
    assert_eq!(parse_query("report", now()).ext_len, None);

    for (input, want) in [
        ("extlen:>3", SizeFilter::Gt(3)),
        ("extlen:>=4", SizeFilter::Ge(4)),
        ("extlen:<4", SizeFilter::Lt(4)),
        ("extlen:<=3", SizeFilter::Le(3)),
        ("extlen:=4", SizeFilter::Eq(4)),
        ("extlen:4", SizeFilter::Eq(4)),
        ("EXTLEN:>3", SizeFilter::Gt(3)),
        ("Extlen:>=4", SizeFilter::Ge(4)),
        ("EXTLEN:<4", SizeFilter::Lt(4)),
    ] {
        let q = parse_query(input, now());
        assert_eq!(q.ext_len, Some(want), "{input}");
        assert!(q.must.is_empty(), "{input}");
        assert!(q.size.is_none(), "{input}");
        assert!(q.name_len.is_none(), "{input}");
        assert!(q.depth.is_none(), "{input}");
    }

    let mixed = parse_query("report extlen:>3", now());
    assert_eq!(mixed.ext_len, Some(SizeFilter::Gt(3)));
    assert_eq!(mixed.must.len(), 1);
    match &mixed.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let last = parse_query("extlen:>10 extlen:<5", now());
    assert_eq!(last.ext_len, Some(SizeFilter::Lt(5)));
    assert!(last.must.is_empty());

    let invalid = parse_query("extlen:abc extlen:", now());
    assert_eq!(invalid.ext_len, None);
    assert!(invalid.must.is_empty());
}

#[test]
fn extlen_gt_3_hits_jpeg_not_txt() {
    let catalog = catalog_with_exts();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("extlen:>3", now()),
        now(),
    );
    let got = names(&hits);
    assert!(got.contains(&"foo.jpeg".into()), "{got:?}");
    assert!(got.contains(&"photo.jpeg".into()), "{got:?}");
    assert!(got.contains(&"slide.pptx".into()), "{got:?}");
    assert!(got.contains(&"app.swift".into()), "{got:?}");
    assert!(!got.contains(&"notes.txt".into()), "{got:?}");
    assert!(!got.contains(&"lib.rs".into()), "{got:?}");
    assert!(!got.contains(&"README".into()), "{got:?}");
    assert!(!got.contains(&"archive.tar.gz".into()), "{got:?}");
    assert_eq!(got.len(), 4, "{got:?}");
}

#[test]
fn extlen_eq_4_hits_jpeg_only() {
    let catalog = catalog_with_exts();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("extlen:4", now()),
        now(),
    );
    let got = names(&hits);
    assert!(got.contains(&"foo.jpeg".into()), "{got:?}");
    assert!(got.contains(&"photo.jpeg".into()), "{got:?}");
    assert!(got.contains(&"slide.pptx".into()), "{got:?}");
    assert!(!got.contains(&"app.swift".into()), "{got:?}");
    assert!(!got.contains(&"notes.txt".into()), "{got:?}");
    assert_eq!(got.len(), 3, "{got:?}");
}

#[test]
fn extlen_uses_chars_after_last_dot() {
    let catalog = catalog_with_exts();
    let content = ContentIndex::new();
    // archive.tar.gz → "gz" (2), not "tar.gz"
    let gz = search(
        &catalog,
        &content,
        &parse_query("extlen:2", now()),
        now(),
    );
    let got = names(&gz);
    assert!(got.contains(&"lib.rs".into()), "{got:?}");
    assert!(got.contains(&"archive.tar.gz".into()), "{got:?}");
    assert_eq!(got.len(), 2, "{got:?}");

    let none = search(
        &catalog,
        &content,
        &parse_query("extlen:0", now()),
        now(),
    );
    assert_eq!(names(&none), vec!["README".to_string()]);
}

#[test]
fn extlen_counts_unicode_scalars_not_bytes() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "pic.한글", r"C:\docs\pic.한글"));
    catalog.insert(rec(2, "pic.jpeg", r"C:\docs\pic.jpeg"));
    let content = ContentIndex::new();
    assert_eq!("한글".chars().count(), 2);
    assert!("한글".len() > 2);

    let hits = search(
        &catalog,
        &content,
        &parse_query("extlen:=2", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["pic.한글".to_string()]);
}

#[test]
fn extlen_combines_with_name_terms() {
    let catalog = catalog_with_exts();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("extlen:>3 foo", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["foo.jpeg".to_string()]);

    let none = search(
        &catalog,
        &content,
        &parse_query("extlen:>3 notes", now()),
        now(),
    );
    assert!(names(&none).is_empty(), "{:?}", names(&none));
}

#[test]
fn queries_without_extlen_still_match() {
    let catalog = catalog_with_exts();
    let content = ContentIndex::new();
    let hits = search(&catalog, &content, &parse_query("foo", now()), now());
    assert_eq!(names(&hits), vec!["foo.jpeg".to_string()]);
}
