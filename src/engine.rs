use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::catalog::Catalog;
use crate::content::ContentIndex;
use crate::search::search_text;
use crate::types::{CatalogEvent, Hit};
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

    pub fn apply(&mut self, event: CatalogEvent) {
        self.catalog.apply(event);
    }

    pub fn query(&self, input: &str, now: SystemTime) -> Vec<Hit> {
        search_text(&self.catalog, &self.content, input, now)
    }
}
