//! Unprivileged FsChange → CatalogEvent mapper (no watcher thread required).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use flashseek::watch::{fs_change_to_event, FsChange, FsChangeKind};
use flashseek::{search_text, Catalog, ContentIndex};

fn change(
    kind: FsChangeKind,
    path: &str,
    new_path: Option<&str>,
    is_dir: bool,
) -> FsChange {
    FsChange {
        kind,
        path: PathBuf::from(path),
        new_path: new_path.map(PathBuf::from),
        is_dir,
        size: 12,
        modified: SystemTime::UNIX_EPOCH,
    }
}

#[test]
fn create_is_visible_via_by_path_and_query() {
    let mut catalog = Catalog::new();
    let event = fs_change_to_event(
        &change(FsChangeKind::Create, r"C:\docs\notes.txt", None, false),
        1001,
        None,
    )
    .expect("create maps");
    catalog.apply(event);

    let rec = catalog
        .by_path(Path::new(r"C:\docs\notes.txt"))
        .expect("created path");
    assert_eq!(rec.id, 1001);
    assert_eq!(rec.name, "notes.txt");
    assert_eq!(rec.size, 12);
    assert!(!rec.is_dir);

    let hits = search_text(
        &catalog,
        &ContentIndex::new(),
        "notes",
        SystemTime::UNIX_EPOCH,
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].record.id, 1001);
    assert_eq!(hits[0].record.name, "notes.txt");
}

#[test]
fn delete_removes_existing_id_from_catalog() {
    let mut catalog = Catalog::new();
    catalog.apply(
        fs_change_to_event(
            &change(FsChangeKind::Create, r"C:\docs\notes.txt", None, false),
            1001,
            None,
        )
        .unwrap(),
    );

    let event = fs_change_to_event(
        &change(FsChangeKind::Delete, r"C:\docs\notes.txt", None, false),
        1002,
        Some(1001),
    )
    .expect("delete maps with existing_id");
    catalog.apply(event);

    assert!(catalog.by_path(Path::new(r"C:\docs\notes.txt")).is_none());
    assert!(catalog.get(1001).is_none());
    let hits = search_text(
        &catalog,
        &ContentIndex::new(),
        "notes",
        SystemTime::UNIX_EPOCH,
    );
    assert!(hits.is_empty());
}

#[test]
fn delete_without_existing_id_is_none() {
    assert!(fs_change_to_event(
        &change(FsChangeKind::Delete, r"C:\docs\notes.txt", None, false),
        1,
        None,
    )
    .is_none());
}

#[test]
fn rename_updates_path_and_query() {
    let mut catalog = Catalog::new();
    catalog.apply(
        fs_change_to_event(
            &change(FsChangeKind::Create, r"C:\docs\notes.txt", None, false),
            1001,
            None,
        )
        .unwrap(),
    );

    let event = fs_change_to_event(
        &change(
            FsChangeKind::Rename,
            r"C:\docs\notes.txt",
            Some(r"C:\docs\renamed.txt"),
            false,
        ),
        1002,
        Some(1001),
    )
    .expect("rename maps");
    catalog.apply(event);

    assert!(catalog.by_path(Path::new(r"C:\docs\notes.txt")).is_none());
    let rec = catalog
        .by_path(Path::new(r"C:\docs\renamed.txt"))
        .expect("renamed path");
    assert_eq!(rec.id, 1001);
    assert_eq!(rec.name, "renamed.txt");

    let hits = search_text(
        &catalog,
        &ContentIndex::new(),
        "renamed",
        SystemTime::UNIX_EPOCH,
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].record.id, 1001);
}

#[test]
fn rename_without_existing_id_or_new_path_is_none() {
    assert!(fs_change_to_event(
        &change(
            FsChangeKind::Rename,
            r"C:\docs\notes.txt",
            Some(r"C:\docs\renamed.txt"),
            false,
        ),
        1,
        None,
    )
    .is_none());
    assert!(fs_change_to_event(
        &change(FsChangeKind::Rename, r"C:\docs\notes.txt", None, false),
        1,
        Some(1001),
    )
    .is_none());
}

#[test]
fn modify_is_ignored_for_name_catalog() {
    let mut catalog = Catalog::new();
    catalog.apply(
        fs_change_to_event(
            &change(FsChangeKind::Create, r"C:\docs\notes.txt", None, false),
            1001,
            None,
        )
        .unwrap(),
    );

    assert!(fs_change_to_event(
        &change(FsChangeKind::Modify, r"C:\docs\notes.txt", None, false),
        1002,
        Some(1001),
    )
    .is_none());

    let rec = catalog
        .by_path(Path::new(r"C:\docs\notes.txt"))
        .expect("modify must not drop the name");
    assert_eq!(rec.id, 1001);
    assert_eq!(rec.name, "notes.txt");
}

#[test]
fn directory_rename_rewrites_child_paths() {
    let mut catalog = Catalog::new();
    catalog.apply(
        fs_change_to_event(
            &change(FsChangeKind::Create, r"C:\docs", None, true),
            1,
            None,
        )
        .unwrap(),
    );
    catalog.apply(
        fs_change_to_event(
            &change(FsChangeKind::Create, r"C:\docs\a.txt", None, false),
            2,
            None,
        )
        .unwrap(),
    );

    let event = fs_change_to_event(
        &change(
            FsChangeKind::Rename,
            r"C:\docs",
            Some(r"C:\work"),
            true,
        ),
        3,
        Some(1),
    )
    .expect("dir rename maps");
    catalog.apply(event);

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

    assert!(catalog.by_path(Path::new(r"C:\docs")).is_none());
    assert!(catalog.by_path(Path::new(r"C:\docs\a.txt")).is_none());

    let hits = search_text(
        &catalog,
        &ContentIndex::new(),
        "a.txt",
        SystemTime::UNIX_EPOCH,
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].record.path, PathBuf::from(r"C:\work\a.txt"));
}
