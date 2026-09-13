//! Everything-class `len:` filters by basename character length (unicode scalars).

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

fn catalog_with_varied_names() -> Catalog {
    let mut catalog = Catalog::new();
    // 2 unicode scalars
    catalog.insert(rec(1, "ab", r"C:\very\long\path\that\should\not\count\ab"));
    // 4
    catalog.insert(rec(2, "a.rs", r"C:\docs\a.rs"));
    // 5
    catalog.insert(rec(3, "ab.rs", r"C:\docs\ab.rs"));
    // 9
    catalog.insert(rec(4, "hello.txt", r"C:\docs\hello.txt"));
    // 15
    catalog.insert(rec(5, "hello-world.txt", r"C:\docs\hello-world.txt"));
    // 6 unicode scalars (한 글 . t x t)
    catalog.insert(rec(6, "한글.txt", r"C:\docs\한글.txt"));
    catalog
}

#[test]
fn len_tokens_parse_as_size_filters() {
    assert_eq!(parse_query("report", now()).name_len, None);

    for (input, want) in [
        ("len:>10", SizeFilter::Gt(10)),
        ("len:>=3", SizeFilter::Ge(3)),
        ("len:<20", SizeFilter::Lt(20)),
        ("len:<=5", SizeFilter::Le(5)),
        ("len:=4", SizeFilter::Eq(4)),
        ("len:4", SizeFilter::Eq(4)),
        ("LEN:>10", SizeFilter::Gt(10)),
        ("Len:>=3", SizeFilter::Ge(3)),
        ("LEN:<5", SizeFilter::Lt(5)),
    ] {
        let q = parse_query(input, now());
        assert_eq!(q.name_len, Some(want), "{input}");
        assert!(q.must.is_empty(), "{input}");
        assert!(q.size.is_none(), "{input}");
    }

    let mixed = parse_query("report len:>10", now());
    assert_eq!(mixed.name_len, Some(SizeFilter::Gt(10)));
    assert_eq!(mixed.must.len(), 1);
    match &mixed.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let last = parse_query("len:>10 len:<5", now());
    assert_eq!(last.name_len, Some(SizeFilter::Lt(5)));
    assert!(last.must.is_empty());

    let invalid = parse_query("len:abc len:", now());
    assert_eq!(invalid.name_len, None);
    assert!(invalid.must.is_empty());
}

#[test]
fn len_gt_10_hits_long_basenames_only() {
    let catalog = catalog_with_varied_names();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("len:>10", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["hello-world.txt".to_string()]);
}

#[test]
fn len_lt_5_hits_short_basenames_only() {
    let catalog = catalog_with_varied_names();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("len:<5", now()),
        now(),
    );
    let got = names(&hits);
    assert!(got.contains(&"ab".into()), "{got:?}");
    assert!(got.contains(&"a.rs".into()), "{got:?}");
    assert!(!got.contains(&"ab.rs".into()), "{got:?}");
    assert!(!got.contains(&"hello.txt".into()), "{got:?}");
    assert!(!got.contains(&"hello-world.txt".into()), "{got:?}");
    assert_eq!(got.len(), 2, "{got:?}");
}

#[test]
fn len_ge_counts_unicode_scalars_not_bytes() {
    let catalog = catalog_with_varied_names();
    let content = ContentIndex::new();
    // "한글.txt" is 6 unicode scalars, more than 6 UTF-8 bytes.
    assert_eq!("한글.txt".chars().count(), 6);
    assert!("한글.txt".len() > 6);

    let hits = search(
        &catalog,
        &content,
        &parse_query("len:=6", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["한글.txt".to_string()]);

    let too_long = search(
        &catalog,
        &content,
        &parse_query("len:>6", now()),
        now(),
    );
    let too_long_names = names(&too_long);
    assert!(!too_long_names.contains(&"한글.txt".into()), "{too_long_names:?}");
    assert!(too_long_names.contains(&"hello.txt".into()), "{too_long_names:?}");
}

#[test]
fn len_ignores_path_length() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(
        1,
        "x.rs",
        r"C:\very\long\directory\name\that\exceeds\ten\chars\x.rs",
    ));
    catalog.insert(rec(2, "hello-world.txt", r"C:\a\hello-world.txt"));
    let content = ContentIndex::new();

    let short = search(&catalog, &content, &parse_query("len:<5", now()), now());
    assert_eq!(names(&short), vec!["x.rs".to_string()]);

    let long = search(&catalog, &content, &parse_query("len:>10", now()), now());
    assert_eq!(names(&long), vec!["hello-world.txt".to_string()]);
}

#[test]
fn queries_without_len_still_match() {
    let catalog = catalog_with_varied_names();
    let content = ContentIndex::new();
    let hits = search(&catalog, &content, &parse_query("hello", now()), now());
    let got = names(&hits);
    assert!(got.contains(&"hello.txt".into()), "{got:?}");
    assert!(got.contains(&"hello-world.txt".into()), "{got:?}");
}
