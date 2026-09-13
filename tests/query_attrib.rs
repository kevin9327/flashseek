//! Everything-class `attrib:R` `attrib:H` `attrib:D` filters.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::parse_query;
use flashseek::{
    search, Catalog, ContentIndex, FileRecord, ATTR_DIRECTORY, ATTR_HIDDEN, ATTR_READONLY,
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
fn attrib_tokens_set_mask_and_any() {
    let r = parse_query("attrib:R", now());
    assert_eq!(r.attrib_mask, ATTR_READONLY);
    assert_eq!(r.attrib_any, 0);
    assert!(r.must.is_empty());

    let h = parse_query("attrib:H", now());
    assert_eq!(h.attrib_mask, ATTR_HIDDEN);
    assert_eq!(h.attrib_any, 0);

    let d = parse_query("attrib:D", now());
    assert_eq!(d.attrib_mask, ATTR_DIRECTORY);
    assert_eq!(d.attrib_any, 0);

    let combined = parse_query("attrib:RH", now());
    assert_eq!(combined.attrib_mask, ATTR_READONLY | ATTR_HIDDEN);

    let spaced = parse_query("attrib:R attrib:H", now());
    assert_eq!(spaced.attrib_mask, ATTR_READONLY | ATTR_HIDDEN);
    assert_eq!(spaced.attrib_any, 0);

    let any = parse_query("attrib:R|H", now());
    assert_eq!(any.attrib_mask, 0);
    assert_eq!(any.attrib_any, ATTR_READONLY | ATTR_HIDDEN);

    let ci = parse_query("ATTRIB:d", now());
    assert_eq!(ci.attrib_mask, ATTR_DIRECTORY);
}

#[test]
fn attrib_r_h_d_filter_records() {
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
        "both.txt",
        r"C:\docs\both.txt",
        false,
        ATTR_READONLY | ATTR_HIDDEN,
    ));
    catalog.insert(rec(5, "work", r"C:\work", true, 0));
    catalog.insert(rec(
        6,
        "hidden-dir",
        r"C:\hidden-dir",
        true,
        ATTR_HIDDEN,
    ));
    let content = ContentIndex::new();

    let ro = search(&catalog, &content, &parse_query("attrib:R", now()), now());
    let ro_names = names(&ro);
    assert!(ro_names.contains(&"readonly.txt".into()), "{ro_names:?}");
    assert!(ro_names.contains(&"both.txt".into()), "{ro_names:?}");
    assert!(!ro_names.contains(&"plain.txt".into()), "{ro_names:?}");
    assert!(!ro_names.contains(&"hidden.txt".into()), "{ro_names:?}");

    let hidden = search(&catalog, &content, &parse_query("attrib:H", now()), now());
    let hidden_names = names(&hidden);
    assert!(hidden_names.contains(&"hidden.txt".into()), "{hidden_names:?}");
    assert!(hidden_names.contains(&"both.txt".into()), "{hidden_names:?}");
    assert!(hidden_names.contains(&"hidden-dir".into()), "{hidden_names:?}");
    assert!(!hidden_names.contains(&"plain.txt".into()), "{hidden_names:?}");
    assert!(!hidden_names.contains(&"readonly.txt".into()), "{hidden_names:?}");

    let dirs = search(&catalog, &content, &parse_query("attrib:D", now()), now());
    let dir_names = names(&dirs);
    assert_eq!(dir_names.len(), 2, "{dir_names:?}");
    assert!(dir_names.contains(&"work".into()), "{dir_names:?}");
    assert!(dir_names.contains(&"hidden-dir".into()), "{dir_names:?}");

    let both = search(&catalog, &content, &parse_query("attrib:RH", now()), now());
    assert_eq!(names(&both), vec!["both.txt".to_string()]);

    let any = search(&catalog, &content, &parse_query("attrib:R|H", now()), now());
    let any_names = names(&any);
    assert!(any_names.contains(&"readonly.txt".into()), "{any_names:?}");
    assert!(any_names.contains(&"hidden.txt".into()), "{any_names:?}");
    assert!(any_names.contains(&"both.txt".into()), "{any_names:?}");
    assert!(any_names.contains(&"hidden-dir".into()), "{any_names:?}");
    assert!(!any_names.contains(&"plain.txt".into()), "{any_names:?}");
    assert!(!any_names.contains(&"work".into()), "{any_names:?}");
}

#[test]
fn attrib_d_uses_is_dir_when_directory_bit_missing() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "folder", r"C:\folder", true, 0));
    catalog.insert(rec(
        2,
        "flagged.txt",
        r"C:\flagged.txt",
        false,
        ATTR_DIRECTORY,
    ));
    let content = ContentIndex::new();

    let hits = search(&catalog, &content, &parse_query("attrib:D", now()), now());
    let hit_names = names(&hits);
    assert!(hit_names.contains(&"folder".into()), "{hit_names:?}");
    assert!(
        hit_names.contains(&"flagged.txt".into()),
        "explicit directory bit still matches: {hit_names:?}"
    );
}
