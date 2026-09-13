//! Skip `.git` / `node_modules` children while walking the name catalog.

use std::fs;
use std::path::Path;

use flashseek::walk::ingest_tree;
use flashseek::Engine;

fn catalog_names(engine: &Engine) -> Vec<String> {
    engine.catalog.iter().map(|r| r.name.clone()).collect()
}

fn has_name(engine: &Engine, name: &str) -> bool {
    engine.catalog.iter().any(|r| r.name == name)
}

fn write_tree(root: &Path) {
    fs::create_dir(root.join(".git")).unwrap();
    fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/main").unwrap();
    fs::write(root.join("keep.txt"), "keep").unwrap();
}

#[test]
fn ingest_skips_git_children_keeps_sibling_files() {
    let tmp = tempfile::tempdir().unwrap();
    write_tree(tmp.path());

    let mut engine = Engine::new();
    ingest_tree(&mut engine, tmp.path(), &[]).unwrap();

    let names = catalog_names(&engine);
    assert!(has_name(&engine, "keep.txt"), "{names:?}");
    assert!(!has_name(&engine, "HEAD"), "{names:?}");
}

#[test]
fn name_root_at_git_still_indexes_children() {
    let tmp = tempfile::tempdir().unwrap();
    write_tree(tmp.path());
    let git = tmp.path().join(".git");

    let mut engine = Engine::new();
    ingest_tree(&mut engine, &git, &[]).unwrap();

    let names = catalog_names(&engine);
    assert!(has_name(&engine, "HEAD"), "{names:?}");
}

#[test]
fn ingest_skips_node_modules_svn_and_hg_children() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir(root.join("node_modules")).unwrap();
    fs::write(root.join("node_modules").join("pkg.js"), "module.exports = 1").unwrap();
    fs::create_dir(root.join(".svn")).unwrap();
    fs::write(root.join(".svn").join("entries"), "svn").unwrap();
    fs::create_dir(root.join(".hg")).unwrap();
    fs::write(root.join(".hg").join("dirstate"), "hg").unwrap();
    fs::write(root.join("keep.txt"), "keep").unwrap();

    let mut engine = Engine::new();
    ingest_tree(&mut engine, root, &[]).unwrap();

    let names = catalog_names(&engine);
    assert!(has_name(&engine, "keep.txt"), "{names:?}");
    assert!(!has_name(&engine, "pkg.js"), "{names:?}");
    assert!(!has_name(&engine, "entries"), "{names:?}");
    assert!(!has_name(&engine, "dirstate"), "{names:?}");
}
