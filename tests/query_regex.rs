//! Everything-class `regex:` / `r:` name matching (std-only subset).

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::query::{parse_query, Atom, Pattern};
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

fn term_pattern(input: &str) -> Pattern {
    let q = parse_query(input, now());
    assert_eq!(q.must.len(), 1, "{input}");
    match &q.must[0] {
        Atom::Term(p) => p.clone(),
        other => panic!("{input}: expected Term, got {other:?}"),
    }
}

#[test]
fn regex_and_r_tokens_compile_to_regex_pattern() {
    for input in ["regex:foo.*bar", "r:foo.*bar", "REGEX:foo.*bar", "R:foo.*bar"] {
        let p = term_pattern(input);
        assert_eq!(p.raw, "foo.*bar", "{input}");
        assert!(p.regex, "{input}");
        assert!(!p.glob, "{input}");
    }

    let bare = parse_query("regex:", now());
    assert!(bare.must.is_empty());
    let bare_r = parse_query("r:", now());
    assert!(bare_r.must.is_empty());
}

#[test]
fn regex_dot_star_and_anchors() {
    let p = Pattern::regex("foo.*bar");
    assert!(p.matches("fooXYZbar.txt"));
    assert!(p.matches("FOObar"));
    assert!(!p.matches("barfoo"));
    assert!(!p.matches("foonly"));

    let any = Pattern::regex("a.c");
    assert!(any.matches("abc"));
    assert!(any.matches("a-c"));
    assert!(!any.matches("ac"));
    assert!(!any.matches("abbc"));

    let star = Pattern::regex("ab*c");
    assert!(star.matches("ac"));
    assert!(star.matches("abc"));
    assert!(star.matches("abbbc"));
    assert!(!star.matches("aXc"));

    let start = Pattern::regex("^alpha");
    assert!(start.matches("alpha.txt"));
    assert!(!start.matches("pre-alpha.txt"));

    let end = Pattern::regex("txt$");
    assert!(end.matches("notes.txt"));
    assert!(!end.matches("txt.log"));

    let full = Pattern::regex("^foo.txt$");
    assert!(full.matches("foo.txt"));
    assert!(!full.matches("afoo.txt"));
    assert!(!full.matches("foo.txt.bak"));
}

#[test]
fn regex_is_not_glob_and_honors_case() {
    let glob = Pattern::new("a.*b");
    assert!(glob.glob && !glob.regex);
    assert!(glob.matches("a.xb"));
    assert!(!glob.matches("axb"), "glob treats '.' as literal");

    let re = term_pattern("regex:a.*b");
    assert!(re.regex && !re.glob);
    assert!(re.matches("axb"));
    assert!(re.matches("a.xb"));
    assert!(re.matches("xxaYYYbzz"));

    let p = Pattern::regex("Foo.*Bar");
    assert!(p.matches("fooZZZbar"));
    assert!(p.matches_with("FooZZZBar", true, false));
    assert!(!p.matches_with("fooZZZbar", true, false));
}

#[test]
fn regex_search_matches_names() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "fooXYZbar.txt", r"C:\docs\fooXYZbar.txt"));
    catalog.insert(rec(2, "foobar.txt", r"C:\docs\foobar.txt"));
    catalog.insert(rec(3, "barfoo.txt", r"C:\docs\barfoo.txt"));
    catalog.insert(rec(4, "notes.txt", r"C:\docs\notes.txt"));
    catalog.insert(rec(5, "alpha-report.log", r"C:\docs\alpha-report.log"));
    let content = ContentIndex::new();

    let hits = search(
        &catalog,
        &content,
        &parse_query("regex:foo.*bar", now()),
        now(),
    );
    let hit_names = names(&hits);
    assert!(hit_names.contains(&"fooXYZbar.txt".into()), "{hit_names:?}");
    assert!(hit_names.contains(&"foobar.txt".into()), "{hit_names:?}");
    assert!(!hit_names.contains(&"barfoo.txt".into()), "{hit_names:?}");
    assert!(!hit_names.contains(&"notes.txt".into()), "{hit_names:?}");

    let short = search(&catalog, &content, &parse_query("r:foo.*bar", now()), now());
    assert_eq!(names(&short), names(&hits));

    let anchored = search(
        &catalog,
        &content,
        &parse_query("regex:^alpha", now()),
        now(),
    );
    assert_eq!(names(&anchored), vec!["alpha-report.log".to_string()]);

    let end = search(&catalog, &content, &parse_query("r:log$", now()), now());
    assert_eq!(names(&end), vec!["alpha-report.log".to_string()]);
}

#[test]
fn regex_honors_case_switch_and_name_only() {
    let mut catalog = Catalog::new();
    catalog.insert(rec(1, "FooZZBar.txt", r"C:\docs\FooZZBar.txt"));
    catalog.insert(rec(2, "foozzbar.txt", r"C:\docs\foozzbar.txt"));
    catalog.insert(rec(3, "other.txt", r"C:\fooZZbar\other.txt"));
    let mut content = ContentIndex::new();
    content.index(
        PathBuf::from(r"C:\fooZZbar\other.txt"),
        "body mentions FooZZBar here".into(),
    );

    let insensitive = search(
        &catalog,
        &content,
        &parse_query("regex:Foo.*Bar", now()),
        now(),
    );
    let ins_names = names(&insensitive);
    assert!(ins_names.contains(&"FooZZBar.txt".into()), "{ins_names:?}");
    assert!(ins_names.contains(&"foozzbar.txt".into()), "{ins_names:?}");

    let sensitive = search(
        &catalog,
        &content,
        &parse_query("case: regex:Foo.*Bar", now()),
        now(),
    );
    let sen_names = names(&sensitive);
    assert!(sen_names.contains(&"FooZZBar.txt".into()), "{sen_names:?}");
    assert!(!sen_names.contains(&"foozzbar.txt".into()), "{sen_names:?}");

    let named = search(
        &catalog,
        &content,
        &parse_query("n:regex:Foo.*Bar", now()),
        now(),
    );
    let named_names = names(&named);
    assert!(named_names.contains(&"FooZZBar.txt".into()), "{named_names:?}");
    assert!(named_names.contains(&"foozzbar.txt".into()), "{named_names:?}");
    assert!(
        !named_names.contains(&"other.txt".into()),
        "name-only regex ignores path/body: {named_names:?}"
    );
}
