//! Everything-class `type:` alias of `ext:`.

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

#[test]
fn type_is_an_alias_of_ext() {
    let ext = parse_query("ext:pdf", now());
    for input in ["type:pdf", "TYPE:pdf", "Type:PDF", "TYPE:PDF"] {
        let q = parse_query(input, now());
        assert_eq!(q.extensions, ext.extensions, "{input}");
        assert_eq!(q.extensions, vec!["pdf".to_string()], "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn type_semicolon_list_matches_ext() {
    let ext = parse_query("ext:pdf;txt", now());
    for input in ["type:pdf;txt", "TYPE:pdf;txt", "Type:PDF;TXT"] {
        let q = parse_query(input, now());
        assert_eq!(q.extensions, ext.extensions, "{input}");
        assert_eq!(
            q.extensions,
            vec!["pdf".to_string(), "txt".to_string()],
            "{input}"
        );
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn type_is_a_filter_not_a_term() {
    let q = parse_query("report type:pdf", now());
    assert_eq!(q.extensions, vec!["pdf".to_string()]);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("report type:pdf;txt", now());
    assert_eq!(q.extensions, vec!["pdf".to_string(), "txt".to_string()]);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn filetype_is_not_an_ext_filter() {
    let q = parse_query("filetype:pdf", now());
    assert!(q.extensions.is_empty());
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "filetype:pdf"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn type_pdf_filters_like_ext() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "invoice.pdf", r"C:\docs\invoice.pdf"));
    catalog.insert(rec(2, "notes.txt", r"C:\docs\notes.txt"));
    catalog.insert(rec(3, "photo.jpg", r"C:\docs\photo.jpg"));
    let content = ContentIndex::new();

    for input in ["ext:pdf", "type:pdf"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        assert_eq!(names(&hits), vec!["invoice.pdf".to_string()], "{input}");
    }

    for input in ["ext:pdf;txt", "type:pdf;txt"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        let got = names(&hits);
        assert!(got.contains(&"invoice.pdf".into()), "{input}: {got:?}");
        assert!(got.contains(&"notes.txt".into()), "{input}: {got:?}");
        assert!(!got.contains(&"photo.jpg".into()), "{input}: {got:?}");
        assert_eq!(got.len(), 2, "{input}: {got:?}");
    }
}
