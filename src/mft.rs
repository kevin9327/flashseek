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
pub fn try_index_volume(_volume: &Path, _catalog: &mut Catalog) -> io::Result<u64> {
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "MFT indexing is not yet wired; use Engine::from_roots or an injected catalog",
    ))
}
