//! MFT volume probe and USN → CatalogEvent mapper (no live journal required).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use flashseek::mft::try_index_volume;
use flashseek::usn::{
    usn_record_to_event, UsnRecord, USN_REASON_FILE_CREATE, USN_REASON_FILE_DELETE,
    USN_REASON_RENAME_NEW_NAME,
};
use flashseek::{Catalog, CatalogEvent};

fn fixture(
    file_id: u64,
    parent_id: Option<u64>,
    reason: u32,
    filename: &str,
    path: &str,
) -> UsnRecord {
    UsnRecord {
        file_id,
        parent_id,
        reason,
        filename: filename.to_string(),
        path: PathBuf::from(path),
        size: 12,
        modified: SystemTime::UNIX_EPOCH,
        is_dir: false,
    }
}

#[test]
fn usn_create_maps_to_catalog_event_create() {
    let rec = fixture(
        1001,
        Some(100),
        USN_REASON_FILE_CREATE,
        "notes.txt",
        r"C:\docs\notes.txt",
    );
    let event = usn_record_to_event(&rec).expect("create reason must map");
    match event {
        CatalogEvent::Create(fr) => {
            assert_eq!(fr.id, 1001);
            assert_eq!(fr.parent_id, Some(100));
            assert_eq!(fr.name, "notes.txt");
            assert_eq!(fr.path, PathBuf::from(r"C:\docs\notes.txt"));
            assert_eq!(fr.size, 12);
            assert!(!fr.is_dir);
        }
        other => panic!("expected Create, got {other:?}"),
    }
}

#[test]
fn usn_delete_maps_to_catalog_event_delete() {
    let rec = fixture(
        1001,
        Some(100),
        USN_REASON_FILE_DELETE,
        "notes.txt",
        r"C:\docs\notes.txt",
    );
    let event = usn_record_to_event(&rec).expect("delete reason must map");
    match event {
        CatalogEvent::Delete { id } => assert_eq!(id, 1001),
        other => panic!("expected Delete, got {other:?}"),
    }
}

#[test]
fn usn_rename_maps_to_catalog_event_rename() {
    let rec = fixture(
        1001,
        Some(100),
        USN_REASON_RENAME_NEW_NAME,
        "renamed.txt",
        r"C:\docs\renamed.txt",
    );
    let event = usn_record_to_event(&rec).expect("rename reason must map");
    match event {
        CatalogEvent::Rename {
            id,
            new_name,
            new_path,
        } => {
            assert_eq!(id, 1001);
            assert_eq!(new_name, "renamed.txt");
            assert_eq!(new_path, PathBuf::from(r"C:\docs\renamed.txt"));
        }
        other => panic!("expected Rename, got {other:?}"),
    }
}

#[test]
fn try_index_volume_probes_c_drive() {
    let mut catalog = Catalog::new();
    match try_index_volume(Path::new(r"C:\"), &mut catalog) {
        Ok(count) => {
            eprintln!("MFT access succeeded: indexed {count} FileRecord(s)");
            assert!(
                count > 0,
                "MFT open succeeded but indexed 0 records; refusing to treat as success"
            );
            assert_eq!(catalog.len(), count as usize);
            assert!(catalog.iter().next().is_some());
        }
        Err(e) => {
            eprintln!(
                "MFT access denied or failed: kind={:?} os={:?} {e}",
                e.kind(),
                e.raw_os_error()
            );
        }
    }
}
