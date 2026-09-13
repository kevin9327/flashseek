//! Everything-class `system:` alias of `attrib:S`.
//!
//! Tokens must have empty rest (`system:`), case-insensitive.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom};
use flashseek::{
    search, Catalog, ContentIndex, FileRecord, ATTR_DIRECTORY, ATTR_HIDDEN, ATTR_READONLY,
    ATTR_SYSTEM,
};

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
}

fn rec(id: u64, name: &str, path: &str, is_dir: bool, attributes: u32) -> FileRecord {
    FileRecord {
        id,
        parent_id: None,
        name: name.into(),
        path: PathBuf::from(path),
        size: 1,
        modified: now(),
        is_dir,
        attributes,
    }
}

fn names(hits: &[flashseek::Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

#[test]
fn system_is_attrib_s_alias() {
    let s = parse_query("attrib:S", now());
    for input in ["system:", "SYSTEM:", "System:"] {
        let q = parse_query(input, now());
        assert_eq!(q.attrib_mask, s.attrib_mask, "{input}");
        assert_eq!(q.attrib_mask, ATTR_SYSTEM, "{input}");
        assert_eq!(q.attrib_any, 0, "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn aliases_combine_and_are_filters_not_terms() {
    let both = parse_query("system: hidden:", now());
    assert_eq!(both.attrib_mask, ATTR_SYSTEM | ATTR_HIDDEN);
    assert_eq!(both.attrib_any, 0);
    assert!(both.must.is_empty());

    let mixed = parse_query("report system:", now());
    assert_eq!(mixed.attrib_mask, ATTR_SYSTEM);
    assert_eq!(mixed.must.len(), 1);
    match &mixed.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let with_attrib = parse_query("system: attrib:H", now());
    assert_eq!(with_attrib.attrib_mask, ATTR_SYSTEM | ATTR_HIDDEN);
    assert!(with_attrib.must.is_empty());

    let with_readonly = parse_query("system: readonly:", now());
    assert_eq!(with_readonly.attrib_mask, ATTR_SYSTEM | ATTR_READONLY);
    assert!(with_readonly.must.is_empty());
}

#[test]
fn non_empty_rest_is_not_an_alias() {
    for input in ["system:yes", "system:S", "system:1"] {
        let q = parse_query(input, now());
        assert_eq!(q.attrib_mask, 0, "{input}");
        assert_eq!(q.attrib_any, 0, "{input}");
    }
}

#[test]
fn system_filters_records() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "plain.txt", r"C:\docs\plain.txt", false, 0));
    catalog.insert(rec(
        2,
        "readonly.txt",
        r"C:\docs\readonly.txt",
        false,
        ATTR_READONLY,
    ));
    catalog.insert(rec(
        3,
        "hidden.txt",
        r"C:\docs\hidden.txt",
        false,
        ATTR_HIDDEN,
    ));
    catalog.insert(rec(
        4,
        "system.txt",
        r"C:\docs\system.txt",
        false,
        ATTR_SYSTEM,
    ));
    catalog.insert(rec(
        5,
        "both.txt",
        r"C:\docs\both.txt",
        false,
        ATTR_SYSTEM | ATTR_HIDDEN,
    ));
    catalog.insert(rec(6, "work", r"C:\work", true, ATTR_DIRECTORY));
    catalog.insert(rec(
        7,
        "system-dir",
        r"C:\system-dir",
        true,
        ATTR_SYSTEM | ATTR_DIRECTORY,
    ));
    let content = ContentIndex::new();

    for input in ["system:", "SYSTEM:", "attrib:S"] {
        let hits = search(&catalog, &content, &parse_query(input, now()), now());
        let got = names(&hits);
        assert!(got.contains(&"system.txt".into()), "{input}: {got:?}");
        assert!(got.contains(&"both.txt".into()), "{input}: {got:?}");
        assert!(got.contains(&"system-dir".into()), "{input}: {got:?}");
        assert!(!got.contains(&"plain.txt".into()), "{input}: {got:?}");
        assert!(!got.contains(&"readonly.txt".into()), "{input}: {got:?}");
        assert!(!got.contains(&"hidden.txt".into()), "{input}: {got:?}");
        assert!(!got.contains(&"work".into()), "{input}: {got:?}");
    }

    let both = search(
        &catalog,
        &content,
        &parse_query("system: hidden:", now()),
        now(),
    );
    assert_eq!(names(&both), vec!["both.txt".to_string()]);
}
