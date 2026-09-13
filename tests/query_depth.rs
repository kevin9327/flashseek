//! Everything-class `depth:` filters by path-component count after the drive/root.
//!
//! Depth is `Path::components()` skipping `Prefix` and `RootDir`.
//! `C:\a\b\c.txt` is 3 (`a`, `b`, `c.txt`), not 4 including the drive.

use std::path::{Component, Path, PathBuf};
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

/// Count `Path::components` skipping Prefix and RootDir.
fn depth_of(path: &str) -> u64 {
    Path::new(path)
        .components()
        .filter(|c| !matches!(c, Component::Prefix(_) | Component::RootDir))
        .count() as u64
}

fn catalog_with_depths() -> Catalog {
    let mut catalog = Catalog::new();
    // depth 1
    catalog.insert(rec(1, "root.txt", r"C:\root.txt"));
    // depth 2
    catalog.insert(rec(2, "shallow.txt", r"C:\docs\shallow.txt"));
    // depth 3
    catalog.insert(rec(3, "c.txt", r"C:\a\b\c.txt"));
    catalog.insert(rec(4, "mid.txt", r"C:\docs\notes\mid.txt"));
    // depth 4
    catalog.insert(rec(5, "deep.txt", r"C:\a\b\c\deep.txt"));
    catalog
}

#[test]
fn depth_counts_components_skipping_prefix_and_root() {
    // Drive prefix and root dir do not count; remaining Normal components do.
    assert_eq!(depth_of(r"C:\a\b\c.txt"), 3);
    assert_eq!(depth_of(r"C:\root.txt"), 1);
    assert_eq!(depth_of(r"C:\docs\shallow.txt"), 2);
    assert_eq!(depth_of(r"C:\a\b\c\deep.txt"), 4);
}

#[test]
fn depth_tokens_parse_as_size_filters() {
    assert_eq!(parse_query("report", now()).depth, None);

    for (input, want) in [
        ("depth:>2", SizeFilter::Gt(2)),
        ("depth:>=3", SizeFilter::Ge(3)),
        ("depth:<4", SizeFilter::Lt(4)),
        ("depth:<=2", SizeFilter::Le(2)),
        ("depth:=3", SizeFilter::Eq(3)),
        ("depth:3", SizeFilter::Eq(3)),
        ("DEPTH:>2", SizeFilter::Gt(2)),
        ("Depth:>=3", SizeFilter::Ge(3)),
        ("DEPTH:<4", SizeFilter::Lt(4)),
    ] {
        let q = parse_query(input, now());
        assert_eq!(q.depth, Some(want), "{input}");
        assert!(q.must.is_empty(), "{input}");
        assert!(q.size.is_none(), "{input}");
        assert!(q.name_len.is_none(), "{input}");
    }

    let mixed = parse_query("report depth:>2", now());
    assert_eq!(mixed.depth, Some(SizeFilter::Gt(2)));
    assert_eq!(mixed.must.len(), 1);
    match &mixed.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let last = parse_query("depth:>10 depth:<5", now());
    assert_eq!(last.depth, Some(SizeFilter::Lt(5)));
    assert!(last.must.is_empty());

    let invalid = parse_query("depth:abc depth:", now());
    assert_eq!(invalid.depth, None);
    assert!(invalid.must.is_empty());
}

#[test]
fn depth_eq_3_hits_three_component_paths_only() {
    let catalog = catalog_with_depths();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("depth:3", now()),
        now(),
    );
    let got = names(&hits);
    assert!(got.contains(&"c.txt".into()), "{got:?}");
    assert!(got.contains(&"mid.txt".into()), "{got:?}");
    assert!(!got.contains(&"root.txt".into()), "{got:?}");
    assert!(!got.contains(&"shallow.txt".into()), "{got:?}");
    assert!(!got.contains(&"deep.txt".into()), "{got:?}");
    assert_eq!(got.len(), 2, "{got:?}");
}

#[test]
fn depth_gt_2_hits_deeper_paths_only() {
    let catalog = catalog_with_depths();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("depth:>2", now()),
        now(),
    );
    let got = names(&hits);
    assert!(got.contains(&"c.txt".into()), "{got:?}");
    assert!(got.contains(&"mid.txt".into()), "{got:?}");
    assert!(got.contains(&"deep.txt".into()), "{got:?}");
    assert!(!got.contains(&"root.txt".into()), "{got:?}");
    assert!(!got.contains(&"shallow.txt".into()), "{got:?}");
    assert_eq!(got.len(), 3, "{got:?}");
}

#[test]
fn depth_lt_3_hits_shallow_paths_only() {
    let catalog = catalog_with_depths();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("depth:<3", now()),
        now(),
    );
    let got = names(&hits);
    assert!(got.contains(&"root.txt".into()), "{got:?}");
    assert!(got.contains(&"shallow.txt".into()), "{got:?}");
    assert!(!got.contains(&"c.txt".into()), "{got:?}");
    assert!(!got.contains(&"mid.txt".into()), "{got:?}");
    assert!(!got.contains(&"deep.txt".into()), "{got:?}");
    assert_eq!(got.len(), 2, "{got:?}");
}

#[test]
fn depth_combines_with_name_terms() {
    let catalog = catalog_with_depths();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("depth:3 mid", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["mid.txt".to_string()]);

    let none = search(
        &catalog,
        &content,
        &parse_query("depth:3 deep", now()),
        now(),
    );
    assert!(names(&none).is_empty(), "{:?}", names(&none));
}

#[test]
fn queries_without_depth_still_match() {
    let catalog = catalog_with_depths();
    let content = ContentIndex::new();
    let hits = search(&catalog, &content, &parse_query("txt", now()), now());
    let got = names(&hits);
    assert!(got.contains(&"root.txt".into()), "{got:?}");
    assert!(got.contains(&"shallow.txt".into()), "{got:?}");
    assert!(got.contains(&"c.txt".into()), "{got:?}");
    assert!(got.contains(&"deep.txt".into()), "{got:?}");
    assert_eq!(got.len(), 5, "{got:?}");
}
