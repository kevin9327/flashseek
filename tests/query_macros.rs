//! Everything-class `pic:` / `video:` / `audio:` macros SET extension lists.

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

const PIC: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "webp"];
const VIDEO: &[&str] = &["mp4", "mkv", "avi", "webm", "mov"];
const AUDIO: &[&str] = &["mp3", "wav", "flac", "aac", "ogg"];

fn ext_strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn pic_video_audio_set_extension_lists() {
    for input in ["pic:", "PIC:", "Pic:"] {
        let q = parse_query(input, now());
        assert_eq!(q.extensions, ext_strings(PIC), "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
    for input in ["video:", "VIDEO:", "Video:"] {
        let q = parse_query(input, now());
        assert_eq!(q.extensions, ext_strings(VIDEO), "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
    for input in ["audio:", "AUDIO:", "Audio:"] {
        let q = parse_query(input, now());
        assert_eq!(q.extensions, ext_strings(AUDIO), "{input}");
        assert!(q.must.is_empty(), "{input}");
    }
}

#[test]
fn macros_match_ext_semicolon_lists() {
    let pic = parse_query("ext:jpg;jpeg;png;gif;bmp;webp", now());
    assert_eq!(parse_query("pic:", now()).extensions, pic.extensions);

    let video = parse_query("ext:mp4;mkv;avi;webm;mov", now());
    assert_eq!(parse_query("video:", now()).extensions, video.extensions);

    let audio = parse_query("ext:mp3;wav;flac;aac;ogg", now());
    assert_eq!(parse_query("audio:", now()).extensions, audio.extensions);
}

#[test]
fn macros_are_filters_not_terms() {
    let q = parse_query("vacation pic:", now());
    assert_eq!(q.extensions, ext_strings(PIC));
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "vacation"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("clip video:", now());
    assert_eq!(q.extensions, ext_strings(VIDEO));
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "clip"),
        other => panic!("expected Term, got {other:?}"),
    }

    let q = parse_query("track audio:", now());
    assert_eq!(q.extensions, ext_strings(AUDIO));
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "track"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn last_extension_token_wins() {
    let after_pic = parse_query("pic: ext:pdf", now());
    assert_eq!(after_pic.extensions, vec!["pdf".to_string()]);
    assert!(after_pic.must.is_empty());

    let after_ext = parse_query("ext:pdf pic:", now());
    assert_eq!(after_ext.extensions, ext_strings(PIC));
    assert!(after_ext.must.is_empty());

    let video_then_audio = parse_query("video: audio:", now());
    assert_eq!(video_then_audio.extensions, ext_strings(AUDIO));
}

#[test]
fn picture_is_not_a_pic_filter() {
    let q = parse_query("picture:", now());
    assert!(q.extensions.is_empty());
    assert_eq!(q.must.len(), 1);
    match &q.must[0] {
        Atom::Term(p) => assert_eq!(p.raw, "picture:"),
        other => panic!("expected Term, got {other:?}"),
    }
}

#[test]
fn macros_filter_search_hits() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "photo.jpg", r"C:\media\photo.jpg"));
    catalog.insert(rec(2, "photo.png", r"C:\media\photo.png"));
    catalog.insert(rec(3, "clip.mp4", r"C:\media\clip.mp4"));
    catalog.insert(rec(4, "song.mp3", r"C:\media\song.mp3"));
    catalog.insert(rec(5, "notes.txt", r"C:\media\notes.txt"));
    catalog.insert(rec(6, "scan.webp", r"C:\media\scan.webp"));
    catalog.insert(rec(7, "film.mkv", r"C:\media\film.mkv"));
    catalog.insert(rec(8, "take.flac", r"C:\media\take.flac"));
    let content = ContentIndex::new();

    let pic = names(&search(
        &catalog,
        &content,
        &parse_query("pic:", now()),
        now(),
    ));
    assert!(pic.contains(&"photo.jpg".into()), "{pic:?}");
    assert!(pic.contains(&"photo.png".into()), "{pic:?}");
    assert!(pic.contains(&"scan.webp".into()), "{pic:?}");
    assert!(!pic.contains(&"clip.mp4".into()), "{pic:?}");
    assert!(!pic.contains(&"song.mp3".into()), "{pic:?}");
    assert!(!pic.contains(&"notes.txt".into()), "{pic:?}");
    assert_eq!(pic.len(), 3, "{pic:?}");

    let video = names(&search(
        &catalog,
        &content,
        &parse_query("video:", now()),
        now(),
    ));
    assert!(video.contains(&"clip.mp4".into()), "{video:?}");
    assert!(video.contains(&"film.mkv".into()), "{video:?}");
    assert!(!video.contains(&"photo.jpg".into()), "{video:?}");
    assert!(!video.contains(&"song.mp3".into()), "{video:?}");
    assert_eq!(video.len(), 2, "{video:?}");

    let audio = names(&search(
        &catalog,
        &content,
        &parse_query("audio:", now()),
        now(),
    ));
    assert!(audio.contains(&"song.mp3".into()), "{audio:?}");
    assert!(audio.contains(&"take.flac".into()), "{audio:?}");
    assert!(!audio.contains(&"clip.mp4".into()), "{audio:?}");
    assert!(!audio.contains(&"notes.txt".into()), "{audio:?}");
    assert_eq!(audio.len(), 2, "{audio:?}");

    let mixed = names(&search(
        &catalog,
        &content,
        &parse_query("photo pic:", now()),
        now(),
    ));
    assert!(mixed.contains(&"photo.jpg".into()), "{mixed:?}");
    assert!(mixed.contains(&"photo.png".into()), "{mixed:?}");
    assert!(!mixed.contains(&"scan.webp".into()), "{mixed:?}");
    assert_eq!(mixed.len(), 2, "{mixed:?}");
}
