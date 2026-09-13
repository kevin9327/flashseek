//! Prefix map: first 1–3 lowercase name chars retrieve candidates.

use std::path::PathBuf;
use std::time::SystemTime;

use flashseek::{Catalog, CatalogEvent, FileRecord};

fn rec(id: u64, name: &str) -> FileRecord {
    FileRecord {
        id,
        parent_id: None,
        name: name.to_string(),
        path: PathBuf::from(format!(r"C:\x\{name}")),
        size: 1,
        modified: SystemTime::UNIX_EPOCH,
        is_dir: false,
        attributes: 0,
    }
}

fn names(catalog: &Catalog, prefix: &str) -> Vec<String> {
    catalog
        .names_with_prefix(prefix)
        .into_iter()
        .map(|r| r.name.clone())
        .collect()
}

#[test]
fn names_with_prefix_returns_alpha_report_not_beta() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "alpha-report"));
    catalog.insert(rec(2, "beta"));

    let hits = names(&catalog, "al");
    assert!(hits.contains(&"alpha-report".to_string()), "{hits:?}");
    assert!(!hits.contains(&"beta".to_string()), "{hits:?}");
}

#[test]
fn names_with_prefix_is_case_insensitive() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "Alpha-report"));
    catalog.insert(rec(2, "BETA"));

    let hits = names(&catalog, "AL");
    assert_eq!(hits, vec!["Alpha-report".to_string()]);
}

#[test]
fn apply_delete_removes_from_prefix_map() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "alpha-report"));
    catalog.apply(CatalogEvent::Delete { id: 1 });
    assert!(catalog.names_with_prefix("al").is_empty());
}

#[test]
fn apply_rename_moves_prefix_buckets() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "alpha-report"));
    catalog.apply(CatalogEvent::Rename {
        id: 1,
        new_name: "beta".into(),
        new_path: PathBuf::from(r"C:\x\beta"),
    });
    assert!(catalog.names_with_prefix("al").is_empty());
    let hits = names(&catalog, "be");
    assert_eq!(hits, vec!["beta".to_string()]);
}
