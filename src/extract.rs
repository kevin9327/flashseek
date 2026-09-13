use std::fs;
use std::io::{self, Read};
use std::path::Path;

const EXTRACTABLE: &[&str] = &[
    "txt", "md", "html", "htm", "xml", "pdf", "docx", "xlsx", "pptx", "csv", "json", "log",
    "ini", "cfg", "toml", "yaml", "yml", "rs", "py", "js", "ts", "tsx", "jsx", "go", "java",
    "kt", "c", "h", "cpp", "cs", "rb", "php", "sh", "ps1", "rtf", "svg",
];

pub fn is_extractable(path: &Path) -> bool {
    ext_of(path)
        .map(|e| EXTRACTABLE.contains(&e.as_str()))
        .unwrap_or(false)
}

pub fn ext_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
}

/// Skip a UTF-8 BOM (`EF BB BF`) so the first body token is searchable.
fn skip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

fn read_plain_text(path: &Path) -> io::Result<String> {
    let bytes = fs::read(path)?;
    decode_plain_bytes(&bytes)
}

pub fn decode_plain_bytes(bytes: &[u8]) -> io::Result<String> {
    if bytes.starts_with(&[0xFF, 0xFE]) && bytes.len() >= 2 {
        let u16s: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return Ok(String::from_utf16_lossy(&u16s));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) && bytes.len() >= 2 {
        let u16s: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return Ok(String::from_utf16_lossy(&u16s));
    }
    String::from_utf8(skip_utf8_bom(bytes).to_vec())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn extract_text(path: &Path) -> io::Result<String> {
    let ext = ext_of(path).unwrap_or_default();
    match ext.as_str() {
        "txt" | "md" | "csv" | "json" | "log" | "ini" | "cfg" | "toml" | "yaml" | "yml" | "rs"
        | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java" | "kt" | "c" | "h" | "cpp" | "cs"
        | "rb" | "php" | "sh" | "ps1" => read_plain_text(path),
        "html" | "htm" | "xml" | "svg" => Ok(strip_html(&fs::read_to_string(path)?)),
        "rtf" => Ok(extract_rtf(&fs::read_to_string(path)?)),
        "pdf" => Ok(extract_pdf(&fs::read(path)?)),
        "docx" => extract_office(path, |n| n == "word/document.xml" || n.ends_with("/document.xml")),
        "xlsx" => extract_office(path, |n| {
            n.contains("sharedStrings") || n.contains("/worksheets/sheet")
        }),
        "pptx" => extract_office(path, |n| n.contains("/slides/slide") && n.ends_with(".xml")),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported type {ext}"),
        )),
    }
}

/// Pull visible text from a tiny RTF document (`{\rtf1 ... hello}`).
pub fn extract_rtf(rtf: &str) -> String {
    let mut out = String::new();
    let mut chars = rtf.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' | '}' => {}
            '\\' => {
                if chars.peek() == Some(&'\\') {
                    chars.next();
                    out.push('\\');
                    continue;
                }
                while let Some(&n) = chars.peek() {
                    if n.is_ascii_alphabetic() {
                        chars.next();
                    } else {
                        break;
                    }
                }
                if chars.peek() == Some(&' ') {
                    chars.next();
                }
            }
            _ => out.push(c),
        }
    }
    collapse_ws(&out)
}

pub fn strip_html(html: &str) -> String {
    xml_to_text(html)
}

fn extract_office(path: &Path, want: impl Fn(&str) -> bool) -> io::Result<String> {
    let file = fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let mut out = String::new();
    for i in 0..zip.len() {
        let mut f = zip
            .by_index(i)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let name = f.name().replace('\\', "/");
        if !want(&name) {
            continue;
        }
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;
        out.push(' ');
        out.push_str(&xml_to_text(&buf));
    }
    Ok(out)
}

pub fn xml_to_text(xml: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut in_entity = false;
    let mut entity = String::new();
    for c in xml.chars() {
        match c {
            '<' => {
                in_tag = true;
                if !out.ends_with(' ') {
                    out.push(' ');
                }
            }
            '>' => in_tag = false,
            '&' if !in_tag => {
                in_entity = true;
                entity.clear();
            }
            ';' if in_entity => {
                out.push_str(&decode_entity(&entity));
                in_entity = false;
            }
            _ if in_entity => entity.push(c),
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    collapse_ws(&out)
}

fn decode_entity(e: &str) -> String {
    match e {
        "amp" => "&".into(),
        "lt" => "<".into(),
        "gt" => ">".into(),
        "quot" => "\"".into(),
        "apos" => "'".into(),
        "nbsp" => " ".into(),
        other => {
            if let Some(hex) = other.strip_prefix("#x").or_else(|| other.strip_prefix("#X")) {
                if let Ok(cp) = u32::from_str_radix(hex, 16) {
                    if let Some(ch) = char::from_u32(cp) {
                        return ch.to_string();
                    }
                }
            } else if let Some(n) = other.strip_prefix('#') {
                if let Ok(cp) = n.parse::<u32>() {
                    if let Some(ch) = char::from_u32(cp) {
                        return ch.to_string();
                    }
                }
            }
            String::new()
        }
    }
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::new();
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            prev_space = false;
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// Pull printable UTF-8 (including Hangul) and PDF literal strings `(...)`.
pub fn extract_pdf(bytes: &[u8]) -> String {
    let mut out = String::new();
    // Parenthesized literals.
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let mut s = String::new();
            i += 1;
            while i < bytes.len() && bytes[i] != b')' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    s.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                s.push(bytes[i] as char);
                i += 1;
            }
            out.push(' ');
            out.push_str(&s);
        }
        i += 1;
    }
    // UTF-8 runs (so Korean embedded in the file is found even without a CMap).
    out.push(' ');
    out.push_str(&utf8_runs(bytes));
    collapse_ws(&out)
}

fn utf8_runs(bytes: &[u8]) -> String {
    let lossy = String::from_utf8_lossy(bytes);
    let mut out = String::new();
    for c in lossy.chars() {
        if c == '\u{FFFD}' {
            continue;
        }
        if c.is_ascii_graphic() || c.is_whitespace() || is_hangul(c) || (c as u32) > 127 {
            if !c.is_control() {
                out.push(c);
            }
        }
    }
    out
}

fn is_hangul(c: char) -> bool {
    let u = c as u32;
    (0xAC00..=0xD7A3).contains(&u) || (0x1100..=0x11FF).contains(&u) || (0x3130..=0x318F).contains(&u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtf_keeps_visible_text() {
        let t = extract_rtf(r"{\rtf1\ansi hello \b 세금\b0 }");
        assert!(t.contains("hello"));
        assert!(t.contains("세금"));
        assert!(is_extractable(Path::new("a.rtf")));
    }

    #[test]
    fn html_strips_tags() {
        assert!(strip_html("<p>Hello <b>세금</b></p>").contains("세금"));
    }

    #[test]
    fn csv_json_log_are_plain_text() {
        assert!(is_extractable(Path::new("a.csv")));
        assert!(is_extractable(Path::new("a.json")));
        assert!(is_extractable(Path::new("a.log")));
        assert!(is_extractable(Path::new("a.xml")));
        assert!(is_extractable(Path::new("a.toml")));
        assert!(is_extractable(Path::new("a.yaml")));
        assert!(is_extractable(Path::new("main.rs")));
        assert!(is_extractable(Path::new("app.ts")));
    }

    #[test]
    fn utf16_le_bom_decodes() {
        let mut v = vec![0xFF, 0xFE];
        for c in "세금".encode_utf16() {
            v.extend_from_slice(&c.to_le_bytes());
        }
        assert_eq!(decode_plain_bytes(&v).unwrap(), "세금");
    }

    #[test]
    fn skip_utf8_bom_drops_ef_bb_bf() {
        assert_eq!(skip_utf8_bom(&[0xEF, 0xBB, 0xBF, b'a']), b"a");
        assert_eq!(skip_utf8_bom(b"abc"), b"abc");
        assert_eq!(skip_utf8_bom(&[0xEF, 0xBB]), &[0xEF, 0xBB]);
    }

    #[test]
    fn pdf_finds_embedded_utf8() {
        let mut pdf = b"%PDF-1.1\n".to_vec();
        pdf.extend_from_slice("세금".as_bytes());
        pdf.extend_from_slice(b"\n%%EOF");
        assert!(extract_pdf(&pdf).contains("세금"));
    }
}
