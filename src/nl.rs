use std::time::{Duration, SystemTime};

use crate::query::{parse_query, tokenize, Atom, Pattern, Query};

const KNOWN_EXT: &[&str] = &[
    "pdf", "txt", "md", "html", "htm", "docx", "xlsx", "pptx", "png", "jpg", "jpeg", "gif", "rs",
    "py", "js", "ts", "json", "xml", "csv", "zip", "log",
];

/// Compile a query that may be Korean NL, Everything operators, or a mix.
/// `지난주 세금 pdf` → ~7-day window + ext pdf + token 세금.
pub fn compile_nl(input: &str, now: SystemTime) -> Query {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Query::default();
    }

    // Structured operators always go through the query language.
    if looks_structured(trimmed) {
        return parse_query(trimmed, now);
    }

    let tokens = tokenize(trimmed);
    let mut q = Query::default();
    let mut skip_next_ju = false;
    for (i, tok) in tokens.iter().enumerate() {
        if skip_next_ju {
            skip_next_ju = false;
            continue;
        }
        let lower = tok.to_lowercase();
        if lower == "지난주" || lower == "지난주일" {
            q.modified_after = now.checked_sub(Duration::from_secs(7 * 24 * 3600));
            continue;
        }
        if lower == "지난" {
            if tokens.get(i + 1).map(|s| s.as_str()) == Some("주") {
                q.modified_after = now.checked_sub(Duration::from_secs(7 * 24 * 3600));
                skip_next_ju = true;
                continue;
            }
        }
        if lower == "오늘" {
            q.modified_after = now.checked_sub(Duration::from_secs(24 * 3600));
            continue;
        }
        if lower == "어제" {
            q.modified_after = now.checked_sub(Duration::from_secs(48 * 3600));
            q.modified_before = now.checked_sub(Duration::from_secs(24 * 3600));
            continue;
        }
        if let Some(rest) = strip_prefix_ci(tok, "ext:") {
            q.extensions = rest
                .split(';')
                .map(|s| s.trim().trim_start_matches('.').to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            continue;
        }
        if strip_prefix_ci(tok, "size:").is_some() || strip_prefix_ci(tok, "dm:").is_some() {
            let parsed = parse_query(tok, now);
            if q.size.is_none() {
                q.size = parsed.size;
            }
            if q.modified_after.is_none() {
                q.modified_after = parsed.modified_after;
            }
            if q.modified_before.is_none() {
                q.modified_before = parsed.modified_before;
            }
            continue;
        }
        let ext_tok = tok.trim_start_matches('.').to_ascii_lowercase();
        if KNOWN_EXT.contains(&ext_tok.as_str()) {
            if !q.extensions.contains(&ext_tok) {
                q.extensions.push(ext_tok);
            }
            continue;
        }
        q.must.push(Atom::Term(Pattern::new(tok.clone())));
    }
    q
}

fn looks_structured(s: &str) -> bool {
    s.contains('|')
        || s.contains('!')
        || s.to_ascii_lowercase().contains("ext:")
        || s.to_ascii_lowercase().contains("size:")
        || s.to_ascii_lowercase().contains("dm:")
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() < prefix.len() || !s.is_char_boundary(prefix.len()) {
        return None;
    }
    if s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_week_tax_pdf() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let q = compile_nl("지난주 세금 pdf", now);
        assert_eq!(q.extensions, vec!["pdf".to_string()]);
        assert!(q.modified_after.is_some());
        assert_eq!(q.must.len(), 1);
        match &q.must[0] {
            Atom::Term(p) => assert_eq!(p.raw, "세금"),
            _ => panic!("expected term"),
        }
        let window = now.duration_since(q.modified_after.unwrap()).unwrap();
        assert_eq!(window.as_secs(), 7 * 24 * 3600);
    }

    #[test]
    fn passthrough_ext_dm_size() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let q = compile_nl("invoice ext:docx size:>10kb dm:today", now);
        assert_eq!(q.extensions, vec!["docx".to_string()]);
        assert!(q.size.is_some());
        assert!(q.modified_after.is_some());
    }
}
