//! Gating tests: shipped parse → name-match → content-match → rank on fixture files.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use filetime::{set_file_mtime, FileTime};
use flashseek::{compile_nl, extract_text, Engine};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const NOW_SECS: u64 = 1_800_000_000; // 2027-01-15-ish, frozen

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(NOW_SECS)
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
    // Minimal PDF with the token as a literal `(...)` and as raw UTF-8.
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
    let path = dir.join(rel);
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    let file = fs::File::create(&path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opt = SimpleFileOptions::default();
    zip.start_file("[Content_Types].xml", opt).unwrap();
    zip.write_all(b"<?xml version=\"1.0\"?><Types></Types>").unwrap();
    zip.start_file("word/document.xml", opt).unwrap();
    let xml = format!(
        "<?xml version=\"1.0\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body><w:p><w:r><w:t>{body_token}</w:t></w:r></w:p></w:body></w:document>"
    );
    zip.write_all(xml.as_bytes()).unwrap();
    zip.finish().unwrap();
    set_mtime(&path, modified);
    path
}

fn corpus() -> (tempfile::TempDir, Engine) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_txt(root, "notes\\alpha-report.txt", "nothing special", days_ago(1));
    write_txt(root, "notes\\beta-memo.txt", "hello world", days_ago(1));
    write_txt(root, "notes\\gamma.log", "tmp scratch", days_ago(1));
    write_txt(
        root,
        "docs\\body-only.txt",
        "the secret token unicorn lives only in the body",
        days_ago(1),
    );
    write_pdf(
        root,
        "docs\\invoice-march.pdf",
        "세금",
        days_ago(2),
    );
    write_pdf(
        root,
        "docs\\old-invoice.pdf",
        "세금",
        days_ago(30),
    );
    write_pdf(
        root,
        "docs\\wrong-type.pdf",
        "nope",
        days_ago(2),
    );
    write_txt(
        root,
        "docs\\tax-note.txt",
        "세금 but not a pdf",
        days_ago(2),
    );
    write_docx(
        root,
        "docs\\minutes.docx",
        "unicorn-office",
        days_ago(1),
    );
    write_txt(root, "big\\huge.bin", &"x".repeat(2 * 1024 * 1024), days_ago(1));
    write_txt(root, "big\\tiny.bin", "y", days_ago(1));
    let engine = Engine::from_roots(root, &[root.to_path_buf()]).unwrap();
    (tmp, engine)
}

fn paths(hits: &[flashseek::Hit]) -> Vec<String> {
    hits.iter()
        .map(|h| h.record.name.clone())
        .collect()
}

#[test]
fn partial_name_hit_from_complete_catalog() {
    let (_tmp, engine) = corpus();
    let hits = engine.query("alpha", now());
    assert!(
        paths(&hits).iter().any(|n| n == "alpha-report.txt"),
        "partial name should hit, got {hits:?}"
    );
    // Catalog is the full tree we ingested, not a subset of "indexed locations".
    assert!(engine.catalog.len() >= 10, "catalog was {:?}", engine.catalog.len());
}

#[test]
fn and_or_not_wildcard_ext_size_date() {
    let (_tmp, engine) = corpus();

    let and = engine.query("alpha report", now());
    assert!(paths(&and).contains(&"alpha-report.txt".into()));
    assert!(!paths(&and).contains(&"beta-memo.txt".into()));

    let or_hits = engine.query("alpha|beta", now());
    let names = paths(&or_hits);
    assert!(names.contains(&"alpha-report.txt".into()));
    assert!(names.contains(&"beta-memo.txt".into()));

    let not = engine.query("memo !beta", now());
    assert!(!paths(&not).contains(&"beta-memo.txt".into()));

    let wild = engine.query("alpha*", now());
    assert!(paths(&wild).contains(&"alpha-report.txt".into()));

    let ext = engine.query("ext:pdf", now());
    assert!(paths(&ext).iter().all(|n| n.ends_with(".pdf")));
    assert!(paths(&ext).contains(&"invoice-march.pdf".into()));

    let size = engine.query("size:>1mb", now());
    assert!(paths(&size).contains(&"huge.bin".into()));
    assert!(!paths(&size).contains(&"tiny.bin".into()));

    let dm = engine.query("dm:lastweek ext:pdf", now());
    let dm_names = paths(&dm);
    assert!(dm_names.contains(&"invoice-march.pdf".into()));
    assert!(!dm_names.contains(&"old-invoice.pdf".into()));
}

#[test]
fn body_only_plain_text_and_office() {
    let (_tmp, engine) = corpus();
    let txt = engine.query("unicorn", now());
    assert!(
        paths(&txt).contains(&"body-only.txt".into()),
        "body-only txt, got {:?}",
        paths(&txt)
    );
    assert!(
        !txt.iter()
            .find(|h| h.record.name == "body-only.txt")
            .unwrap()
            .name_match
    );

    let docx = engine.query("unicorn-office", now());
    assert!(
        paths(&docx).contains(&"minutes.docx".into()),
        "body-only docx, got {:?}",
        paths(&docx)
    );
}

#[test]
fn korean_nl_last_week_tax_pdf_body_only() {
    let q = compile_nl("지난주 세금 pdf", now());
    assert_eq!(q.extensions, vec!["pdf".to_string()]);
    assert!(q.modified_after.is_some());

    let (_tmp, engine) = corpus();
    let hits = engine.query("지난주 세금 pdf", now());
    let hit = hits
        .iter()
        .find(|h| h.record.name == "invoice-march.pdf")
        .expect("in-window pdf whose body has 세금");
    assert!(
        !hit.record.name.contains("세금"),
        "name must not contain the token"
    );
    let sn = hit.snippet.as_ref().expect("preview snippet");
    assert!(sn.text.contains("세금"), "snippet was {:?}", sn.text);
    assert!(
        sn.highlights.iter().any(|&(a, b)| &sn.text[a..b] == "세금"),
        "highlights {:?}",
        sn.highlights
    );

    assert!(
        !hits.iter().any(|h| h.record.name == "old-invoice.pdf"),
        "outside window must not match"
    );
    assert!(
        !hits.iter().any(|h| h.record.name == "tax-note.txt"),
        "wrong ext must not match"
    );
}

#[test]
fn live_apply_create_delete_rename() {
    let (_tmp, mut engine) = corpus();
    assert!(engine.query("brand-new", now()).is_empty());

    let rec = flashseek::FileRecord {
        id: 9_001,
        parent_id: None,
        name: "brand-new.txt".into(),
        path: PathBuf::from("C:\\virt\\brand-new.txt"),
        size: 3,
        modified: now(),
        is_dir: false,
        attributes: 0,
    };
    engine.apply(flashseek::CatalogEvent::Create(rec.clone()));
    assert!(
        engine
            .query("brand-new", now())
            .iter()
            .any(|h| h.record.name == "brand-new.txt")
    );

    engine.apply(flashseek::CatalogEvent::Rename {
        id: 9_001,
        new_name: "renamed-now.txt".into(),
        new_path: PathBuf::from("C:\\virt\\renamed-now.txt"),
    });
    assert!(engine.query("brand-new", now()).is_empty());
    assert!(
        engine
            .query("renamed-now", now())
            .iter()
            .any(|h| h.record.name == "renamed-now.txt")
    );

    engine.apply(flashseek::CatalogEvent::Delete { id: 9_001 });
    assert!(engine.query("renamed-now", now()).is_empty());
}

#[test]
fn ranking_prefers_name_over_path() {
    let (_tmp, engine) = corpus();
    // "notes" appears in the path of several files; a file named with the token should win.
    let hits = engine.query("alpha", now());
    assert_eq!(hits[0].record.name, "alpha-report.txt");
    assert!(hits[0].name_match);
}

#[test]
fn extractor_reads_pdf_and_docx_bodies() {
    let tmp = tempfile::tempdir().unwrap();
    let pdf = write_pdf(tmp.path(), "a.pdf", "세금", now());
    let docx = write_docx(tmp.path(), "a.docx", "unicorn-office", now());
    assert!(extract_text(&pdf).unwrap().contains("세금"));
    assert!(extract_text(&docx).unwrap().contains("unicorn-office"));
}
