//! Cover CLI `--config FILE` by indexing a fixture through `Engine::from_config_file`.
//!
//! The CLI loads `name_root` / `content_root` from the same parser and then
//! calls `Engine::from_roots`. Hitting `from_config_file` is the shipped path.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use flashseek::Engine;

fn write_fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let notes = tmp.path().join("notes.txt");
    std::fs::write(&notes, "the token confighit is in this body\n").unwrap();
    let cfg_path = tmp.path().join("flashseek.conf");
    std::fs::write(
        &cfg_path,
        format!(
            "name_root={}\ncontent_root={}\n",
            tmp.path().display(),
            tmp.path().display()
        ),
    )
    .unwrap();
    (tmp, cfg_path, notes)
}

fn debug_cli_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/flashseek-cli.exe")
}

fn hit_paths_contain(hits: &[flashseek::Hit], expected: &Path) -> bool {
    hits.iter().any(|h| h.record.path == expected)
}

#[test]
fn from_config_file_indexes_fixture_and_finds_confighit() {
    let (_tmp, cfg_path, notes) = write_fixture();
    let engine = Engine::from_config_file(&cfg_path).unwrap();
    let hits = engine.query("confighit", SystemTime::now());
    assert!(
        hit_paths_contain(&hits, &notes),
        "expected content hit for {notes:?}, got {:?}",
        hits.iter().map(|h| &h.record.path).collect::<Vec<_>>()
    );
    assert!(
        hits.iter()
            .any(|h| h.record.path == notes && h.content_match),
        "confighit should match file body via content_root, got {hits:?}"
    );
}

#[test]
fn cli_binary_searches_fixture_via_config_when_present() {
    let bin = debug_cli_bin();
    if !bin.exists() {
        return;
    }
    let (_tmp, cfg_path, notes) = write_fixture();
    let output = Command::new(&bin)
        .args([
            "--config",
            cfg_path.to_str().expect("utf-8 config path"),
            "--query",
            "confighit",
        ])
        .output()
        .expect("spawn flashseek-cli");
    assert!(
        output.status.success(),
        "cli failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let notes_s = notes.to_string_lossy();
    assert!(
        stdout.contains(notes_s.as_ref()),
        "cli stdout missing fixture path {notes_s}: {stdout}"
    );
}
