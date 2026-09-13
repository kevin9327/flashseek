use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::engine::Engine;
use crate::extract::{extract_text, is_extractable};
use crate::types::FileRecord;

/// Skip body extraction for huge blobs so the name catalog stays instant.
pub const MAX_CONTENT_BYTES: u64 = 32 * 1024 * 1024;

pub fn ingest_tree(
    engine: &mut Engine,
    name_root: &Path,
    content_roots: &[PathBuf],
) -> io::Result<u64> {
    let mut next_id = engine.catalog.iter().map(|r| r.id).max().unwrap_or(0) + 1;
    walk(engine, name_root, None, content_roots, &mut next_id)?;
    Ok(next_id)
}

fn walk(
    engine: &mut Engine,
    dir: &Path,
    parent: Option<u64>,
    content_roots: &[PathBuf],
    next_id: &mut u64,
) -> io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            // Skip unreadable dirs so a full-volume walk still returns the rest.
            if e.kind() == io::ErrorKind::PermissionDenied {
                return Ok(());
            }
            return Err(e);
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = meta.is_dir();
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let size = if is_dir { 0 } else { meta.len() };
        let name = entry.file_name().to_string_lossy().into_owned();
        let skip_children = is_dir && should_skip_dir_name(&name);
        let id = *next_id;
        *next_id += 1;
        let rec = FileRecord {
            id,
            parent_id: parent,
            name,
            path: path.clone(),
            size,
            modified,
            is_dir,
            attributes: file_attributes(&meta, is_dir),
        };
        engine.catalog.insert(rec);
        if !is_dir && should_index_content(size, is_extractable(&path), under_any(&path, content_roots))
        {
            if let Ok(body) = extract_text(&path) {
                engine.content.index(path.clone(), body);
            }
        }
        if is_dir && !skip_children {
            walk(engine, &path, Some(id), content_roots, next_id)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn file_attributes(meta: &fs::Metadata, _is_dir: bool) -> u32 {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes()
}

#[cfg(not(windows))]
fn file_attributes(_meta: &fs::Metadata, is_dir: bool) -> u32 {
    if is_dir {
        crate::types::ATTR_DIRECTORY
    } else {
        0
    }
}

fn under_any(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|r| path.starts_with(r))
}

pub fn should_index_content(size: u64, extractable: bool, under_root: bool) -> bool {
    extractable && under_root && size <= MAX_CONTENT_BYTES
}

/// Skip VCS and package trees so the name catalog stays useful.
/// `name_root` itself is still walked even when it matches these names.
pub fn should_skip_dir_name(name: &str) -> bool {
    matches!(name, ".git" | ".svn" | ".hg") || name.eq_ignore_ascii_case("node_modules")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_huge_blobs_keeps_normal_docs() {
        assert!(should_index_content(1024, true, true));
        assert!(!should_index_content(MAX_CONTENT_BYTES + 1, true, true));
        assert!(!should_index_content(1024, false, true));
        assert!(!should_index_content(1024, true, false));
    }

    #[test]
    fn skips_vcs_and_node_modules_dir_names() {
        assert!(should_skip_dir_name(".git"));
        assert!(should_skip_dir_name(".svn"));
        assert!(should_skip_dir_name(".hg"));
        assert!(should_skip_dir_name("node_modules"));
        assert!(should_skip_dir_name("NODE_MODULES"));
        assert!(should_skip_dir_name("Node_Modules"));
        assert!(!should_skip_dir_name("keep.txt"));
        assert!(!should_skip_dir_name(".gitignore"));
        assert!(!should_skip_dir_name("src"));
    }
}
