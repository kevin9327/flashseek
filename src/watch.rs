//! Unprivileged filesystem notifications → `CatalogEvent`.
//!
//! A live directory watcher can fill `FsChange` without `$MFT`/`$UsnJrnl`
//! access. This mapper is pure so tests (and callers) can drive
//! create/delete/rename without starting a watcher thread.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::types::{CatalogEvent, FileRecord};

/// Kind of filesystem notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsChangeKind {
    Create,
    Delete,
    Rename,
    Modify,
}

/// Structured filesystem change. Tests construct fixtures; a watcher would
/// fill the same shape from OS notifications.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsChange {
    pub kind: FsChangeKind,
    pub path: PathBuf,
    pub new_path: Option<PathBuf>,
    pub is_dir: bool,
    pub size: u64,
    pub modified: SystemTime,
}

/// Map a filesystem change onto a catalog create, delete, or rename.
///
/// `next_id` is assigned on `Create`. `existing_id` is the catalog id already
/// known for `path` (from a previous create or a `by_path` lookup).
///
/// `Modify` returns `None`: the name catalog does not need size/mtime-only
/// updates. `Delete`/`Rename` without `existing_id` also return `None`.
pub fn fs_change_to_event(
    change: &FsChange,
    next_id: u64,
    existing_id: Option<u64>,
) -> Option<CatalogEvent> {
    match change.kind {
        FsChangeKind::Create => Some(CatalogEvent::Create(FileRecord {
            id: next_id,
            parent_id: None,
            name: name_from(&change.path),
            path: change.path.clone(),
            size: change.size,
            modified: change.modified,
            is_dir: change.is_dir,
            attributes: 0,
        })),
        FsChangeKind::Delete => existing_id.map(|id| CatalogEvent::Delete { id }),
        FsChangeKind::Rename => {
            let id = existing_id?;
            let new_path = change.new_path.clone()?;
            Some(CatalogEvent::Rename {
                id,
                new_name: name_from(&new_path),
                new_path,
            })
        }
        // Name catalog stays valid without metadata-only refreshes.
        FsChangeKind::Modify => None,
    }
}

fn name_from(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
