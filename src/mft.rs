//! NTFS `$MFT` / USN adapter.
//!
//! Privileged path: open `\\.\C:` and parse the master file table, then follow
//! `$UsnJrnl` for live create/delete/rename. Unprivileged path: the directory
//! walker in `walk` produces the same `FileRecord` shape, which tests inject.
//!
//! This module is a thin adapter so search never depends on a real volume.

use std::io;
use std::path::Path;

use crate::catalog::Catalog;

/// Attempt to fill `catalog` from the NTFS MFT of `volume` (e.g. `C:\\`).
/// Returns `Err` when the handle is denied; callers fall back to a walker or fixture.
pub fn try_index_volume(volume: &Path, catalog: &mut Catalog) -> io::Result<u64> {
    #[cfg(windows)]
    {
        win::index_volume(volume, catalog)
    }
    #[cfg(not(windows))]
    {
        let _ = (volume, catalog);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "MFT indexing requires Windows NTFS",
        ))
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::mem;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{FromRawHandle, RawHandle};
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    use crate::types::FileRecord;

    const GENERIC_READ: u32 = 0x8000_0000;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_WRITE: u32 = 0x2;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FSCTL_ENUM_USN_DATA: u32 = 0x0009_00B3;
    const ERROR_HANDLE_EOF: i32 = 38;
    const ERROR_MORE_DATA: i32 = 234;
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    const MFT_ROOT_INDEX: u64 = 5;
    const FRN_INDEX_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

    #[repr(C)]
    struct MftEnumDataV0 {
        start_file_reference_number: u64,
        low_usn: i64,
        high_usn: i64,
    }

    struct MftNode {
        parent: Option<u64>,
        name: String,
        is_dir: bool,
        modified: SystemTime,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateFileW(
            lp_file_name: *const u16,
            dw_desired_access: u32,
            dw_share_mode: u32,
            lp_security_attributes: *mut u8,
            dw_creation_disposition: u32,
            dw_flags_and_attributes: u32,
            h_template_file: RawHandle,
        ) -> RawHandle;

        fn DeviceIoControl(
            h_device: RawHandle,
            dw_io_control_code: u32,
            lp_in_buffer: *mut u8,
            n_in_buffer_size: u32,
            lp_out_buffer: *mut u8,
            n_out_buffer_size: u32,
            lp_bytes_returned: *mut u32,
            lp_overlapped: *mut u8,
        ) -> i32;
    }

    pub(super) fn index_volume(volume: &Path, catalog: &mut Catalog) -> io::Result<u64> {
        let device = volume_device_path(volume)?;
        let file = open_volume(&device)?;
        verify_ntfs_boot(&file)?;
        let nodes = enum_mft(&file)?;
        drop(file);

        let root = volume_root_path(volume);
        let mut cache = HashMap::new();
        let mut n = 0u64;
        for (&id, node) in &nodes {
            let path = resolve_path(id, &nodes, &mut cache, &root);
            catalog.insert(FileRecord {
                id,
                parent_id: node
                    .parent
                    .filter(|_| (id & FRN_INDEX_MASK) != MFT_ROOT_INDEX),
                name: node.name.clone(),
                path,
                size: 0,
                modified: node.modified,
                is_dir: node.is_dir,
            });
            n += 1;
        }
        Ok(n)
    }

    fn volume_device_path(volume: &Path) -> io::Result<PathBuf> {
        let s = volume.to_string_lossy();
        let rest = s
            .strip_prefix(r"\\.\")
            .or_else(|| s.strip_prefix(r"\\?\"))
            .unwrap_or(s.as_ref());
        let rest = rest.trim_end_matches(['\\', '/']);
        let mut chars = rest.chars();
        match (chars.next(), chars.next()) {
            (Some(letter), Some(':')) if letter.is_ascii_alphabetic() => Ok(PathBuf::from(
                format!(r"\\.\{}:", letter.to_ascii_uppercase()),
            )),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("cannot derive volume device path from {}", volume.display()),
            )),
        }
    }

    fn volume_root_path(volume: &Path) -> PathBuf {
        let s = volume.to_string_lossy();
        let rest = s
            .strip_prefix(r"\\.\")
            .or_else(|| s.strip_prefix(r"\\?\"))
            .unwrap_or(s.as_ref());
        let mut chars = rest.chars();
        match (chars.next(), chars.next()) {
            (Some(letter), Some(':')) if letter.is_ascii_alphabetic() => {
                PathBuf::from(format!("{}:\\", letter.to_ascii_uppercase()))
            }
            _ => volume.to_path_buf(),
        }
    }

    fn open_volume(device: &Path) -> io::Result<File> {
        let wide: Vec<u16> = device
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                0 as RawHandle,
            )
        };
        if handle == INVALID_HANDLE {
            return Err(io::Error::last_os_error());
        }
        Ok(unsafe { File::from_raw_handle(handle) })
    }

    const INVALID_HANDLE: RawHandle = -1isize as RawHandle;

    fn verify_ntfs_boot(mut file: &File) -> io::Result<()> {
        file.seek(SeekFrom::Start(0))?;
        let mut boot = [0u8; 4096];
        file.read_exact(&mut boot)?;
        if &boot[3..11] != b"NTFS    " {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "volume boot sector is not NTFS",
            ));
        }
        Ok(())
    }

    fn enum_mft(file: &File) -> io::Result<HashMap<u64, MftNode>> {
        use std::os::windows::io::AsRawHandle;

        let mut med = MftEnumDataV0 {
            start_file_reference_number: 0,
            low_usn: 0,
            high_usn: i64::MAX,
        };
        let mut buf = vec![0u8; 1024 * 1024];
        let mut nodes = HashMap::new();
        let handle = file.as_raw_handle();

        loop {
            let mut bytes = 0u32;
            let ok = unsafe {
                DeviceIoControl(
                    handle,
                    FSCTL_ENUM_USN_DATA,
                    &mut med as *mut MftEnumDataV0 as *mut u8,
                    mem::size_of::<MftEnumDataV0>() as u32,
                    buf.as_mut_ptr(),
                    buf.len() as u32,
                    &mut bytes,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                let err = io::Error::last_os_error();
                match err.raw_os_error() {
                    Some(ERROR_HANDLE_EOF) => break,
                    Some(ERROR_MORE_DATA) if bytes as usize > 8 => {}
                    _ => return Err(err),
                }
            }
            let n = bytes as usize;
            if n < 8 {
                break;
            }
            let next = u64::from_le_bytes(buf[0..8].try_into().unwrap());
            parse_enum_records(&buf[8..n], &mut nodes);
            if next == med.start_file_reference_number {
                break;
            }
            med.start_file_reference_number = next;
        }
        Ok(nodes)
    }

    fn parse_enum_records(buf: &[u8], nodes: &mut HashMap<u64, MftNode>) {
        let mut offset = 0usize;
        while offset < buf.len() {
            if offset + 4 > buf.len() {
                break;
            }
            let record_length =
                u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap()) as usize;
            if record_length < 60 || offset + record_length > buf.len() {
                break;
            }
            let rec = &buf[offset..offset + record_length];
            let major = u16::from_le_bytes(rec[4..6].try_into().unwrap());
            if let Some((id, node)) = parse_one(rec, major) {
                nodes.insert(id, node);
            }
            if record_length == 0 {
                break;
            }
            offset += record_length;
        }
    }

    fn parse_one(rec: &[u8], major: u16) -> Option<(u64, MftNode)> {
        let (id, parent, attrs, name_len, name_off, timestamp) = match major {
            2 if rec.len() >= 60 => {
                let id = u64::from_le_bytes(rec[8..16].try_into().ok()?);
                let parent = u64::from_le_bytes(rec[16..24].try_into().ok()?);
                let timestamp = i64::from_le_bytes(rec[32..40].try_into().ok()?);
                let attrs = u32::from_le_bytes(rec[52..56].try_into().ok()?);
                let name_len = u16::from_le_bytes(rec[56..58].try_into().ok()?) as usize;
                let name_off = u16::from_le_bytes(rec[58..60].try_into().ok()?) as usize;
                (id, parent, attrs, name_len, name_off, timestamp)
            }
            3 if rec.len() >= 76 => {
                let id = u64::from_le_bytes(rec[8..16].try_into().ok()?);
                let parent = u64::from_le_bytes(rec[24..32].try_into().ok()?);
                let timestamp = i64::from_le_bytes(rec[48..56].try_into().ok()?);
                let attrs = u32::from_le_bytes(rec[68..72].try_into().ok()?);
                let name_len = u16::from_le_bytes(rec[72..74].try_into().ok()?) as usize;
                let name_off = u16::from_le_bytes(rec[74..76].try_into().ok()?) as usize;
                (id, parent, attrs, name_len, name_off, timestamp)
            }
            _ => return None,
        };
        if name_off + name_len > rec.len() {
            return None;
        }
        let name = utf16le(&rec[name_off..name_off + name_len]);
        if name.is_empty() {
            return None;
        }
        Some((
            id,
            MftNode {
                parent: if parent == 0 { None } else { Some(parent) },
                name,
                is_dir: attrs & FILE_ATTRIBUTE_DIRECTORY != 0,
                modified: filetime_to_system_time(timestamp),
            },
        ))
    }

    fn utf16le(bytes: &[u8]) -> String {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    }

    fn filetime_to_system_time(ft: i64) -> SystemTime {
        const EPOCH_DIFF: i64 = 116_444_736_000_000_000;
        let t = ft.saturating_sub(EPOCH_DIFF);
        if t <= 0 {
            SystemTime::UNIX_EPOCH
        } else {
            SystemTime::UNIX_EPOCH + Duration::from_nanos((t as u64).saturating_mul(100))
        }
    }

    fn resolve_path(
        id: u64,
        nodes: &HashMap<u64, MftNode>,
        cache: &mut HashMap<u64, PathBuf>,
        root: &Path,
    ) -> PathBuf {
        if let Some(p) = cache.get(&id) {
            return p.clone();
        }
        let mut parts = Vec::new();
        let mut cur = id;
        for _ in 0..512 {
            if let Some(p) = cache.get(&cur) {
                let mut path = p.clone();
                for name in parts.iter().rev() {
                    path.push(name);
                }
                cache.insert(id, path.clone());
                return path;
            }
            if cur & FRN_INDEX_MASK == MFT_ROOT_INDEX {
                break;
            }
            match nodes.get(&cur) {
                Some(node) if !node.name.is_empty() && node.name != "." => {
                    parts.push(node.name.clone());
                    match node.parent {
                        Some(p) if p != cur => cur = p,
                        _ => break,
                    }
                }
                _ => break,
            }
        }
        let mut path = root.to_path_buf();
        for name in parts.iter().rev() {
            path.push(name);
        }
        cache.insert(id, path.clone());
        path
    }
}
