use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::query::contains_ci;
use crate::types::Snippet;

#[derive(Debug, Default, Clone)]
pub struct ContentIndex {
    bodies: HashMap<PathBuf, String>,
}

impl ContentIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn index(&mut self, path: PathBuf, body: String) {
        self.bodies.insert(path, body);
    }

    pub fn remove(&mut self, path: &Path) {
        self.bodies.remove(path);
    }

    pub fn body(&self, path: &Path) -> Option<&str> {
        self.bodies.get(path).map(String::as_str)
    }

    pub fn contains_all(&self, path: &Path, terms: &[String]) -> bool {
        match self.body(path) {
            None => false,
            Some(body) => terms.iter().all(|t| contains_ci(body, t)),
        }
    }

    pub fn snippet_for(&self, path: &Path, terms: &[String]) -> Option<Snippet> {
        let body = self.body(path)?;
        make_snippet(body, terms, 80)
    }
}

pub fn make_snippet(body: &str, terms: &[String], ctx: usize) -> Option<Snippet> {
    if terms.is_empty() {
        return None;
    }
    let lower = body.to_lowercase();
    let mut first: Option<(usize, usize)> = None;
    for term in terms {
        if term.is_empty() {
            continue;
        }
        let t = term.to_lowercase();
        if let Some(pos) = lower.find(&t) {
            first = Some((pos, t.len()));
            break;
        }
    }
    let (pos, tlen) = first?;
    let mut start = pos.saturating_sub(ctx);
    while start > 0 && !body.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (pos + tlen + ctx).min(body.len());
    while end < body.len() && !body.is_char_boundary(end) {
        end += 1;
    }
    let text = body[start..end].to_string();
    let snippet_lower = text.to_lowercase();
    let mut highlights = Vec::new();
    for term in terms {
        if term.is_empty() {
            continue;
        }
        let t = term.to_lowercase();
        let mut from = 0;
        while let Some(rel) = snippet_lower[from..].find(&t) {
            let a = from + rel;
            let b = a + t.len();
            if text.is_char_boundary(a) && text.is_char_boundary(b) {
                highlights.push((a, b));
            }
            from = b;
            if from >= snippet_lower.len() {
                break;
            }
        }
    }
    Some(Snippet { text, highlights })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_highlights_term() {
        let s = make_snippet("alpha 세금 omega", &["세금".into()], 10).unwrap();
        assert!(s.text.contains("세금"));
        assert!(!s.highlights.is_empty());
        let (a, b) = s.highlights[0];
        assert_eq!(&s.text[a..b], "세금");
    }
}
