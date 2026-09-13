//! Everything-class `stem:` matches the basename without extension (file stem).

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom, Pattern};
use flashseek::{search, Catalog, ContentIndex, FileRecord, Hit};

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

fn names(hits: &[Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

fn term_pattern(input: &str) -> Pattern {
    let q = parse_query(input, now());
    assert_eq!(q.must.len(), 1, "{input}");
    match &q.must[0] {
        Atom::Term(p) => p.clone(),
        other => panic!("{input}: expected Term, got {other:?}"),
    }
}

#[test]
fn stem_tokens_parse_as_stem_only_terms() {
    for input in ["stem:invoice", "STEM:invoice", "Stem:invoice"] {
        let p = term_pattern(input);
        assert_eq!(p.raw, "invoice", "{input}");
        assert!(p.stem_only, "{input}");
        assert!(!p.prefix && !p.suffix, "{input}");
        assert!(!p.glob && !p.regex && !p.content_only, "{input}");
    }

    let mixed = parse_query("report stem:invoice", now());
    assert_eq!(mixed.must.len(), 2);
    match &mixed.must[0] {
        Atom::Term(p) => {
            assert_eq!(p.raw, "report");
            assert!(!p.stem_only);
        }
        other => panic!("expected Term, got {other:?}"),
    }
    match &mixed.must[1] {
        Atom::Term(p) => {
            assert_eq!(p.raw, "invoice");
            assert!(p.stem_only);
        }
        other => panic!("expected stem Term, got {other:?}"),
    }

    let bare = parse_query("stem:", now());
    assert!(bare.must.is_empty());
    assert!(!bare.name_only);
}

#[test]
fn stems_is_not_a_stem_operator() {
    let q = parse_query("stems:invoice", now());
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => {
            assert_eq!(p.raw, "stems:invoice");
            assert!(!p.stem_only);
        }
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn stem_matches_substring_on_file_stem_not_extension() {
    let p = Pattern::stem("invoice");
    assert!(p.matches("invoice.txt"));
    assert!(p.matches("INVOICE.TXT"));
    assert!(p.matches("invoice-backup.txt"));
    assert!(p.matches("myinvoice.doc"));
    assert!(p.matches("invoice"));
    assert!(!p.matches("x.invoice"));
    assert!(!p.matches("notes.txt"));
    assert!(p.matches_with("Invoice.pdf", false, false));
    assert!(p.matches_with("invoice.txt", true, false));
    assert!(!p.matches_with("Invoice.txt", true, false));
}

#[test]
fn stem_invoice_hits_stem_substring_not_extension_path_or_body() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "invoice.txt", r"C:\docs\invoice.txt"));
    catalog.insert(rec(2, "invoice-backup.txt", r"C:\docs\invoice-backup.txt"));
    catalog.insert(rec(3, "x.invoice", r"C:\docs\x.invoice"));
    catalog.insert(rec(4, "other.txt", r"C:\invoice\other.txt"));
    catalog.insert(rec(5, "body.txt", r"C:\docs\body.txt"));

    let mut content = ContentIndex::new();
    content.index(
        PathBuf::from(r"C:\docs\body.txt"),
        "invoice lives only in the body".into(),
    );

    for input in ["stem:invoice", "STEM:invoice", "Stem:INVOICE"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        let got = names(&hits);
        assert_eq!(
            got,
            vec![
                "invoice.txt".to_string(),
                "invoice-backup.txt".to_string()
            ],
            "{input}: {got:?}"
        );
        for h in &hits {
            assert!(h.name_match, "{input}");
            assert!(!h.path_match, "{input}");
            assert!(!h.content_match, "{input}");
        }
    }
}
