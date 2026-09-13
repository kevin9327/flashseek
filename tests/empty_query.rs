use std::time::SystemTime;

use flashseek::Engine;

#[test]
fn empty_query_lists_the_complete_catalog() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.txt"), "a").unwrap();
    std::fs::write(tmp.path().join("b.log"), "b").unwrap();
    let engine = Engine::from_roots(tmp.path(), &[]).unwrap();
    let hits = engine.query("", SystemTime::now());
    let names: Vec<_> = hits.iter().map(|h| h.record.name.as_str()).collect();
    assert!(names.contains(&"a.txt"), "{names:?}");
    assert!(names.contains(&"b.log"), "{names:?}");
    assert_eq!(hits.len(), engine.catalog.len());
}
