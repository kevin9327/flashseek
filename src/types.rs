use std::path::PathBuf;
use std::time::SystemTime;

/// Windows `FILE_ATTRIBUTE_READONLY`.
pub const ATTR_READONLY: u32 = 0x0000_0001;
/// Windows `FILE_ATTRIBUTE_HIDDEN`.
pub const ATTR_HIDDEN: u32 = 0x0000_0002;
/// Windows `FILE_ATTRIBUTE_SYSTEM`.
pub const ATTR_SYSTEM: u32 = 0x0000_0004;
/// Windows `FILE_ATTRIBUTE_DIRECTORY`.
pub const ATTR_DIRECTORY: u32 = 0x0000_0010;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecord {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub is_dir: bool,
    /// Windows file attributes (`FILE_ATTRIBUTE_*` bits).
    pub attributes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogEvent {
    Create(FileRecord),
    Delete { id: u64 },
    Rename {
        id: u64,
        new_name: String,
        new_path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Snippet {
    pub text: String,
    /// Byte offsets into `text` for matched terms.
    pub highlights: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub record: FileRecord,
    pub score: f64,
    pub name_match: bool,
    pub path_match: bool,
    pub content_match: bool,
    pub snippet: Option<Snippet>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HitJson {
    pub path: String,
    pub name: String,
    pub score: f64,
    pub name_match: bool,
    pub path_match: bool,
    pub content_match: bool,
    pub snippet: Option<String>,
    pub highlights: Vec<(usize, usize)>,
}

impl From<&Hit> for HitJson {
    fn from(hit: &Hit) -> Self {
        HitJson {
            path: hit.record.path.to_string_lossy().into_owned(),
            name: hit.record.name.clone(),
            score: hit.score,
            name_match: hit.name_match,
            path_match: hit.path_match,
            content_match: hit.content_match,
            snippet: hit.snippet.as_ref().map(|s| s.text.clone()),
            highlights: hit
                .snippet
                .as_ref()
                .map(|s| s.highlights.clone())
                .unwrap_or_default(),
        }
    }
}
