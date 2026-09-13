//! Body-only extract + query for html, xlsx, and pptx via shipped extract_text + Engine.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use flashseek::{extract_text, Engine};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const NOW_SECS: u64 = 1_800_000_000;

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(NOW_SECS)
}

fn write_html(dir: &Path, rel: &str, body_token: &str) -> PathBuf {
    let path = dir.join(rel);
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    let html = format!(
        "<!DOCTYPE html><html><head><title>notes</title></head><body><h1>memo</h1><p>secret <b>{body_token}</b> only in body</p></body></html>"
    );
    fs::write(&path, html).unwrap();
    path
}

fn write_xlsx(dir: &Path, rel: &str, body_token: &str) -> PathBuf {
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
    zip.start_file("xl/sharedStrings.xml", opt).unwrap();
    let xml = format!(
        "<?xml version=\"1.0\"?><sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" count=\"1\" uniqueCount=\"1\"><si><t>{body_token}</t></si></sst>"
    );
    zip.write_all(xml.as_bytes()).unwrap();
    zip.finish().unwrap();
    path
}

fn write_pptx(dir: &Path, rel: &str, body_token: &str) -> PathBuf {
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
    zip.start_file("ppt/slides/slide1.xml", opt).unwrap();
    let xml = format!(
        "<?xml version=\"1.0\"?><p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>{body_token}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"
    );
    zip.write_all(xml.as_bytes()).unwrap();
    zip.finish().unwrap();
    path
}

fn ingest(root: &Path) -> Engine {
    Engine::from_roots(root, &[root.to_path_buf()]).unwrap()
}

fn hit_named<'a>(hits: &'a [flashseek::Hit], name: &str) -> &'a flashseek::Hit {
    hits.iter()
        .find(|h| h.record.name == name)
        .unwrap_or_else(|| panic!("expected {name} in {:?}", hits.iter().map(|h| &h.record.name).collect::<Vec<_>>()))
}

fn assert_body_only_hit(engine: &Engine, token: &str, file_name: &str) {
    assert!(
        !file_name.contains(token),
        "token {token} must not appear in filename {file_name}"
    );
    let hits = engine.query(token, now());
    let hit = hit_named(&hits, file_name);
    assert!(!hit.name_match, "name must not match for {file_name}");
    assert!(hit.content_match, "content must match for {file_name}");
    let sn = hit.snippet.as_ref().expect("preview snippet");
    assert!(
        sn.text.contains(token),
        "snippet for {file_name} was {:?}",
        sn.text
    );
}

#[test]
fn html_extract_and_body_only_query() {
    let tmp = tempfile::tempdir().unwrap();
    let token = "htmlbodytoken";
    let path = write_html(tmp.path(), "page.html", token);
    let extracted = extract_text(&path).unwrap();
    assert!(
        extracted.contains(token),
        "extract_text html missed token, got {extracted:?}"
    );
    assert!(
        !extracted.contains("<b>"),
        "html tags should be stripped, got {extracted:?}"
    );

    let engine = ingest(tmp.path());
    assert_body_only_hit(&engine, token, "page.html");
}

#[test]
fn xlsx_extract_and_body_only_query() {
    let tmp = tempfile::tempdir().unwrap();
    let token = "xlsxsharedcell";
    let path = write_xlsx(tmp.path(), "sheet.xlsx", token);
    let extracted = extract_text(&path).unwrap();
    assert!(
        extracted.contains(token),
        "extract_text xlsx missed token, got {extracted:?}"
    );

    let engine = ingest(tmp.path());
    assert_body_only_hit(&engine, token, "sheet.xlsx");
}

#[test]
fn pptx_extract_and_body_only_query() {
    let tmp = tempfile::tempdir().unwrap();
    let token = "pptxslidetoken";
    let path = write_pptx(tmp.path(), "deck.pptx", token);
    let extracted = extract_text(&path).unwrap();
    assert!(
        extracted.contains(token),
        "extract_text pptx missed token, got {extracted:?}"
    );

    let engine = ingest(tmp.path());
    assert_body_only_hit(&engine, token, "deck.pptx");
}
