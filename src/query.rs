use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub raw: String,
    pub glob: bool,
    pub regex: bool,
}

impl Pattern {
    pub fn new(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let glob = raw.contains('*') || raw.contains('?');
        Pattern {
            raw,
            glob,
            regex: false,
        }
    }

    pub fn regex(raw: impl Into<String>) -> Self {
        Pattern {
            raw: raw.into(),
            glob: false,
            regex: true,
        }
    }

    pub fn matches(&self, text: &str) -> bool {
        self.matches_with(text, false, false)
    }

    pub fn matches_with(&self, text: &str, case_sensitive: bool, whole_word: bool) -> bool {
        if self.regex {
            regex_match(&self.raw, text, case_sensitive)
        } else if self.glob {
            glob_match(&self.raw, text, case_sensitive)
        } else {
            contains_match(text, &self.raw, case_sensitive, whole_word)
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
        self.matches_text_with(text, false, false)
    }

    pub fn matches_text_with(&self, text: &str, case_sensitive: bool, whole_word: bool) -> bool {
        match self {
            Atom::Term(p) => p.matches_with(text, case_sensitive, whole_word),
            Atom::Or(ps) => ps.iter().any(|p| p.matches_with(text, case_sensitive, whole_word)),
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

/// Result order after hits are collected. `Score` keeps ranking; the others replace it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Score,
    Size,
    Date,
    Name,
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
    /// `case:` — match terms without lowercasing.
    pub case_sensitive: bool,
    /// `ww:` / `wholeword:` — match terms only at non-alphanumeric bounds.
    pub whole_word: bool,
    /// `attrib:RH` — Windows `FILE_ATTRIBUTE_*` bits that must all be set.
    pub attrib_mask: u32,
    /// `attrib:R|H` — Windows `FILE_ATTRIBUTE_*` bits of which any may be set.
    pub attrib_any: u32,
    /// `parent:` — `FileRecord.path`'s parent equals this path.
    pub parent: Option<String>,
    /// `path:` — the full path contains this token (filter, not a must-term).
    pub path_contains: Option<String>,
    /// `sort:size` / `sort:date` / `sort:name` — replace score ranking.
    pub sort: SortMode,
}

impl Query {
    pub fn term_strings(&self) -> Vec<String> {
        self.must
            .iter()
            .flat_map(|a| match a {
                Atom::Term(p) => vec![p],
                Atom::Or(ps) => ps.iter().collect(),
            })
            .filter(|p| !p.glob && !p.regex)
            .map(|p| p.raw.clone())
            .collect()
    }
}

/// Everything-class operators: space=AND, `|=OR`, `!=NOT`, `*`/`?`,
/// `ext:`, `size:`, `dm:`, `n:`/`name:`, `file:`, `folder:`, `case:`,
/// `ww:`/`wholeword:`, `regex:`/`r:`, `attrib:R`/`H`/`D`, `parent:`, `path:`,
/// `sort:size`/`sort:date`/`sort:name`, `"quoted phrase"`.
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
        } else if let Some(rest) = strip_prefix_ci(t, "case:") {
            q.case_sensitive = true;
            if !rest.is_empty() {
                push_must(&mut q, rest);
            }
        } else if let Some(rest) = strip_ww_prefix(t) {
            q.whole_word = true;
            if !rest.is_empty() {
                push_must(&mut q, rest);
            }
        } else if let Some(rest) = strip_regex_prefix(t) {
            if !rest.is_empty() {
                q.must.push(Atom::Term(Pattern::regex(rest)));
            }
        } else if let Some(rest) = strip_prefix_ci(t, "attrib:") {
            apply_attrib(&mut q, rest);
        } else if let Some(rest) = strip_prefix_ci(t, "parent:") {
            if !rest.is_empty() {
                q.parent = Some(rest.to_string());
            }
        } else if let Some(rest) = strip_prefix_ci(t, "path:") {
            if !rest.is_empty() {
                q.path_contains = Some(rest.to_string());
            }
        } else if let Some(rest) = strip_prefix_ci(t, "sort:") {
            if let Some(mode) = parse_sort(rest) {
                q.sort = mode;
            }
        } else if t == "|" {
            let next = tokens.get(i + 1).cloned();
            if let (Some(Atom::Term(prev)), Some(n)) = (q.must.pop(), next) {
                if n != "|" && !n.starts_with('!') && !is_filter(&n) {
                    q.must.push(Atom::Or(vec![prev, parse_pattern(&n)]));
                    i += 1;
                } else {
                    q.must.push(Atom::Term(prev));
                }
            }
        } else if t == "!" {
            if let Some(n) = tokens.get(i + 1) {
                if !is_filter(n) {
                    q.must_not.push(parse_pattern(n));
                    i += 1;
                }
            }
        } else if let Some(rest) = t.strip_prefix('!') {
            if !rest.is_empty() {
                q.must_not.push(parse_pattern(rest));
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
            .map(parse_pattern)
            .collect();
        if parts.len() == 1 {
            q.must.push(Atom::Term(parts.into_iter().next().unwrap()));
        } else if !parts.is_empty() {
            q.must.push(Atom::Or(parts));
        }
    } else {
        q.must.push(Atom::Term(parse_pattern(t)));
    }
}

fn parse_pattern(t: &str) -> Pattern {
    if let Some(rest) = strip_regex_prefix(t) {
        Pattern::regex(rest)
    } else {
        Pattern::new(t)
    }
}

fn strip_name_prefix(t: &str) -> Option<&str> {
    strip_prefix_ci(t, "name:").or_else(|| strip_prefix_ci(t, "n:"))
}

fn strip_ww_prefix(t: &str) -> Option<&str> {
    strip_prefix_ci(t, "wholeword:").or_else(|| strip_prefix_ci(t, "ww:"))
}

fn strip_regex_prefix(t: &str) -> Option<&str> {
    strip_prefix_ci(t, "regex:").or_else(|| strip_prefix_ci(t, "r:"))
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
        || l.starts_with("case:")
        || l.starts_with("ww:")
        || l.starts_with("wholeword:")
        || l.starts_with("attrib:")
        || l.starts_with("parent:")
        || l.starts_with("path:")
        || l.starts_with("sort:")
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

fn parse_sort(spec: &str) -> Option<SortMode> {
    match spec.trim().to_ascii_lowercase().as_str() {
        "size" => Some(SortMode::Size),
        "date" => Some(SortMode::Date),
        "name" => Some(SortMode::Name),
        "score" => Some(SortMode::Score),
        _ => None,
    }
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

fn apply_attrib(q: &mut Query, spec: &str) {
    if spec.contains('|') {
        for part in spec.split('|') {
            q.attrib_any |= parse_attrib_letters(part);
        }
    } else {
        q.attrib_mask |= parse_attrib_letters(spec);
    }
}

fn parse_attrib_letters(spec: &str) -> u32 {
    use crate::types::{ATTR_DIRECTORY, ATTR_HIDDEN, ATTR_READONLY, ATTR_SYSTEM};
    let mut bits = 0u32;
    for c in spec.chars() {
        match c.to_ascii_uppercase() {
            'R' => bits |= ATTR_READONLY,
            'H' => bits |= ATTR_HIDDEN,
            'S' => bits |= ATTR_SYSTEM,
            'D' => bits |= ATTR_DIRECTORY,
            _ => {}
        }
    }
    bits
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

fn contains_match(hay: &str, needle: &str, case_sensitive: bool, whole_word: bool) -> bool {
    if needle.is_empty() {
        return true;
    }
    if !whole_word {
        return if case_sensitive {
            hay.contains(needle)
        } else {
            contains_ci(hay, needle)
        };
    }
    whole_word_match(hay, needle, case_sensitive)
}

fn whole_word_match(hay: &str, needle: &str, case_sensitive: bool) -> bool {
    let hay_chars: Vec<char> = hay.chars().collect();
    let needle_chars: Vec<char> = needle.chars().collect();
    let nlen = needle_chars.len();
    if nlen == 0 {
        return true;
    }
    if hay_chars.len() < nlen {
        return false;
    }
    for i in 0..=hay_chars.len() - nlen {
        let window = &hay_chars[i..i + nlen];
        let eq = if case_sensitive {
            window == needle_chars.as_slice()
        } else {
            window
                .iter()
                .zip(needle_chars.iter())
                .all(|(a, b)| eq_ci(*a, *b))
        };
        if !eq {
            continue;
        }
        let left_ok = i == 0 || !is_word_char(hay_chars[i - 1]);
        let right_ok = i + nlen == hay_chars.len() || !is_word_char(hay_chars[i + nlen]);
        if left_ok && right_ok {
            return true;
        }
    }
    false
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || is_hangul_syllable(c)
}

fn is_hangul_syllable(c: char) -> bool {
    matches!(c as u32, 0xAC00..=0xD7A3)
}

fn glob_match(pat: &str, text: &str, case_sensitive: bool) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = text.chars().collect();
    glob_rec(&p, &t, case_sensitive)
}

fn glob_rec(pat: &[char], text: &[char], case_sensitive: bool) -> bool {
    match (pat.first().copied(), text.first().copied()) {
        (None, None) => true,
        (Some('*'), _) => {
            glob_rec(&pat[1..], text, case_sensitive)
                || (!text.is_empty() && glob_rec(pat, &text[1..], case_sensitive))
        }
        (Some('?'), Some(_)) => glob_rec(&pat[1..], &text[1..], case_sensitive),
        (Some(pc), Some(tc)) if chars_eq(pc, tc, case_sensitive) => {
            glob_rec(&pat[1..], &text[1..], case_sensitive)
        }
        _ => false,
    }
}

#[derive(Clone, Copy)]
enum ReOp {
    Any,
    Lit(char),
    StarAny,
    StarLit(char),
    Start,
    End,
}

fn regex_match(pat: &str, text: &str, case_sensitive: bool) -> bool {
    let ops = compile_re(pat);
    let t: Vec<char> = text.chars().collect();
    for i in 0..=t.len() {
        if re_here(&ops, &t, i, case_sensitive) {
            return true;
        }
    }
    false
}

fn compile_re(pat: &str) -> Vec<ReOp> {
    let mut ops = Vec::new();
    for c in pat.chars() {
        if c == '*' {
            match ops.pop() {
                Some(ReOp::Lit(ch)) => ops.push(ReOp::StarLit(ch)),
                Some(ReOp::Any) => ops.push(ReOp::StarAny),
                Some(prev) => ops.push(prev),
                None => {}
            }
            continue;
        }
        ops.push(match c {
            '.' => ReOp::Any,
            '^' => ReOp::Start,
            '$' => ReOp::End,
            other => ReOp::Lit(other),
        });
    }
    ops
}

fn re_here(ops: &[ReOp], text: &[char], pos: usize, case_sensitive: bool) -> bool {
    match ops.first().copied() {
        None => true,
        Some(ReOp::Start) => pos == 0 && re_here(&ops[1..], text, pos, case_sensitive),
        Some(ReOp::End) => pos == text.len() && re_here(&ops[1..], text, pos, case_sensitive),
        Some(ReOp::Any) => {
            pos < text.len() && re_here(&ops[1..], text, pos + 1, case_sensitive)
        }
        Some(ReOp::Lit(c)) => {
            pos < text.len()
                && chars_eq(c, text[pos], case_sensitive)
                && re_here(&ops[1..], text, pos + 1, case_sensitive)
        }
        Some(ReOp::StarAny) => {
            re_here(&ops[1..], text, pos, case_sensitive)
                || (pos < text.len() && re_here(ops, text, pos + 1, case_sensitive))
        }
        Some(ReOp::StarLit(c)) => {
            re_here(&ops[1..], text, pos, case_sensitive)
                || (pos < text.len()
                    && chars_eq(c, text[pos], case_sensitive)
                    && re_here(ops, text, pos + 1, case_sensitive))
        }
    }
}

fn chars_eq(a: char, b: char, case_sensitive: bool) -> bool {
    if case_sensitive {
        a == b
    } else {
        eq_ci(a, b)
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

    #[test]
    fn case_and_whole_word_switches() {
        let c = parse_query("case:", now());
        assert!(c.case_sensitive);
        assert!(!c.whole_word);
        assert!(c.must.is_empty());

        let attached = parse_query("case:Foo", now());
        assert!(attached.case_sensitive);
        match &attached.must[0] {
            Atom::Term(p) => assert_eq!(p.raw, "Foo"),
            _ => panic!("expected term"),
        }

        let ww = parse_query("ww:", now());
        assert!(ww.whole_word);
        assert!(!ww.case_sensitive);

        let whole = parse_query("wholeword:bar", now());
        assert!(whole.whole_word);
        match &whole.must[0] {
            Atom::Term(p) => assert_eq!(p.raw, "bar"),
            _ => panic!("expected term"),
        }
    }

    #[test]
    fn pattern_honors_case_and_whole_word() {
        let p = Pattern::new("Foo");
        assert!(p.matches("xxfooYY"));
        assert!(p.matches_with("xxFooYY", true, false));
        assert!(!p.matches_with("xxfooYY", true, false));

        let p = Pattern::new("report");
        assert!(p.matches("reporting.txt"));
        assert!(p.matches_with("alpha-report.txt", false, true));
        assert!(p.matches_with("report.txt", false, true));
        assert!(!p.matches_with("reporting.txt", false, true));
        assert!(!p.matches_with("myreport.txt", false, true));

        let h = Pattern::new("한글");
        assert!(h.matches("한글파일.txt"));
        assert!(h.matches_with("한글.txt", false, true));
        assert!(h.matches_with("다른 한글 메모.txt", false, true));
        assert!(!h.matches_with("한글파일.txt", false, true));
    }

    #[test]
    fn regex_prefix_compiles_and_matches() {
        for input in ["regex:foo.*bar", "r:foo.*bar", "REGEX:foo.*bar", "R:foo.*bar"] {
            let q = parse_query(input, now());
            assert_eq!(q.must.len(), 1, "{input}");
            match &q.must[0] {
                Atom::Term(p) => {
                    assert_eq!(p.raw, "foo.*bar", "{input}");
                    assert!(p.regex, "{input}");
                    assert!(!p.glob, "{input}");
                }
                other => panic!("{input}: expected Term, got {other:?}"),
            }
        }

        let p = Pattern::regex("foo.*bar");
        assert!(p.matches("fooXYZbar"));
        assert!(p.matches("xxFOOyyBARzz"));
        assert!(!p.matches("foo"));
        assert!(!p.matches("barfoo"));

        let p = Pattern::regex("^foo");
        assert!(p.matches("foo.txt"));
        assert!(!p.matches("xfoo.txt"));

        let p = Pattern::regex("txt$");
        assert!(p.matches("foo.txt"));
        assert!(!p.matches("txt.log"));

        let p = Pattern::regex("a.c");
        assert!(p.matches("abc"));
        assert!(p.matches("aXc"));
        assert!(!p.matches("ac"));
        assert!(!p.matches("abbc"));

        let p = Pattern::regex("ab*c");
        assert!(p.matches("ac"));
        assert!(p.matches("abc"));
        assert!(p.matches("abbbc"));
        assert!(!p.matches("abXc"));

        let p = Pattern::regex("Foo.*Bar");
        assert!(p.matches("fooZZZbar"));
        assert!(p.matches_with("FooZZZBar", true, false));
        assert!(!p.matches_with("fooZZZbar", true, false));
    }

    #[test]
    fn attrib_tokens_set_mask_and_any() {
        use crate::types::{ATTR_DIRECTORY, ATTR_HIDDEN, ATTR_READONLY};

        let r = parse_query("attrib:R", now());
        assert_eq!(r.attrib_mask, ATTR_READONLY);
        assert_eq!(r.attrib_any, 0);
        assert!(r.must.is_empty());

        let h = parse_query("attrib:H", now());
        assert_eq!(h.attrib_mask, ATTR_HIDDEN);

        let d = parse_query("attrib:D", now());
        assert_eq!(d.attrib_mask, ATTR_DIRECTORY);

        let ci = parse_query("ATTRIB:rh", now());
        assert_eq!(ci.attrib_mask, ATTR_READONLY | ATTR_HIDDEN);

        let both = parse_query("attrib:R attrib:H", now());
        assert_eq!(both.attrib_mask, ATTR_READONLY | ATTR_HIDDEN);
        assert_eq!(both.attrib_any, 0);

        let any = parse_query("attrib:R|H", now());
        assert_eq!(any.attrib_any, ATTR_READONLY | ATTR_HIDDEN);
        assert_eq!(any.attrib_mask, 0);

        let bare = parse_query("attrib:", now());
        assert_eq!(bare.attrib_mask, 0);
        assert_eq!(bare.attrib_any, 0);
    }

    #[test]
    fn parent_and_path_tokens_are_filters() {
        let p = parse_query(r"parent:C:\docs", now());
        assert_eq!(p.parent.as_deref(), Some(r"C:\docs"));
        assert!(p.must.is_empty());
        assert!(p.path_contains.is_none());

        let path = parse_query("path:docs", now());
        assert_eq!(path.path_contains.as_deref(), Some("docs"));
        assert!(path.must.is_empty());
        assert!(path.parent.is_none());

        let ci = parse_query(r"PARENT:C:\docs PATH:Docs", now());
        assert_eq!(ci.parent.as_deref(), Some(r"C:\docs"));
        assert_eq!(ci.path_contains.as_deref(), Some("Docs"));

        let mixed = parse_query(r"report parent:C:\docs", now());
        assert_eq!(mixed.parent.as_deref(), Some(r"C:\docs"));
        match &mixed.must[0] {
            Atom::Term(pat) => assert_eq!(pat.raw, "report"),
            _ => panic!("expected term"),
        }

        let quoted = parse_query(r#"parent:"C:\Program Files""#, now());
        assert_eq!(quoted.parent.as_deref(), Some(r"C:\Program Files"));
        assert!(quoted.must.is_empty());

        let bare = parse_query("parent: path:", now());
        assert!(bare.parent.is_none());
        assert!(bare.path_contains.is_none());
        assert!(bare.must.is_empty());
    }

    #[test]
    fn sort_tokens_set_mode_and_are_filters() {
        assert_eq!(parse_query("report", now()).sort, SortMode::Score);

        let size = parse_query("sort:size", now());
        assert_eq!(size.sort, SortMode::Size);
        assert!(size.must.is_empty());

        let date = parse_query("SORT:DATE", now());
        assert_eq!(date.sort, SortMode::Date);
        assert!(date.must.is_empty());

        let name = parse_query("sort:name", now());
        assert_eq!(name.sort, SortMode::Name);
        assert!(name.must.is_empty());

        let mixed = parse_query("report sort:size", now());
        assert_eq!(mixed.sort, SortMode::Size);
        match &mixed.must[0] {
            Atom::Term(p) => assert_eq!(p.raw, "report"),
            _ => panic!("expected term"),
        }

        let last = parse_query("sort:size sort:date", now());
        assert_eq!(last.sort, SortMode::Date);
    }
}
