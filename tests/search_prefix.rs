//! Name-only queries use the prefix index as a candidate fast path.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::parse_query;
use flashseek::{search, Catalog, ContentIndex, Engine, FileRecord, Query};

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
    }
}

fn names(hits: &[flashseek::Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

#[test]
fn name_only_query_finds_alpha_via_search() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "alpha.txt", r"C:\notes\alpha.txt"));
    catalog.insert(rec(2, "other.txt", r"C:\alpha\other.txt"));

    let content = ContentIndex::new();
    let query = parse_query("n:alpha", now());
    assert!(query.name_only);
    let hits = search(&catalog, &content, &query, now());
    assert_eq!(names(&hits), vec!["alpha.txt".to_string()]);
}

#[test]
fn name_only_query_finds_alpha_via_engine() {
    let mut engine = Engine::new();
    engine.catalog.insert(rec(1, "alpha.txt", r"C:\notes\alpha.txt"));
    engine.catalog.insert(rec(2, "other.txt", r"C:\alpha\other.txt"));

    let query = Query {
        name_only: true,
        must: parse_query("alpha", now()).must,
        ..Query::default()
    };
    let hits = search(&engine.catalog, &engine.content, &query, now());
    let hit_names = names(&hits);
    assert!(
        hit_names.contains(&"alpha.txt".to_string()),
        "{hit_names:?}"
    );
    assert!(
        !hit_names.contains(&"other.txt".to_string()),
        "{hit_names:?}"
    );
}
