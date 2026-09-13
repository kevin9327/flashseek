use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::catalog::Catalog;
use crate::content::ContentIndex;
use crate::search::search_text;
use crate::types::{CatalogEvent, FileRecord, Hit};
use crate::walk::ingest_tree;

#[derive(Debug, Default, Clone)]
pub struct Engine {
    pub catalog: Catalog,
    pub content: ContentIndex,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_roots(name_root: &Path, content_roots: &[PathBuf]) -> io::Result<Self> {
        let mut engine = Self::new();
        ingest_tree(&mut engine, name_root, content_roots)?;
        Ok(engine)
    }

    pub fn from_config_file(path: &Path) -> io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let cfg = parse_engine_config(&text)?;
        Self::from_roots(&cfg.name_root, &cfg.content_roots)
    }

    pub fn apply(&mut self, event: CatalogEvent) {
        self.catalog.apply(event);
    }

    /// Index one file's body if it is extractable. Used after a live create.
    pub fn index_path_body(&mut self, path: &Path) {
        if crate::extract::is_extractable(path) {
            if let Ok(body) = crate::extract::extract_text(path) {
                self.content.index(path.to_path_buf(), body);
            }
        }
    }

    pub fn query(&self, input: &str, now: SystemTime) -> Vec<Hit> {
        search_text(&self.catalog, &self.content, input, now)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub name_root: PathBuf,
    pub content_roots: Vec<PathBuf>,
}

/// Tiny line-oriented config:
/// `name_root=C:\`
/// `content_root=C:\Users\me\Documents`
pub fn parse_engine_config(text: &str) -> io::Result<EngineConfig> {
    let mut name_root = None;
    let mut content_roots = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("line {}: expected key=value", i + 1),
            ));
        };
        let key = k.trim();
        let val = PathBuf::from(v.trim());
        match key {
            "name_root" => name_root = Some(val),
            "content_root" => content_roots.push(val),
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown key {other}"),
                ));
            }
        }
    }
    let name_root = name_root.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "name_root is required")
    })?;
    Ok(EngineConfig {
        name_root,
        content_roots,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_and_content_roots() {
        let cfg = parse_engine_config(
            "name_root=C:\\\n# comment\ncontent_root=C:\\Users\\a\\Documents\ncontent_root=C:\\Users\\a\\Downloads\n",
        )
        .unwrap();
        assert_eq!(cfg.name_root, PathBuf::from("C:\\"));
        assert_eq!(cfg.content_roots.len(), 2);
    }

    #[test]
    fn rejects_missing_name_root() {
        let err = parse_engine_config("content_root=C:\\docs\n").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn from_config_file_indexes_named_root() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("hello.txt"), "hi").unwrap();
        let cfg_path = tmp.path().join("flashseek.conf");
        std::fs::write(
            &cfg_path,
            format!(
                "name_root={}\ncontent_root={}\n",
                tmp.path().display(),
                tmp.path().display()
            ),
        )
        .unwrap();
        let engine = Engine::from_config_file(&cfg_path).unwrap();
        assert!(engine.catalog.iter().any(|r| r.name == "hello.txt"));
    }

    #[test]
    fn index_path_body_makes_body_searchable() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("secret.txt");
        std::fs::write(&path, "bodytoken-xyz").unwrap();
        let mut engine = Engine::new();
        engine.index_path_body(&path);
        let rec = FileRecord {
            id: 1,
            parent_id: None,
            name: "secret.txt".into(),
            path: path.clone(),
            size: 13,
            modified: SystemTime::now(),
            is_dir: false,
            attributes: 0,
        };
        engine.catalog.insert(rec);
        let hits = engine.query("bodytoken-xyz", SystemTime::now());
        assert_eq!(hits.len(), 1);
        assert!(hits[0].content_match);
    }
}
