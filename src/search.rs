use std::path::Path;
use std::time::SystemTime;

use crate::catalog::Catalog;
use crate::content::ContentIndex;
use crate::nl::compile_nl;
use crate::query::{Atom, Query};
use crate::rank::rank_hits_with_terms;
use crate::types::{FileRecord, Hit};

pub fn search(catalog: &Catalog, content: &ContentIndex, query: &Query, now: SystemTime) -> Vec<Hit> {
    let terms = query.term_strings();
    let mut hits = if let Some(prefix) = name_prefix_fast_path(query) {
        collect_hits(
            catalog.names_with_prefix(prefix).into_iter(),
            content,
            query,
            &terms,
        )
    } else {
        collect_hits(catalog.iter(), content, query, &terms)
    };
    rank_hits_with_terms(&mut hits, now, &terms);
    hits
}

/// Prefix map is name-only and 1–3 chars; using it for path/body queries misses hits.
fn name_prefix_fast_path(query: &Query) -> Option<&str> {
    if !query.name_only {
        return None;
    }
    match query.must.first() {
        Some(Atom::Term(p)) if !p.glob && !p.regex && p.raw.chars().count() >= 2 => {
            Some(p.raw.as_str())
        }
        _ => None,
    }
}

fn collect_hits<'a>(
    records: impl Iterator<Item = &'a FileRecord>,
    content: &ContentIndex,
    query: &Query,
    terms: &[String],
) -> Vec<Hit> {
    let mut hits = Vec::new();
    for rec in records {
        if !filters_ok(rec.path.as_path(), rec.size, rec.modified, rec.is_dir, query) {
            continue;
        }
        let name = rec.name.as_str();
        let path = rec.path.to_string_lossy();
        let body = content.body(&rec.path);

        if query.must_not.iter().any(|p| {
            let m = |s: &str| p.matches_with(s, query.case_sensitive, query.whole_word);
            m(name) || m(&path) || body.map(m).unwrap_or(false)
        }) {
            continue;
        }

        let mut name_all = true;
        let mut path_all = true;
        let mut content_all = true;
        let mut ok = true;
        if query.must.is_empty() {
            name_all = false;
            path_all = false;
            content_all = false;
        } else {
            for atom in &query.must {
                let (n, p, c) = atom_match(atom, name, &path, body, query);
                if !(n || p || c) {
                    ok = false;
                    break;
                }
                name_all &= n;
                path_all &= p;
                content_all &= c;
            }
        }
        if !ok {
            continue;
        }
        let snippet = if content_all {
            content.snippet_for(&rec.path, terms)
        } else if let Some(b) = body {
            crate::content::make_snippet(b, terms, 80)
        } else {
            None
        };
        hits.push(Hit {
            record: rec.clone(),
            score: 0.0,
            name_match: name_all,
            path_match: path_all,
            content_match: content_all,
            snippet,
        });
    }
    hits
}

pub fn search_text(
    catalog: &Catalog,
    content: &ContentIndex,
    input: &str,
    now: SystemTime,
) -> Vec<Hit> {
    let query = compile_nl(input, now);
    search(catalog, content, &query, now)
}

fn atom_match(
    atom: &Atom,
    name: &str,
    path: &str,
    body: Option<&str>,
    query: &Query,
) -> (bool, bool, bool) {
    let n = atom.matches_text_with(name, query.case_sensitive, query.whole_word);
    if query.name_only {
        // Path and body must not satisfy the atom when `n:` / `name:` is set.
        return (n, false, false);
    }
    (
        n,
        atom.matches_text_with(path, query.case_sensitive, query.whole_word),
        body.map(|b| atom.matches_text_with(b, query.case_sensitive, query.whole_word))
            .unwrap_or(false),
    )
}

fn filters_ok(
    path: &Path,
    size: u64,
    modified: SystemTime,
    is_dir: bool,
    query: &Query,
) -> bool {
    if query.files_only && is_dir {
        return false;
    }
    if query.folders_only && !is_dir {
        return false;
    }
    if !query.extensions.is_empty() {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if is_dir || !query.extensions.iter().any(|e| e == &ext) {
            return false;
        }
    }
    if let Some(sz) = query.size {
        if !sz.matches(size) {
            return false;
        }
    }
    if let Some(after) = query.modified_after {
        if modified < after {
            return false;
        }
    }
    if let Some(before) = query.modified_before {
        if modified >= before {
            return false;
        }
    }
    true
}
