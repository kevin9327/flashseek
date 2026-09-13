use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::parse_query;
use flashseek::types::{CatalogEvent, FileRecord, ATTR_DIRECTORY};
use flashseek::{search, Catalog, ContentIndex};

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
}

fn rec(id: u64, name: &str, is_dir: bool) -> FileRecord {
    FileRecord {
        id,
        parent_id: None,
        name: name.into(),
        path: PathBuf::from(format!("C:\\x\\{name}")),
        size: 1,
        modified: now(),
        is_dir,
        attributes: if is_dir { ATTR_DIRECTORY } else { 0 },
    }
}

#[test]
fn dot_token_sets_flag() {
    let q = parse_query("dot:", now());
    assert!(q.dotfile);
    assert!(q.must.is_empty());
}

#[test]
fn dot_matches_dotfiles_only() {
    let mut cat = Catalog::new();
    cat.apply(CatalogEvent::Create(rec(1, ".env", false)));
    cat.apply(CatalogEvent::Create(rec(2, "env.txt", false)));
    cat.apply(CatalogEvent::Create(rec(3, ".git", true)));
    let q = parse_query("dot:", now());
    let hits = search(&cat, &ContentIndex::new(), &q, now());
    let names: Vec<_> = hits.iter().map(|h| h.record.name.as_str()).collect();
    assert!(names.contains(&".env"));
    assert!(names.contains(&".git"));
    assert!(!names.contains(&"env.txt"));
}
