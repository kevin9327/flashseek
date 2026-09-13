//! Extra Korean NL compile + Engine::query coverage (date windows, type words).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use filetime::{set_file_mtime, FileTime};
use flashseek::{compile_nl, Engine};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const NOW_SECS: u64 = 1_800_000_000; // frozen, same as gating.rs

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(NOW_SECS)
}

fn hours_ago(hours: u64) -> SystemTime {
    now() - Duration::from_secs(hours * 3600)
}

fn days_ago(days: u64) -> SystemTime {
    now() - Duration::from_secs(days * 86400)
}

fn set_mtime(path: &Path, t: SystemTime) {
    set_file_mtime(path, FileTime::from_system_time(t)).unwrap();
}

fn write_txt(dir: &Path, rel: &str, body: &str, modified: SystemTime) -> PathBuf {
    let path = dir.join(rel);
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(&path, body).unwrap();
    set_mtime(&path, modified);
    path
}

fn write_pdf(dir: &Path, rel: &str, body_token: &str, modified: SystemTime) -> PathBuf {
    let path = dir.join(rel);
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    let stream = format!("BT /F1 12 Tf 10 100 Td ({body_token}) Tj ET");
    let pdf = format!(
        "%PDF-1.1\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n4 0 obj << /Length {} >> stream\n{stream}\nendstream\nendobj\n5 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> endobj\n{body_token}\n%%EOF\n",
        stream.len()
    );
    fs::write(&path, pdf).unwrap();
    set_mtime(&path, modified);
    path
}

fn write_docx(dir: &Path, rel: &str, body_token: &str, modified: SystemTime) -> PathBuf {
    write_office(
        dir,
        rel,
        body_token,
        modified,
        "word/document.xml",
        &format!(
            "<?xml version=\"1.0\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body><w:p><w:r><w:t>{body_token}</w:t></w:r></w:p></w:body></w:document>"
        ),
    )
}

fn write_xlsx(dir: &Path, rel: &str, body_token: &str, modified: SystemTime) -> PathBuf {
    write_office(
        dir,
        rel,
        body_token,
        modified,
        "xl/sharedStrings.xml",
        &format!("<?xml version=\"1.0\"?><sst><si><t>{body_token}</t></si></sst>"),
    )
}

fn write_pptx(dir: &Path, rel: &str, body_token: &str, modified: SystemTime) -> PathBuf {
    write_office(
        dir,
        rel,
        body_token,
        modified,
        "ppt/slides/slide1.xml",
        &format!("<?xml version=\"1.0\"?><p:sld><p:cSld><p:spTree><a:t>{body_token}</a:t></p:spTree></p:cSld></p:sld>"),
    )
}

fn write_office(
    dir: &Path,
    rel: &str,
    _body_token: &str,
    modified: SystemTime,
    inner: &str,
    xml: &str,
) -> PathBuf {
    let path = dir.join(rel);
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    let file = fs::File::create(&path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opt = SimpleFileOptions::default();
    zip.start_file("[Content_Types].xml", opt).unwrap();
    zip.write_all(b"<?xml version=\"1.0\"?><Types></Types>")
        .unwrap();
    zip.start_file(inner, opt).unwrap();
    zip.write_all(xml.as_bytes()).unwrap();
    zip.finish().unwrap();
    set_mtime(&path, modified);
    path
}

fn names(hits: &[flashseek::Hit]) -> Vec<String> {
    hits.iter().map(|h| h.record.name.clone()).collect()
}

fn window_secs(q: &flashseek::Query, after: bool) -> u64 {
    let t = if after {
        q.modified_after
    } else {
        q.modified_before
    };
    now().duration_since(t.unwrap()).unwrap().as_secs()
}

#[test]
fn compile_nl_keeps_last_week_tax_pdf() {
    let q = compile_nl("지난주 세금 pdf", now());
    assert_eq!(q.extensions, vec!["pdf".to_string()]);
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        flashseek::query::Atom::Term(p) => assert_eq!(p.raw, "세금"),
        _ => panic!("expected term"),
    }
    assert_eq!(window_secs(&q, true), 7 * 24 * 3600);
    assert!(q.modified_before.is_none());
}

#[test]
fn compile_nl_korean_dates_and_type_words() {
    let today = compile_nl("오늘 메모", now());
    assert_eq!(window_secs(&today, true), 24 * 3600);
    assert!(today.modified_before.is_none());

    let yest = compile_nl("어제 메모", now());
    assert_eq!(window_secs(&yest, true), 48 * 3600);
    assert_eq!(window_secs(&yest, false), 24 * 3600);

    let month = compile_nl("이번달 분기실적 스프레드시트", now());
    assert_eq!(window_secs(&month, true), 30 * 24 * 3600);
    assert_eq!(month.extensions, vec!["xlsx".to_string()]);
    assert_eq!(month.must.len(), 1);
    match &month.must[0] {
        flashseek::query::Atom::Term(p) => assert_eq!(p.raw, "분기실적"),
        _ => panic!("expected term"),
    }

    let spaced = compile_nl("이번 달 분기실적", now());
    assert_eq!(window_secs(&spaced, true), 30 * 24 * 3600);
    assert_eq!(spaced.must.len(), 1);

    let year = compile_nl("올해 분기실적", now());
    assert_eq!(window_secs(&year, true), 365 * 24 * 3600);

    let docs = compile_nl("회의 문서", now());
    assert_eq!(docs.extensions, vec!["docx".to_string(), "pdf".to_string()]);

    let slides = compile_nl("발표 슬라이드", now());
    assert_eq!(slides.extensions, vec!["pptx".to_string()]);
}

#[test]
fn this_month_spreadsheet_word_filters_ext_and_date() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_xlsx(root, "in-window.xlsx", "분기실적", days_ago(5));
    write_xlsx(root, "too-old.xlsx", "분기실적", days_ago(60));
    write_txt(root, "wrong-ext.txt", "분기실적", days_ago(5));
    write_pdf(root, "wrong-ext.pdf", "분기실적", days_ago(5));
    write_xlsx(root, "other-token.xlsx", " unrelated ", days_ago(5));

    let engine = Engine::from_roots(root, &[root.to_path_buf()]).unwrap();

    for input in ["이번달 분기실적 스프레드시트", "이번 달 분기실적 스프레드시트"] {
        let q = compile_nl(input, now());
        assert_eq!(q.extensions, vec!["xlsx".to_string()], "{input}");
        assert_eq!(window_secs(&q, true), 30 * 24 * 3600, "{input}");

        let hits = engine.query(input, now());
        let got = names(&hits);
        assert!(
            got.contains(&"in-window.xlsx".into()),
            "{input} missed in-window xlsx, got {got:?}"
        );
        assert!(
            !got.contains(&"too-old.xlsx".into()),
            "{input} leaked old xlsx, got {got:?}"
        );
        assert!(
            !got.contains(&"wrong-ext.txt".into()) && !got.contains(&"wrong-ext.pdf".into()),
            "{input} leaked wrong ext, got {got:?}"
        );
        assert!(
            !got.contains(&"other-token.xlsx".into()),
            "{input} leaked other token, got {got:?}"
        );
        let hit = hits
            .iter()
            .find(|h| h.record.name == "in-window.xlsx")
            .unwrap();
        assert!(
            !hit.record.name.contains("분기실적"),
            "name must not contain the token"
        );
    }
}

#[test]
fn today_yesterday_year_and_document_word() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_txt(root, "today-note.txt", "메모", hours_ago(6));
    write_txt(root, "yesterday-note.txt", "메모", hours_ago(36));
    write_txt(root, "old-note.txt", "메모", days_ago(10));
    write_docx(root, "minutes.docx", "회의록", days_ago(3));
    write_pdf(root, "minutes.pdf", "회의록", days_ago(3));
    write_txt(root, "minutes.txt", "회의록", days_ago(3));
    write_pptx(root, "deck.pptx", "발표자료", days_ago(20));
    write_xlsx(root, "year-sheet.xlsx", "연간", days_ago(200));
    write_xlsx(root, "ancient.xlsx", "연간", days_ago(400));

    let engine = Engine::from_roots(root, &[root.to_path_buf()]).unwrap();

    let today = names(&engine.query("오늘 메모", now()));
    assert!(today.contains(&"today-note.txt".into()), "{today:?}");
    assert!(!today.contains(&"yesterday-note.txt".into()), "{today:?}");

    let yest = names(&engine.query("어제 메모", now()));
    assert!(yest.contains(&"yesterday-note.txt".into()), "{yest:?}");
    assert!(!yest.contains(&"today-note.txt".into()), "{yest:?}");
    assert!(!yest.contains(&"old-note.txt".into()), "{yest:?}");

    let docs = names(&engine.query("회의록 문서", now()));
    assert!(docs.contains(&"minutes.docx".into()), "{docs:?}");
    assert!(docs.contains(&"minutes.pdf".into()), "{docs:?}");
    assert!(!docs.contains(&"minutes.txt".into()), "{docs:?}");

    let slides = names(&engine.query("발표자료 슬라이드", now()));
    assert_eq!(slides, vec!["deck.pptx".to_string()]);

    let year = names(&engine.query("올해 연간 스프레드시트", now()));
    assert!(year.contains(&"year-sheet.xlsx".into()), "{year:?}");
    assert!(!year.contains(&"ancient.xlsx".into()), "{year:?}");
}
