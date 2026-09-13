//! Everything-class `noext:` matches files whose `Path::extension()` is None.
//!
//! Directories are excluded even when they have no extension. Tokens must have
//! empty rest (`noext:`), case-insensitive.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom};
use flashseek::{search, Catalog, ContentIndex, FileRecord, Hit};

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
}

fn rec(id: u64, name: &str, path: &str, is_dir: bool) -> FileRecord {
    FileRecord {
        id,
        parent_id: None,
        name: name.into(),
        path: PathBuf::from(path),
        size: 1,
        modified: now(),
        is_dir,
        attributes: 0,
    }
}

fn names(hits: &[Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

fn catalog_with_exts() -> Catalog {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "README", r"C:\docs\README", false));
    catalog.insert(rec(2, "Makefile", r"C:\src\Makefile", false));
    catalog.insert(rec(3, ".gitignore", r"C:\src\.gitignore", false));
    catalog.insert(rec(4, "notes.txt", r"C:\docs\notes.txt", false));
    catalog.insert(rec(5, "archive.tar.gz", r"C:\docs\archive.tar.gz", false));
    catalog.insert(rec(6, "lib.rs", r"C:\src\lib.rs", false));
    catalog.insert(rec(7, "docs", r"C:\docs", true));
    catalog.insert(rec(8, "src", r"C:\src", true));
    catalog
}

#[test]
fn rust_extension_is_none_for_noext_names() {
    assert!(Path::new(r"C:\docs\README").extension().is_none());
    assert!(Path::new(r"C:\src\Makefile").extension().is_none());
    assert!(Path::new(r"C:\src\.gitignore").extension().is_none());
    assert!(Path::new(r"C:\docs").extension().is_none());
    assert!(Path::new(r"C:\docs\notes.txt").extension().is_some());
    assert!(Path::new(r"C:\docs\archive.tar.gz").extension().is_some());
}

#[test]
fn noext_tokens_parse_as_flag() {
    assert!(!parse_query("report", now()).no_ext);

    for input in ["noext:", "NOEXT:", "Noext:"] {
        let q = parse_query(input, now());
        assert!(q.no_ext, "{input}");
        assert!(q.must.is_empty(), "{input}");
        assert!(!q.files_only, "{input}");
        assert!(!q.folders_only, "{input}");
        assert!(q.extensions.is_empty(), "{input}");
        assert!(q.ext_len.is_none(), "{input}");
    }

    let mixed = parse_query("report noext:", now());
    assert!(mixed.no_ext);
    assert_eq!(mixed.must.len(), 1);
    match &mixed.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn non_empty_rest_is_not_noext() {
    for input in ["noext:yes", "noext:1", "noext:README"] {
        let q = parse_query(input, now());
        assert!(!q.no_ext, "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn noexts_is_not_a_noext_filter() {
    let q = parse_query("noexts:", now());
    assert!(!q.no_ext);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "noexts:"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn noext_hits_files_without_extension() {
    let catalog = catalog_with_exts();
    let content = ContentIndex::new();
    for input in ["noext:", "NOEXT:"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        let got = names(&hits);
        assert!(got.contains(&"README".into()), "{input}: {got:?}");
        assert!(got.contains(&"Makefile".into()), "{input}: {got:?}");
        assert!(got.contains(&".gitignore".into()), "{input}: {got:?}");
        assert!(!got.contains(&"notes.txt".into()), "{input}: {got:?}");
        assert!(!got.contains(&"archive.tar.gz".into()), "{input}: {got:?}");
        assert!(!got.contains(&"lib.rs".into()), "{input}: {got:?}");
        assert!(!got.contains(&"docs".into()), "{input}: {got:?}");
        assert!(!got.contains(&"src".into()), "{input}: {got:?}");
        assert_eq!(got.len(), 3, "{input}: {got:?}");
    }
}

#[test]
fn noext_combines_with_name_terms() {
    let catalog = catalog_with_exts();
    let content = ContentIndex::new();
    let hits = search(
        &catalog,
        &content,
        &parse_query("noext: README", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["README".to_string()]);

    let none = search(
        &catalog,
        &content,
        &parse_query("noext: notes", now()),
        now(),
    );
    assert!(names(&none).is_empty(), "{:?}", names(&none));
}

#[test]
fn queries_without_noext_still_match_extensions() {
    let catalog = catalog_with_exts();
    let content = ContentIndex::new();
    let hits = search(&catalog, &content, &parse_query("notes", now()), now());
    assert_eq!(names(&hits), vec!["notes.txt".to_string()]);
}
