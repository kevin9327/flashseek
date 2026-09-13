use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub raw: String,
    pub glob: bool,
}

impl Pattern {
    pub fn new(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let glob = raw.contains('*') || raw.contains('?');
        Pattern { raw, glob }
    }

    pub fn matches(&self, text: &str) -> bool {
        if self.glob {
            glob_ci(&self.raw, text)
        } else {
            contains_ci(text, &self.raw)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Term(Pattern),
    Or(Vec<Pattern>),
}

impl Atom {
    pub fn matches_text(&self, text: &str) -> bool {
        match self {
            Atom::Term(p) => p.matches(text),
            Atom::Or(ps) => ps.iter().any(|p| p.matches(text)),
        }
    }

    pub fn terms(&self) -> Vec<&str> {
        match self {
            Atom::Term(p) => vec![p.raw.as_str()],
            Atom::Or(ps) => ps.iter().map(|p| p.raw.as_str()).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeFilter {
    Gt(u64),
    Ge(u64),
    Lt(u64),
    Le(u64),
    Eq(u64),
}

impl SizeFilter {
    pub fn matches(&self, size: u64) -> bool {
        match *self {
            SizeFilter::Gt(n) => size > n,
            SizeFilter::Ge(n) => size >= n,
            SizeFilter::Lt(n) => size < n,
            SizeFilter::Le(n) => size <= n,
            SizeFilter::Eq(n) => size == n,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Query {
    pub must: Vec<Atom>,
    pub must_not: Vec<Pattern>,
    pub extensions: Vec<String>,
    pub size: Option<SizeFilter>,
    pub modified_after: Option<SystemTime>,
    pub modified_before: Option<SystemTime>,
    /// `n:` / `name:` — atoms may match the basename only, not path or body.
    pub name_only: bool,
    /// `file:` — exclude directories.
    pub files_only: bool,
    /// `folder:` — exclude files.
    pub folders_only: bool,
}

impl Query {
    pub fn term_strings(&self) -> Vec<String> {
        self.must
            .iter()
            .flat_map(|a| a.terms().into_iter().map(|s| s.to_string()))
            .filter(|s| !s.contains('*') && !s.contains('?'))
            .collect()
    }
}

/// Everything-class operators: space=AND, `|=OR`, `!=NOT`, `*`/`?`,
/// `ext:`, `size:`, `dm:`, `n:`/`name:`, `file:`, `folder:`, `"quoted phrase"`.
pub fn parse_query(input: &str, now: SystemTime) -> Query {
    let tokens = tokenize(input);
    let mut q = Query::default();
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if let Some(rest) = strip_prefix_ci(t, "ext:") {
            q.extensions = rest
                .split(';')
                .map(|s| s.trim().trim_start_matches('.').to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(rest) = strip_prefix_ci(t, "size:") {
            if let Some(f) = parse_size(rest) {
                q.size = Some(f);
            }
        } else if let Some(rest) = strip_prefix_ci(t, "dm:") {
            apply_dm(&mut q, rest, now);
        } else if let Some(rest) = strip_name_prefix(t) {
            q.name_only = true;
            if !rest.is_empty() {
                push_must(&mut q, rest);
            }
        } else if let Some(rest) = strip_prefix_ci(t, "file:") {
            q.files_only = true;
            if !rest.is_empty() {
                push_must(&mut q, rest);
            }
        } else if let Some(rest) = strip_prefix_ci(t, "folder:") {
            q.folders_only = true;
            if !rest.is_empty() {
                push_must(&mut q, rest);
            }
        } else if t == "|" {
            let next = tokens.get(i + 1).cloned();
            if let (Some(Atom::Term(prev)), Some(n)) = (q.must.pop(), next) {
                if n != "|" && !n.starts_with('!') && !is_filter(&n) {
                    q.must.push(Atom::Or(vec![prev, Pattern::new(n)]));
                    i += 1;
                } else {
                    q.must.push(Atom::Term(prev));
                }
            }
        } else if t == "!" {
            if let Some(n) = tokens.get(i + 1) {
                if !is_filter(n) {
                    q.must_not.push(Pattern::new(n.clone()));
                    i += 1;
                }
            }
        } else if let Some(rest) = t.strip_prefix('!') {
            if !rest.is_empty() {
                q.must_not.push(Pattern::new(rest));
            }
        } else {
            push_must(&mut q, t);
        }
        i += 1;
    }
    q
}

fn push_must(q: &mut Query, t: &str) {
    if t.contains('|') {
        let parts: Vec<Pattern> = t
            .split('|')
            .filter(|s| !s.is_empty())
            .map(Pattern::new)
            .collect();
        if parts.len() == 1 {
            q.must.push(Atom::Term(parts.into_iter().next().unwrap()));
        } else if !parts.is_empty() {
            q.must.push(Atom::Or(parts));
        }
    } else {
        q.must.push(Atom::Term(Pattern::new(t)));
    }
}

fn strip_name_prefix(t: &str) -> Option<&str> {
    strip_prefix_ci(t, "name:").or_else(|| strip_prefix_ci(t, "n:"))
}

fn is_filter(t: &str) -> bool {
    let l = t.to_ascii_lowercase();
    l.starts_with("ext:")
        || l.starts_with("size:")
        || l.starts_with("dm:")
        || l.starts_with("name:")
        || l.starts_with("n:")
        || l.starts_with("file:")
        || l.starts_with("folder:")
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

pub fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for c in s.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse_size(spec: &str) -> Option<SizeFilter> {
    let spec = spec.trim();
    let (kind, rest) = if let Some(r) = spec.strip_prefix(">=") {
        ('G', r)
    } else if let Some(r) = spec.strip_prefix("<=") {
        ('L', r)
    } else if let Some(r) = spec.strip_prefix('>') {
        ('g', r)
    } else if let Some(r) = spec.strip_prefix('<') {
        ('l', r)
    } else if let Some(r) = spec.strip_prefix('=') {
        ('e', r)
    } else {
        ('e', spec)
    };
    let bytes = parse_bytes(rest)?;
    Some(match kind {
        'G' => SizeFilter::Ge(bytes),
        'L' => SizeFilter::Le(bytes),
        'g' => SizeFilter::Gt(bytes),
        'l' => SizeFilter::Lt(bytes),
        _ => SizeFilter::Eq(bytes),
    })
}

fn parse_bytes(s: &str) -> Option<u64> {
    let s = s.trim().to_ascii_lowercase();
    let (num, mul) = if let Some(n) = s.strip_suffix("gb") {
        (n, 1024u64 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("mb") {
        (n, 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("kb") {
        (n, 1024)
    } else if let Some(n) = s.strip_suffix('b') {
        (n, 1)
    } else {
        (s.as_str(), 1)
    };
    let n: f64 = num.trim().parse().ok()?;
    Some((n * mul as f64) as u64)
}

fn apply_dm(q: &mut Query, spec: &str, now: SystemTime) {
    let spec = spec.trim().to_ascii_lowercase();
    match spec.as_str() {
        "today" => q.modified_after = now.checked_sub(Duration::from_secs(24 * 3600)),
        "yesterday" => {
            q.modified_after = now.checked_sub(Duration::from_secs(48 * 3600));
            q.modified_before = now.checked_sub(Duration::from_secs(24 * 3600));
        }
        "lastweek" | "thisweek" => {
            q.modified_after = now.checked_sub(Duration::from_secs(7 * 24 * 3600));
        }
        other => {
            if let Some(rest) = other.strip_prefix('>') {
                if let Some(t) = parse_ymd(rest) {
                    q.modified_after = Some(t);
                }
            } else if let Some(rest) = other.strip_prefix('<') {
                if let Some(t) = parse_ymd(rest) {
                    q.modified_before = Some(t);
                }
            } else if let Some(t) = parse_ymd(other) {
                q.modified_after = Some(t);
            }
        }
    }
}

fn parse_ymd(s: &str) -> Option<SystemTime> {
    let s = s.trim();
    let mut parts = s.split('-');
    let y: u64 = parts.next()?.parse().ok()?;
    let m: u64 = parts.next()?.parse().ok()?;
    let d: u64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || y < 1970 {
        return None;
    }
    let mut days: u64 = 0;
    for yy in 1970..y {
        days += if is_leap(yy) { 366 } else { 365 };
    }
    const MD: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for mm in 1..m {
        days += MD[(mm - 1) as usize];
        if mm == 2 && is_leap(y) {
            days += 1;
        }
    }
    days += d.saturating_sub(1);
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(days * 86400))
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn contains_ci(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    hay.to_lowercase().contains(&needle.to_lowercase())
}

fn glob_ci(pat: &str, text: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = text.chars().collect();
    glob_rec(&p, &t)
}

fn glob_rec(pat: &[char], text: &[char]) -> bool {
    match (pat.first().copied(), text.first().copied()) {
        (None, None) => true,
        (Some('*'), _) => {
            glob_rec(&pat[1..], text) || (!text.is_empty() && glob_rec(pat, &text[1..]))
        }
        (Some('?'), Some(_)) => glob_rec(&pat[1..], &text[1..]),
        (Some(pc), Some(tc)) if eq_ci(pc, tc) => glob_rec(&pat[1..], &text[1..]),
        _ => false,
    }
}

fn eq_ci(a: char, b: char) -> bool {
    a.eq_ignore_ascii_case(&b) || a.to_lowercase().eq(b.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
    }

    #[test]
    fn space_is_and_pipe_is_or() {
        let q = parse_query("foo bar|baz", now());
        assert_eq!(q.must.len(), 2);
        match &q.must[1] {
            Atom::Or(ps) => assert_eq!(ps.len(), 2),
            _ => panic!("expected OR"),
        }
    }

    #[test]
    fn not_and_ext_and_size() {
        let q = parse_query("report !tmp ext:pdf size:>1mb", now());
        assert_eq!(q.extensions, vec!["pdf".to_string()]);
        assert!(matches!(q.size, Some(SizeFilter::Gt(_))));
        assert_eq!(q.must_not.len(), 1);
    }

    #[test]
    fn quoted_phrase_is_one_term() {
        let q = parse_query("\"foo bar\" baz", now());
        assert_eq!(q.must.len(), 2);
        match &q.must[0] {
            Atom::Term(p) => assert_eq!(p.raw, "foo bar"),
            _ => panic!("expected quoted phrase as one term"),
        }
        match &q.must[1] {
            Atom::Term(p) => assert_eq!(p.raw, "baz"),
            _ => panic!("expected trailing term"),
        }
    }

    #[test]
    fn name_file_folder_modifiers() {
        let n = parse_query("n:alpha", now());
        assert!(n.name_only);
        assert!(!n.files_only && !n.folders_only);
        match &n.must[0] {
            Atom::Term(p) => assert_eq!(p.raw, "alpha"),
            _ => panic!("expected term"),
        }

        let name = parse_query("name:Alpha", now());
        assert!(name.name_only);
        match &name.must[0] {
            Atom::Term(p) => assert_eq!(p.raw, "Alpha"),
            _ => panic!("expected term"),
        }

        let f = parse_query("file:", now());
        assert!(f.files_only);
        assert!(!f.folders_only);
        assert!(f.must.is_empty());

        let attached = parse_query("file:report", now());
        assert!(attached.files_only);
        match &attached.must[0] {
            Atom::Term(p) => assert_eq!(p.raw, "report"),
            _ => panic!("expected term"),
        }

        let d = parse_query("folder:", now());
        assert!(d.folders_only);
        assert!(!d.files_only);
    }
}
