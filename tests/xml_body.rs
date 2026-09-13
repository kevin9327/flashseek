use std::time::SystemTime;

use flashseek::{extract_text, Engine};

#[test]
fn xml_body_only_hit() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("note.xml");
    std::fs::write(&path, "<root><item>zebratok</item></root>").unwrap();
    assert!(extract_text(&path).unwrap().contains("zebratok"));
    let engine = Engine::from_roots(tmp.path(), &[tmp.path().to_path_buf()]).unwrap();
    let hits = engine.query("zebratok", SystemTime::now());
    assert!(
        hits.iter().any(|h| h.record.name == "note.xml" && h.content_match && !h.name_match),
        "{:?}",
        hits.iter().map(|h| &h.record.name).collect::<Vec<_>>()
    );
}
