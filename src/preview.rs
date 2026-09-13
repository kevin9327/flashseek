//! Windows preview-handler lookup.
//!
//! `{8895b1c6-b41f-4c1c-a562-0d564250836f}` is the IPreviewHandler shell
//! extension. The GUI uses this when a snippet is missing and a handler is
//! registered. Parsing is a pure function so tests never need a live HWND.

use std::path::Path;

/// IPreviewHandler shell-ex GUID.
pub const PREVIEW_HANDLER_SHELLEX: &str = "{8895b1c6-b41f-4c1c-a562-0d564250836f}";

/// Parse a registry default value into a CLSID string (`{guid}`).
pub fn parse_preview_clsid(value: &str) -> Option<String> {
    let v = value.trim();
    if v.len() == 38 && v.starts_with('{') && v.ends_with('}') {
        let inner = &v[1..37];
        if inner.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return Some(v.to_ascii_uppercase());
        }
    }
    None
}

/// Look up a preview handler CLSID for `path`'s extension via HKCR.
/// Returns `Ok(None)` when nothing is registered; `Err` on registry I/O.
pub fn preview_handler_clsid(path: &Path) -> std::io::Result<Option<String>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{}", s.to_ascii_lowercase()))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no extension"))?;
    lookup_hkcr_preview(&ext)
}

#[cfg(windows)]
fn lookup_hkcr_preview(ext: &str) -> std::io::Result<Option<String>> {
    use std::process::Command;
    // Avoid extra crates: `reg query` is the OS-provided reader.
    let key = format!("HKCR\\{ext}\\shellex\\{PREVIEW_HANDLER_SHELLEX}");
    let out = Command::new("reg")
        .args(["query", &key, "/ve"])
        .output()?;
    if !out.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if let Some(idx) = line.find('{') {
            if let Some(cls) = parse_preview_clsid(&line[idx..]) {
                return Ok(Some(cls));
            }
        }
    }
    Ok(None)
}

#[cfg(not(windows))]
fn lookup_hkcr_preview(_ext: &str) -> std::io::Result<Option<String>> {
    Ok(None)
}

/// True when we should prefer a native preview handler over a text snippet.
pub fn should_use_handler(path: &Path, has_snippet: bool) -> bool {
    if has_snippet {
        return false;
    }
    matches!(
        path.extension().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase()).as_deref(),
        Some("pdf" | "docx" | "xlsx" | "pptx" | "png" | "jpg" | "jpeg" | "gif" | "bmp")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_canonical_clsid() {
        let raw = "{8895b1c6-b41f-4c1c-a562-0d564250836f}";
        assert_eq!(
            parse_preview_clsid(raw).unwrap(),
            "{8895B1C6-B41F-4C1C-A562-0D564250836F}"
        );
        assert!(parse_preview_clsid("not-a-clsid").is_none());
        assert!(parse_preview_clsid("{zz}").is_none());
    }

    #[test]
    fn handler_preferred_for_pdf_without_snippet() {
        assert!(should_use_handler(&PathBuf::from("C:\\\\a.pdf"), false));
        assert!(!should_use_handler(&PathBuf::from("C:\\\\a.pdf"), true));
        assert!(!should_use_handler(&PathBuf::from("C:\\\\a.txt"), false));
    }
}
