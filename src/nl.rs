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
    let mut skip_next = false;
    for (i, tok) in tokens.iter().enumerate() {
        if skip_next {
            skip_next = false;
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
                skip_next = true;
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
        if lower == "이번달" {
            q.modified_after = now.checked_sub(Duration::from_secs(30 * 24 * 3600));
            continue;
        }
        if lower == "이번" {
            if tokens.get(i + 1).map(|s| s.as_str()) == Some("달") {
                q.modified_after = now.checked_sub(Duration::from_secs(30 * 24 * 3600));
                skip_next = true;
                continue;
            }
        }
        if lower == "올해" {
            q.modified_after = now.checked_sub(Duration::from_secs(365 * 24 * 3600));
            continue;
        }
        if lower == "최근" || lower == "일주일" {
            q.modified_after = now.checked_sub(Duration::from_secs(7 * 24 * 3600));
            continue;
        }
        if lower == "사진" || lower == "이미지" {
            push_ext(&mut q, "png");
            push_ext(&mut q, "jpg");
            push_ext(&mut q, "jpeg");
            continue;
        }
        if lower == "동영상" || lower == "영상" {
            push_ext(&mut q, "mp4");
            push_ext(&mut q, "mkv");
            push_ext(&mut q, "webm");
            continue;
        }
        if lower == "음악" {
            push_ext(&mut q, "mp3");
            push_ext(&mut q, "wav");
            push_ext(&mut q, "flac");
            continue;
        }
        if lower == "문서" {
            // Hangul type word: documents → docx + pdf.
            push_ext(&mut q, "docx");
            push_ext(&mut q, "pdf");
            continue;
        }
        if lower == "스프레드시트" {
            push_ext(&mut q, "xlsx");
            continue;
        }
        if lower == "슬라이드" {
            push_ext(&mut q, "pptx");
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
            push_ext(&mut q, &ext_tok);
            continue;
        }
        q.must.push(Atom::Term(Pattern::new(tok.clone())));
    }
    q
}

fn push_ext(q: &mut Query, ext: &str) {
    if !q.extensions.iter().any(|e| e == ext) {
        q.extensions.push(ext.to_string());
    }
}

fn looks_structured(s: &str) -> bool {
    s.contains('|')
        || s.contains('!')
        || s.to_ascii_lowercase().contains("ext:")
        || s.to_ascii_lowercase().contains("size:")
        || s.to_ascii_lowercase().contains("dm:")
        || s.to_ascii_lowercase().contains("attrib:")
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

    #[test]
    fn korean_date_windows_and_type_words() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);

        let today = compile_nl("오늘", now);
        assert_eq!(
            now.duration_since(today.modified_after.unwrap())
                .unwrap()
                .as_secs(),
            24 * 3600
        );
        assert!(today.modified_before.is_none());

        let yest = compile_nl("어제", now);
        assert_eq!(
            now.duration_since(yest.modified_after.unwrap())
                .unwrap()
                .as_secs(),
            48 * 3600
        );
        assert_eq!(
            now.duration_since(yest.modified_before.unwrap())
                .unwrap()
                .as_secs(),
            24 * 3600
        );

        for input in ["이번달", "이번 달"] {
            let q = compile_nl(input, now);
            assert_eq!(
                now.duration_since(q.modified_after.unwrap())
                    .unwrap()
                    .as_secs(),
                30 * 24 * 3600,
                "{input}"
            );
            assert!(q.must.is_empty(), "{input} must not keep 달 as a term");
        }

        let year = compile_nl("올해", now);
        assert_eq!(
            now.duration_since(year.modified_after.unwrap())
                .unwrap()
                .as_secs(),
            365 * 24 * 3600
        );

        let docs = compile_nl("문서", now);
        assert_eq!(docs.extensions, vec!["docx".to_string(), "pdf".to_string()]);
        assert!(docs.must.is_empty());

        let sheet = compile_nl("스프레드시트", now);
        assert_eq!(sheet.extensions, vec!["xlsx".to_string()]);

        let slides = compile_nl("슬라이드", now);
        assert_eq!(slides.extensions, vec!["pptx".to_string()]);
    }
}
