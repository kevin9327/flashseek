//! Everything-class `zip:` macro SETS the common archive extension list.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom};
use flashseek::{search, Catalog, ContentIndex, FileRecord};

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

fn names(hits: &[flashseek::Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

const ZIP: &[&str] = &["zip", "7z", "rar", "tar", "gz", "tgz", "bz2"];

fn ext_strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn zip_sets_archive_extension_list() {
    for input in ["zip:", "ZIP:", "Zip:"] {
        let q = parse_query(input, now());
        assert_eq!(q.extensions, ext_strings(ZIP), "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn zip_matches_ext_semicolon_list() {
    let archives = parse_query("ext:zip;7z;rar;tar;gz;tgz;bz2", now());
    assert_eq!(parse_query("zip:", now()).extensions, archives.extensions);
}

#[test]
fn zip_is_a_filter_not_a_term() {
    let q = parse_query("backup zip:", now());
    assert_eq!(q.extensions, ext_strings(ZIP));
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "backup"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn last_extension_token_wins() {
    let after_zip = parse_query("zip: ext:pdf", now());
    assert_eq!(after_zip.extensions, vec!["pdf".to_string()]);
    assert!(after_zip.must.is_empty());

    let after_ext = parse_query("ext:pdf zip:", now());
    assert_eq!(after_ext.extensions, ext_strings(ZIP));
    assert!(after_ext.must.is_empty());

    let pic_then_zip = parse_query("pic: zip:", now());
    assert_eq!(pic_then_zip.extensions, ext_strings(ZIP));
}

#[test]
fn zipper_is_not_a_zip_filter() {
    let q = parse_query("zipper:", now());
    assert!(q.extensions.is_empty());
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "zipper:"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn zip_filter_search_hits() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "backup.zip", r"C:\media\backup.zip"));
    catalog.insert(rec(2, "pack.7z", r"C:\media\pack.7z"));
    catalog.insert(rec(3, "old.rar", r"C:\media\old.rar"));
    catalog.insert(rec(4, "src.tar", r"C:\media\src.tar"));
    catalog.insert(rec(5, "log.gz", r"C:\media\log.gz"));
    catalog.insert(rec(6, "bundle.tgz", r"C:\media\bundle.tgz"));
    catalog.insert(rec(7, "data.bz2", r"C:\media\data.bz2"));
    catalog.insert(rec(8, "notes.txt", r"C:\media\notes.txt"));
    catalog.insert(rec(9, "photo.jpg", r"C:\media\photo.jpg"));
    catalog.insert(rec(10, "backup.txt", r"C:\media\backup.txt"));
    let content = ContentIndex::new();

    let zip = names(&search(
        &catalog,
        &content,
        &parse_query("zip:", now()),
        now(),
    ));
    assert!(zip.contains(&"backup.zip".into()), "{zip:?}");
    assert!(zip.contains(&"pack.7z".into()), "{zip:?}");
    assert!(zip.contains(&"old.rar".into()), "{zip:?}");
    assert!(zip.contains(&"src.tar".into()), "{zip:?}");
    assert!(zip.contains(&"log.gz".into()), "{zip:?}");
    assert!(zip.contains(&"bundle.tgz".into()), "{zip:?}");
    assert!(zip.contains(&"data.bz2".into()), "{zip:?}");
    assert!(!zip.contains(&"notes.txt".into()), "{zip:?}");
    assert!(!zip.contains(&"photo.jpg".into()), "{zip:?}");
    assert_eq!(zip.len(), 7, "{zip:?}");

    let mixed = names(&search(
        &catalog,
        &content,
        &parse_query("backup zip:", now()),
        now(),
    ));
    assert!(mixed.contains(&"backup.zip".into()), "{mixed:?}");
    assert!(!mixed.contains(&"pack.7z".into()), "{mixed:?}");
    assert!(!mixed.contains(&"backup.txt".into()), "{mixed:?}");
    assert_eq!(mixed.len(), 1, "{mixed:?}");
}
