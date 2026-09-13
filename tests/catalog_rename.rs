//! Directory rename rewrites child paths; recency ranking drives shipped rank_hits / search.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use flashseek::rank::rank_hits;
use flashseek::{search_text, Catalog, CatalogEvent, ContentIndex, FileRecord, Hit};

fn dir(id: u64, name: &str, path: &str) -> FileRecord {
    FileRecord {
        id,
        parent_id: None,
        name: name.into(),
        path: PathBuf::from(path),
        size: 0,
        modified: SystemTime::UNIX_EPOCH,
        is_dir: true,
    }
}

fn file(id: u64, parent: u64, name: &str, path: &str, modified: SystemTime) -> FileRecord {
    FileRecord {
        id,
        parent_id: Some(parent),
        name: name.into(),
        path: PathBuf::from(path),
        size: 1,
        modified,
        is_dir: false,
    }
}

#[test]
fn rename_directory_updates_child_paths() {
    let mut catalog = Catalog::new();
    catalog.apply(CatalogEvent::Create(dir(1, "docs", r"C:\docs")));
    catalog.apply(CatalogEvent::Create(file(
        2,
        1,
        "a.txt",
        r"C:\docs\a.txt",
        SystemTime::UNIX_EPOCH,
    )));

    catalog.apply(CatalogEvent::Rename {
        id: 1,
        new_name: "work".into(),
        new_path: PathBuf::from(r"C:\work"),
    });

    let parent = catalog
        .by_path(Path::new(r"C:\work"))
        .expect("renamed directory");
    assert_eq!(parent.id, 1);
    assert_eq!(parent.name, "work");
    assert!(parent.is_dir);

    let child = catalog
        .by_path(Path::new(r"C:\work\a.txt"))
        .expect("child path rewritten");
    assert_eq!(child.id, 2);
    assert_eq!(child.name, "a.txt");
    assert_eq!(child.parent_id, Some(1));
    assert_eq!(child.path, PathBuf::from(r"C:\work\a.txt"));

    assert!(catalog.by_path(Path::new(r"C:\docs")).is_none());
    assert!(catalog.by_path(Path::new(r"C:\docs\a.txt")).is_none());

    let paths: Vec<_> = catalog.iter().map(|r| r.path.clone()).collect();
    assert!(paths.iter().any(|p| p == Path::new(r"C:\work")));
    assert!(paths.iter().any(|p| p == Path::new(r"C:\work\a.txt")));
}

fn hit(name: &str, modified: SystemTime) -> Hit {
    Hit {
        record: FileRecord {
            id: 1,
            parent_id: None,
            name: name.into(),
            path: PathBuf::from(format!(r"C:\docs\{name}")),
            size: 1,
            modified,
            is_dir: false,
        },
        score: 0.0,
        name_match: true,
        path_match: false,
        content_match: false,
        snippet: None,
    }
}

#[test]
fn recency_aware_rank_hits_prefers_newer_file() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    let older = now - Duration::from_secs(6 * 24 * 3600);
    let newer = now - Duration::from_secs(60);
    let mut hits = vec![hit("report-old.txt", older), hit("report-new.txt", newer)];

    rank_hits(&mut hits, now);

    assert_eq!(hits[0].record.name, "report-new.txt");
    assert!(hits[0].score > hits[1].score);
}

#[test]
fn recency_aware_search_ranks_newer_first() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    let mut catalog = Catalog::new();
    catalog.insert(file(
        10,
        1,
        "report-old.txt",
        r"C:\docs\report-old.txt",
        now - Duration::from_secs(6 * 24 * 3600),
    ));
    catalog.insert(file(
        11,
        1,
        "report-new.txt",
        r"C:\docs\report-new.txt",
        now - Duration::from_secs(60),
    ));

    let hits = search_text(&catalog, &ContentIndex::new(), "report", now);
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].record.name, "report-new.txt");
    assert!(hits[0].score > hits[1].score);
}
