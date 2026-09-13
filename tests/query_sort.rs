//! `sort:size` / `sort:date` / `sort:name` replace score ranking.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom, SortMode};
use flashseek::rank::sort_hits_by;
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
fn sort_tokens_parse_as_filters() {
    assert_eq!(parse_query("report", now()).sort, SortMode::Score);

    for (input, mode) in [
        ("sort:size", SortMode::Size),
        ("sort:date", SortMode::Date),
        ("sort:name", SortMode::Name),
        ("SORT:SIZE", SortMode::Size),
        ("Sort:Date", SortMode::Date),
        ("sort:NAME", SortMode::Name),
    ] {
        let q = parse_query(input, now());
        assert_eq!(q.sort, mode, "{input}");
        assert!(q.must.is_empty(), "{input}");
    }

    let mixed = parse_query("report sort:size", now());
    assert_eq!(mixed.sort, SortMode::Size);
    assert_eq!(mixed.must.len(), 1);
    match &mixed.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let last = parse_query("sort:name sort:date", now());
    assert_eq!(last.sort, SortMode::Date);
    assert!(last.must.is_empty());
}

#[test]
fn sort_size_is_descending_size_then_name() {
    let catalog = catalog_with_varied_files();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("ext:txt sort:size", now()),
        now(),
    );
    assert_eq!(
        names(&hits),
        vec![
            "beta.txt".to_string(),
            "zeta.txt".to_string(),
            "mid.txt".to_string(),
            "alpha.txt".to_string(),
        ]
    );
}

#[test]
fn sort_date_is_descending_modified_then_name() {
    let catalog = catalog_with_varied_files();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("ext:txt sort:date", now()),
        now(),
    );
    assert_eq!(
        names(&hits),
        vec![
            "beta.txt".to_string(),
            "alpha.txt".to_string(),
            "mid.txt".to_string(),
            "zeta.txt".to_string(),
        ]
    );
}

#[test]
fn sort_name_is_ascending_name() {
    let catalog = catalog_with_varied_files();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("ext:txt sort:name", now()),
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

#[test]
fn default_score_sort_still_prefers_name_match() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "other.txt", r"C:\alpha\other.txt", 999, now()));
    catalog.insert(rec(2, "alpha.txt", r"C:\docs\alpha.txt", 1, now() - Duration::from_secs(10_000)));
    let content = ContentIndex::new();

    let hits = search(&catalog, &content, &parse_query("alpha", now()), now());
    assert_eq!(hits[0].record.name, "alpha.txt");
    assert!(hits[0].name_match);
    assert!(hits[0].score > hits[1].score);
}

#[test]
fn sort_hits_by_size_and_date_tie_break_on_name() {
    let t = now();
    let mut hits = vec![
        Hit {
            record: rec(1, "b.bin", r"C:\b.bin", 10, t),
            score: 0.0,
            name_match: true,
            path_match: false,
            content_match: false,
            snippet: None,
        },
        Hit {
            record: rec(2, "a.bin", r"C:\a.bin", 10, t),
            score: 0.0,
            name_match: true,
            path_match: false,
            content_match: false,
            snippet: None,
        },
    ];
    sort_hits_by(&mut hits, SortMode::Size);
    assert_eq!(names(&hits), vec!["a.bin".to_string(), "b.bin".to_string()]);

    sort_hits_by(&mut hits, SortMode::Date);
    assert_eq!(names(&hits), vec!["a.bin".to_string(), "b.bin".to_string()]);
}
