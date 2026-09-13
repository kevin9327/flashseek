//! Everything-class `case:` and `ww:` / `wholeword:` switches.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom, Pattern};
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
fn case_token_sets_case_sensitive() {
    let bare = parse_query("case:", now());
    assert!(bare.case_sensitive);
    assert!(!bare.whole_word);
    assert!(bare.must.is_empty());

    let attached = parse_query("case:Foo", now());
    assert!(attached.case_sensitive);
    assert_eq!(attached.must.len(), 1);
    match &attached.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "Foo"),
        other => panic!("expected Term, got {other:?}"),
    }

    let spaced = parse_query("CASE: Bar", now());
    assert!(spaced.case_sensitive);
    match &spaced.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "Bar"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn ww_and_wholeword_tokens_set_whole_word() {
    for input in ["ww:", "wholeword:", "WW:", "WHOLEWORD:"] {
        let q = parse_query(input, now());
        assert!(q.whole_word, "{input}");
        assert!(!q.case_sensitive, "{input}");
        assert!(q.must.is_empty(), "{input}");
    }

    let attached = parse_query("ww:report", now());
    assert!(attached.whole_word);
    match &attached.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }

    let long = parse_query("wholeword:report", now());
    assert!(long.whole_word);
    match &long.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn default_remains_case_insensitive_substring() {
    let q = parse_query("Report", now());
    assert!(!q.case_sensitive);
    assert!(!q.whole_word);

    let p = Pattern::new("Report");
    assert!(p.matches("xxreportYY"));
    assert!(p.matches("reporting.txt"));

    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "report.txt", r"C:\docs\report.txt"));
    catalog.insert(rec(2, "REPORT.TXT", r"C:\docs\REPORT.TXT"));
    catalog.insert(rec(3, "reporting.txt", r"C:\docs\reporting.txt"));
    let content = ContentIndex::new();

    let hits = search(&catalog, &content, &q, now());
    let hit_names = names(&hits);
    assert!(hit_names.contains(&"report.txt".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"REPORT.TXT".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"reporting.txt".into()), "{hit_names:?}");
}

#[test]
fn case_sensitive_does_not_lowercase() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "Report.txt", r"C:\docs\Report.txt"));
    catalog.insert(rec(2, "report.txt", r"C:\docs\report.txt"));
    catalog.insert(rec(3, "REPORT.txt", r"C:\docs\REPORT.txt"));
    let content = ContentIndex::new();

    let hits = search(
        &catalog,
        &content,
        &parse_query("case: Report", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["Report.txt".to_string()]);

    let attached = search(
        &catalog,
        &content,
        &parse_query("case:Report", now()),
        now(),
    );
    assert_eq!(names(&attached), vec!["Report.txt".to_string()]);
}

#[test]
fn whole_word_requires_non_alnum_bounds() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "report.txt", r"C:\docs\report.txt"));
    catalog.insert(rec(2, "reporting.txt", r"C:\docs\reporting.txt"));
    catalog.insert(rec(3, "myreport.txt", r"C:\docs\myreport.txt"));
    catalog.insert(rec(4, "alpha-report.txt", r"C:\docs\alpha-report.txt"));
    let content = ContentIndex::new();

    let hits = search(&catalog, &content, &parse_query("ww: report", now()), now());
    let hit_names = names(&hits);
    assert!(hit_names.contains(&"report.txt".into()), "{hit_names:?}");
    assert!(
        hit_names.contains(&"alpha-report.txt".into()),
        "hyphen is a word boundary: {hit_names:?}"
    );
    assert!(!hit_names.contains(&"reporting.txt".into()), "{hit_names:?}");
    assert!(!hit_names.contains(&"myreport.txt".into()), "{hit_names:?}");

    let long = search(
        &catalog,
        &content,
        &parse_query("wholeword:report", now()),
        now(),
    );
    let long_names = names(&long);
    assert!(long_names.contains(&"report.txt".into()), "{long_names:?}");
    assert!(!long_names.contains(&"reporting.txt".into()), "{long_names:?}");
}

#[test]
fn hangul_syllables_count_as_word_chars() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "한글.txt", r"C:\docs\한글.txt"));
    catalog.insert(rec(2, "한글파일.txt", r"C:\docs\한글파일.txt"));
    catalog.insert(rec(3, "다른 한글 메모.txt", r"C:\docs\다른 한글 메모.txt"));
    let mut content = ContentIndex::new();
    content.index(PathBuf::from(r"C:\docs\한글파일.txt"), "한글파일 본문".into());

    let plain = search(&catalog, &content, &parse_query("한글", now()), now());
    let plain_names = names(&plain);
    assert!(plain_names.contains(&"한글.txt".into()), "{plain_names:?}");
    assert!(plain_names.contains(&"한글파일.txt".into()), "{plain_names:?}");
    assert!(plain_names.contains(&"다른 한글 메모.txt".into()), "{plain_names:?}");

    let ww = search(&catalog, &content, &parse_query("ww: 한글", now()), now());
    let ww_names = names(&ww);
    assert!(ww_names.contains(&"한글.txt".into()), "{ww_names:?}");
    assert!(ww_names.contains(&"다른 한글 메모.txt".into()), "{ww_names:?}");
    assert!(
        !ww_names.contains(&"한글파일.txt".into()),
        "Hangul syllables are word chars: {ww_names:?}"
    );
}

#[test]
fn case_and_whole_word_combine() {
    let p = Pattern::new("Report");
    assert!(p.matches_with("the Report.txt", true, true));
    assert!(!p.matches_with("the report.txt", true, true));
    assert!(!p.matches_with("Reporting.txt", true, true));

    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "the Report.txt", r"C:\docs\the Report.txt"));
    catalog.insert(rec(2, "the report.txt", r"C:\docs\the report.txt"));
    catalog.insert(rec(3, "Reporting.txt", r"C:\docs\Reporting.txt"));
    let content = ContentIndex::new();

    let hits = search(
        &catalog,
        &content,
        &parse_query("case: ww: Report", now()),
        now(),
    );
    assert_eq!(names(&hits), vec!["the Report.txt".to_string()]);
}
