//! Everything-class `exe:` / `doc:` macros SET extension lists.

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

const EXE: &[&str] = &["exe", "bat", "cmd", "com", "msi", "ps1"];
const DOC: &[&str] = &["pdf", "doc", "docx", "txt", "md", "rtf", "odt"];

fn ext_strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn exe_sets_executable_extension_list() {
    for input in ["exe:", "EXE:", "Exe:"] {
        let q = parse_query(input, now());
        assert_eq!(q.extensions, ext_strings(EXE), "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn doc_sets_document_extension_list() {
    for input in ["doc:", "DOC:", "Doc:"] {
        let q = parse_query(input, now());
        assert_eq!(q.extensions, ext_strings(DOC), "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn exe_and_doc_match_ext_semicolon_lists() {
    let executables = parse_query("ext:exe;bat;cmd;com;msi;ps1", now());
    assert_eq!(parse_query("exe:", now()).extensions, executables.extensions);

    let documents = parse_query("ext:pdf;doc;docx;txt;md;rtf;odt", now());
    assert_eq!(parse_query("doc:", now()).extensions, documents.extensions);
}

#[test]
fn exe_and_doc_are_filters_not_terms() {
    let q = parse_query("setup exe:", now());
    assert_eq!(q.extensions, ext_strings(EXE));
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "setup"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("report doc:", now());
    assert_eq!(q.extensions, ext_strings(DOC));
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "report"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn last_extension_token_wins() {
    let after_exe = parse_query("exe: ext:pdf", now());
    assert_eq!(after_exe.extensions, vec!["pdf".to_string()]);
    assert!(after_exe.must.is_empty());

    let after_ext = parse_query("ext:pdf exe:", now());
    assert_eq!(after_ext.extensions, ext_strings(EXE));
    assert!(after_ext.must.is_empty());

    let after_doc = parse_query("doc: ext:zip", now());
    assert_eq!(after_doc.extensions, vec!["zip".to_string()]);
    assert!(after_doc.must.is_empty());

    let ext_then_doc = parse_query("ext:zip doc:", now());
    assert_eq!(ext_then_doc.extensions, ext_strings(DOC));
    assert!(ext_then_doc.must.is_empty());

    let exe_then_doc = parse_query("exe: doc:", now());
    assert_eq!(exe_then_doc.extensions, ext_strings(DOC));

    let pic_then_exe = parse_query("pic: exe:", now());
    assert_eq!(pic_then_exe.extensions, ext_strings(EXE));
}

#[test]
fn exec_is_not_an_exe_filter() {
    let q = parse_query("exec:", now());
    assert!(q.extensions.is_empty());
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "exec:"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn docker_is_not_a_doc_filter() {
    let q = parse_query("docker:", now());
    assert!(q.extensions.is_empty());
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "docker:"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn exe_and_doc_filter_search_hits() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "setup.exe", r"C:\media\setup.exe"));
    catalog.insert(rec(2, "run.bat", r"C:\media\run.bat"));
    catalog.insert(rec(3, "tool.cmd", r"C:\media\tool.cmd"));
    catalog.insert(rec(4, "legacy.com", r"C:\media\legacy.com"));
    catalog.insert(rec(5, "install.msi", r"C:\media\install.msi"));
    catalog.insert(rec(6, "build.ps1", r"C:\media\build.ps1"));
    catalog.insert(rec(7, "report.pdf", r"C:\media\report.pdf"));
    catalog.insert(rec(8, "notes.doc", r"C:\media\notes.doc"));
    catalog.insert(rec(9, "letter.docx", r"C:\media\letter.docx"));
    catalog.insert(rec(10, "readme.txt", r"C:\media\readme.txt"));
    catalog.insert(rec(11, "guide.md", r"C:\media\guide.md"));
    catalog.insert(rec(12, "memo.rtf", r"C:\media\memo.rtf"));
    catalog.insert(rec(13, "draft.odt", r"C:\media\draft.odt"));
    catalog.insert(rec(14, "photo.jpg", r"C:\media\photo.jpg"));
    catalog.insert(rec(15, "setup.txt", r"C:\media\setup.txt"));
    catalog.insert(rec(16, "report.exe", r"C:\media\report.exe"));
    let content = ContentIndex::new();

    let exe = names(&search(
        &catalog,
        &content,
        &parse_query("exe:", now()),
        now(),
    ));
    assert!(exe.contains(&"setup.exe".into()), "{exe:?}");
    assert!(exe.contains(&"run.bat".into()), "{exe:?}");
    assert!(exe.contains(&"tool.cmd".into()), "{exe:?}");
    assert!(exe.contains(&"legacy.com".into()), "{exe:?}");
    assert!(exe.contains(&"install.msi".into()), "{exe:?}");
    assert!(exe.contains(&"build.ps1".into()), "{exe:?}");
    assert!(exe.contains(&"report.exe".into()), "{exe:?}");
    assert!(!exe.contains(&"report.pdf".into()), "{exe:?}");
    assert!(!exe.contains(&"photo.jpg".into()), "{exe:?}");
    assert!(!exe.contains(&"setup.txt".into()), "{exe:?}");
    assert_eq!(exe.len(), 7, "{exe:?}");

    let doc = names(&search(
        &catalog,
        &content,
        &parse_query("doc:", now()),
        now(),
    ));
    assert!(doc.contains(&"report.pdf".into()), "{doc:?}");
    assert!(doc.contains(&"notes.doc".into()), "{doc:?}");
    assert!(doc.contains(&"letter.docx".into()), "{doc:?}");
    assert!(doc.contains(&"readme.txt".into()), "{doc:?}");
    assert!(doc.contains(&"guide.md".into()), "{doc:?}");
    assert!(doc.contains(&"memo.rtf".into()), "{doc:?}");
    assert!(doc.contains(&"draft.odt".into()), "{doc:?}");
    assert!(doc.contains(&"setup.txt".into()), "{doc:?}");
    assert!(!doc.contains(&"setup.exe".into()), "{doc:?}");
    assert!(!doc.contains(&"photo.jpg".into()), "{doc:?}");
    assert_eq!(doc.len(), 8, "{doc:?}");

    let mixed_exe = names(&search(
        &catalog,
        &content,
        &parse_query("setup exe:", now()),
        now(),
    ));
    assert!(mixed_exe.contains(&"setup.exe".into()), "{mixed_exe:?}");
    assert!(!mixed_exe.contains(&"run.bat".into()), "{mixed_exe:?}");
    assert!(!mixed_exe.contains(&"setup.txt".into()), "{mixed_exe:?}");
    assert_eq!(mixed_exe.len(), 1, "{mixed_exe:?}");

    let mixed_doc = names(&search(
        &catalog,
        &content,
        &parse_query("report doc:", now()),
        now(),
    ));
    assert!(mixed_doc.contains(&"report.pdf".into()), "{mixed_doc:?}");
    assert!(!mixed_doc.contains(&"report.exe".into()), "{mixed_doc:?}");
    assert!(!mixed_doc.contains(&"notes.doc".into()), "{mixed_doc:?}");
    assert_eq!(mixed_doc.len(), 1, "{mixed_doc:?}");
}
