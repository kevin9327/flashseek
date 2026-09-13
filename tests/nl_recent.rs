use std::time::{Duration, SystemTime};

use flashseek::compile_nl;

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
}

#[test]
fn recent_and_week_mean_seven_days() {
    let q = compile_nl("최근 세금", now());
    let window = now().duration_since(q.modified_after.unwrap()).unwrap();
    assert_eq!(window.as_secs(), 7 * 24 * 3600);
    match &q.must[0] {
        flashseek::query::Atom::Term(p) => assert_eq!(p.raw, "세금"),
        _ => panic!("expected term"),
    }
}

#[test]
fn photo_word_sets_image_extensions() {
    let q = compile_nl("사진 휴가", now());
    assert!(q.extensions.contains(&"png".to_string()));
    assert!(q.extensions.contains(&"jpg".to_string()));
}

#[test]
fn video_and_music_words_set_extensions() {
    let q = compile_nl("동영상 휴가", now());
    assert!(q.extensions.contains(&"mp4".to_string()));
    let q = compile_nl("음악 ost", now());
    assert!(q.extensions.contains(&"mp3".to_string()));
}

#[test]
fn last_week_tax_pdf_still_compiles() {
    let q = compile_nl("지난주 세금 pdf", now());
    assert_eq!(q.extensions, vec!["pdf".to_string()]);
    match &q.must[0] {
        flashseek::query::Atom::Term(p) => assert_eq!(p.raw, "세금"),
        _ => panic!("expected 세금"),
    }
}
