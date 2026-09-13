//! `count:N` / `max:N` truncate hits after ranking/sort.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom};
use flashseek::{search, Catalog, ContentIndex, FileRecord, Hit};

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
}

fn rec(id: u64, name: &str, path: &str, size: u64, modified: SystemTime) -> FileRecord {
    FileRecord {
        id,
        parent_id: None,
        name: name.into(),
        path: PathBuf::from(path),
        size,
        modified,
        is_dir: false,
        attributes: 0,
    }
}

fn names(hits: &[Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

fn catalog_with_varied_files() -> Catalog {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "alpha.txt", r"C:\docs\alpha.txt", 10, now() - Duration::from_secs(30)));
    catalog.insert(rec(2, "zeta.txt", r"C:\docs\zeta.txt", 300, now() - Duration::from_secs(90)));
    catalog.insert(rec(3, "beta.txt", r"C:\docs\beta.txt", 300, now() - Duration::from_secs(10)));
    catalog.insert(rec(4, "mid.txt", r"C:\docs\mid.txt", 50, now() - Duration::from_secs(60)));
    catalog
}

#[test]
fn count_and_max_tokens_parse_as_filters() {
    assert_eq!(parse_query("report", now()).max_results, None);

    for (input, n) in [
        ("count:20", 20usize),
        ("max:20", 20),
        ("COUNT:20", 20),
        ("MAX:20", 20),
        ("Count:1", 1),
        ("Max:0", 0),
    ] {
        let q = parse_query(input, now());
        assert_eq!(q.max_results, Some(n), "{input}");
        assert!(q.must.is_empty(), "{input}");
    }

    let mixed = parse_query("report count:3", now());
    assert_eq!(mixed.max_results, Some(3));
    assert_eq!(mixed.must.len(), 1);
    match &mixed.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let last = parse_query("count:10 max:5", now());
    assert_eq!(last.max_results, Some(5));
    assert!(last.must.is_empty());

    let last_count = parse_query("max:5 count:10", now());
    assert_eq!(last_count.max_results, Some(10));

    let invalid = parse_query("count:abc max:", now());
    assert_eq!(invalid.max_results, None);
    assert!(invalid.must.is_empty());
}

#[test]
fn count_truncates_after_score_rank() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "other.txt", r"C:\alpha\other.txt", 999, now()));
    catalog.insert(rec(
        2,
        "alpha.txt",
        r"C:\docs\alpha.txt",
        1,
        now() - Duration::from_secs(10_000),
    ));
    let content = ContentIndex::new();

    let all = search(&catalog, &content, &parse_query("alpha", now()), now());
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].record.name, "alpha.txt");
    assert!(all[0].name_match);
    assert!(all[0].score > all[1].score);

    let hits = search(
        &catalog,
        &content,
        &parse_query("alpha count:1", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["alpha.txt".to_string()]);
}

#[test]
fn max_truncates_after_sort() {
    let catalog = catalog_with_varied_files();
    let content = ContentIndex::new();

    let all = search(
        &catalog,
        &content,
        &parse_query("ext:txt sort:size", now()),
        now(),
    );
    assert_eq!(
        names(&all),
        vec![
            "beta.txt".to_string(),
            "zeta.txt".to_string(),
            "mid.txt".to_string(),
            "alpha.txt".to_string(),
        ]
    );

    let hits = search(
        &catalog,
        &content,
        &parse_query("ext:txt sort:size max:2", now()),
        now(),
    );
    assert_eq!(
        names(&hits),
        vec!["beta.txt".to_string(), "zeta.txt".to_string()]
    );
}

#[test]
fn count_zero_returns_no_hits() {
    let catalog = catalog_with_varied_files();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("ext:txt count:0", now()),
        now(),
    );
    assert!(hits.is_empty());
}

#[test]
fn count_larger_than_hits_keeps_all() {
    let catalog = catalog_with_varied_files();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("ext:txt sort:name count:99", now()),
        now(),
    );
    assert_eq!(
        names(&hits),
        vec![
            "alpha.txt".to_string(),
            "beta.txt".to_string(),
            "mid.txt".to_string(),
            "zeta.txt".to_string(),
        ]
    );
}
