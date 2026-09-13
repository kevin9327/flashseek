//! USN change records → `CatalogEvent`.
//!
//! Live `$UsnJrnl` reads need a volume handle; this mapper is pure so tests
//! (and callers) can drive create/delete/rename without opening a disk.

use std::path::PathBuf;
use std::time::SystemTime;

use crate::types::{CatalogEvent, FileRecord};

pub const USN_REASON_FILE_CREATE: u32 = 0x0000_0100;
pub const USN_REASON_FILE_DELETE: u32 = 0x0000_0200;
pub const USN_REASON_RENAME_OLD_NAME: u32 = 0x0000_1000;
pub const USN_REASON_RENAME_NEW_NAME: u32 = 0x0000_2000;
pub const USN_REASON_CLOSE: u32 = 0x8000_0000;

/// Structured USN-like change. Tests construct fixtures; a journal reader
/// would fill the same shape from `USN_RECORD`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsnRecord {
    pub file_id: u64,
    pub parent_id: Option<u64>,
    pub reason: u32,
    pub filename: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub is_dir: bool,
}

/// Map a USN-like change to a catalog create, delete, or rename.
///
/// Reason bits may be combined (e.g. with `USN_REASON_CLOSE`). Delete wins
/// over rename; `RENAME_OLD_NAME` alone yields `None` so the caller waits
/// for `RENAME_NEW_NAME`.
pub fn usn_record_to_event(record: &UsnRecord) -> Option<CatalogEvent> {
    if record.reason & USN_REASON_FILE_DELETE != 0 {
        Some(CatalogEvent::Delete { id: record.file_id })
    } else if record.reason & USN_REASON_RENAME_NEW_NAME != 0 {
        Some(CatalogEvent::Rename {
            id: record.file_id,
            new_name: record.filename.clone(),
            new_path: record.path.clone(),
        })
    } else if record.reason & USN_REASON_FILE_CREATE != 0 {
        Some(CatalogEvent::Create(FileRecord {
            id: record.file_id,
            parent_id: record.parent_id,
            name: record.filename.clone(),
            path: record.path.clone(),
            size: record.size,
            modified: record.modified,
            is_dir: record.is_dir,
            attributes: 0,
        }))
    } else {
        None
    }
}
