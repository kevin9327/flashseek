use flashseek::extract_text;

#[test]
fn extract_text_strips_utf8_bom() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("note.txt");
    let mut body = vec![0xEF, 0xBB, 0xBF];
    body.extend_from_slice(b"bomtoken");
    std::fs::write(&path, &body).unwrap();
    let text = extract_text(&path).unwrap();
    assert_eq!(text, "bomtoken");
    assert!(text.contains("bomtoken"));
    assert!(!text.starts_with('\u{feff}'));
}
