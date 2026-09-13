use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecord {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub is_dir: bool,
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
