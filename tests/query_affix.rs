//! Everything-class `startwith:` / `endwith:` (aliases `start:` / `end:`) match the basename.

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
fn startwith_and_start_tokens_parse_as_prefix_terms() {
    for input in ["startwith:inv", "start:inv", "STARTWITH:inv", "START:inv"] {
        let p = term_pattern(input);
        assert_eq!(p.raw, "inv", "{input}");
        assert!(p.prefix, "{input}");
        assert!(!p.suffix, "{input}");
        assert!(!p.glob && !p.regex && !p.content_only, "{input}");
    }

    let mixed = parse_query("report startwith:inv", now());
    assert_eq!(mixed.must.len(), 2);
    match &mixed.must[0] {
        Atom::Term(p) => {
            assert_eq!(p.raw, "report");
            assert!(!p.prefix && !p.suffix);
        }
        other => panic!("expected Term, got {other:?}"),
    }
    match &mixed.must[1] {
        Atom::Term(p) => {
            assert_eq!(p.raw, "inv");
            assert!(p.prefix);
        }
        other => panic!("expected prefix Term, got {other:?}"),
    }

    let bare = parse_query("startwith: start:", now());
    assert!(bare.must.is_empty());
}

#[test]
fn endwith_and_end_tokens_parse_as_suffix_terms() {
    for input in ["endwith:.bak", "end:.bak", "ENDWITH:.bak", "END:.bak"] {
        let p = term_pattern(input);
        assert_eq!(p.raw, ".bak", "{input}");
        assert!(p.suffix, "{input}");
        assert!(!p.prefix, "{input}");
        assert!(!p.glob && !p.regex && !p.content_only, "{input}");
    }

    for input in ["endwith:bak", "end:bak"] {
        let p = term_pattern(input);
        assert_eq!(p.raw, "bak", "{input}");
        assert!(p.suffix, "{input}");
    }

    let bare = parse_query("endwith: end:", now());
    assert!(bare.must.is_empty());
}

#[test]
fn prefix_and_suffix_match_text_not_substring() {
    let p = Pattern::prefix("inv");
    assert!(p.matches("invoice.txt"));
    assert!(p.matches("INV.txt"));
    assert!(!p.matches("x-invoice.txt"));
    assert!(!p.matches("pre-invoice.txt"));
    assert!(p.matches_with("Invoice.txt", false, false));
    assert!(p.matches_with("invoice.txt", true, false));
    assert!(!p.matches_with("Invoice.txt", true, false));

    let s = Pattern::suffix("bak");
    assert!(s.matches("foo.bak"));
    assert!(s.matches("foo.BAK"));
    assert!(s.matches("bak"));
    assert!(!s.matches("foo.bak.txt"));
    assert!(!s.matches("backup.txt"));

    let sdot = Pattern::suffix(".bak");
    assert!(sdot.matches("foo.bak"));
    assert!(!sdot.matches("foobak"));
}

#[test]
fn startwith_inv_hits_invoice_not_mid_name_path_or_body() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "invoice.txt", r"C:\docs\invoice.txt"));
    catalog.insert(rec(2, "x-invoice.txt", r"C:\docs\x-invoice.txt"));
    catalog.insert(rec(3, "other.txt", r"C:\invoice\other.txt"));
    catalog.insert(rec(4, "body.txt", r"C:\docs\body.txt"));

    let mut content = ContentIndex::new();
    content.index(
        PathBuf::from(r"C:\docs\body.txt"),
        "invoice lives only in the body".into(),
    );

    let hits = search(
        &catalog,
        &content,
        &parse_query("startwith:inv", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["invoice.txt".to_string()]);
    assert!(hits[0].name_match);
    assert!(!hits[0].path_match);
    assert!(!hits[0].content_match);

    let alias = search(&catalog, &content, &parse_query("start:inv", now()), now());
    assert_eq!(names(&alias), vec!["invoice.txt".to_string()]);

    let ci = search(
        &catalog,
        &content,
        &parse_query("STARTWITH:INV", now()),
        now(),
    );
    assert_eq!(names(&ci), vec!["invoice.txt".to_string()]);
}

#[test]
fn endwith_bak_hits_foo_bak_with_or_without_dot() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "foo.bak", r"C:\docs\foo.bak"));
    catalog.insert(rec(2, "foo.bak.txt", r"C:\docs\foo.bak.txt"));
    catalog.insert(rec(3, "backup.txt", r"C:\docs\backup.txt"));
    catalog.insert(rec(4, "notes.txt", r"C:\bak\notes.txt"));
    catalog.insert(rec(5, "body.txt", r"C:\docs\body.txt"));

    let mut content = ContentIndex::new();
    content.index(PathBuf::from(r"C:\docs\body.txt"), "ends with bak".into());

    for input in ["endwith:.bak", "endwith:bak", "end:.bak", "end:bak"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        assert_eq!(names(&hits), vec!["foo.bak".to_string()], "{input}");
        assert!(hits[0].name_match, "{input}");
        assert!(!hits[0].path_match, "{input}");
        assert!(!hits[0].content_match, "{input}");
    }
}

#[test]
fn queries_without_affix_prefix_still_match_substring() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "invoice.txt", r"C:\docs\invoice.txt"));
    catalog.insert(rec(2, "x-invoice.txt", r"C:\docs\x-invoice.txt"));
    catalog.insert(rec(3, "other.txt", r"C:\invoice\other.txt"));
    catalog.insert(rec(4, "foo.bak", r"C:\docs\foo.bak"));
    catalog.insert(rec(5, "mybak.txt", r"C:\docs\mybak.txt"));

    let content = ContentIndex::new();

    let inv = search(&catalog, &content, &parse_query("inv", now()), now());
    let inv_names = names(&inv);
    assert!(
        inv_names.contains(&"invoice.txt".into()),
        "name prefix still hits as substring: {inv_names:?}"
    );
    assert!(
        inv_names.contains(&"x-invoice.txt".into()),
        "mid-name substring: {inv_names:?}"
    );
    assert!(
        inv_names.contains(&"other.txt".into()),
        "path substring: {inv_names:?}"
    );

    let bak = search(&catalog, &content, &parse_query("bak", now()), now());
    let bak_names = names(&bak);
    assert!(bak_names.contains(&"foo.bak".into()), "{bak_names:?}");
    assert!(
        bak_names.contains(&"mybak.txt".into()),
        "mid-name substring default: {bak_names:?}"
    );
}
