use std::collections::HashMap;
use std::path::Path;

use crate::types::{CatalogEvent, FileRecord};

#[derive(Debug, Default, Clone)]
pub struct Catalog {
    by_id: HashMap<u64, FileRecord>,
}

impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn insert(&mut self, rec: FileRecord) {
        self.by_id.insert(rec.id, rec);
    }

    pub fn get(&self, id: u64) -> Option<&FileRecord> {
        self.by_id.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut FileRecord> {
        self.by_id.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &FileRecord> {
        self.by_id.values()
    }

    pub fn by_path(&self, path: &Path) -> Option<&FileRecord> {
        self.by_id.values().find(|r| r.path == path)
    }

    /// Live updates: after create/delete/rename the next query sees the new catalog.
    pub fn apply(&mut self, event: CatalogEvent) {
        match event {
            CatalogEvent::Create(rec) => {
                self.by_id.insert(rec.id, rec);
            }
            CatalogEvent::Delete { id } => {
                self.by_id.remove(&id);
            }
            CatalogEvent::Rename {
                id,
                new_name,
                new_path,
            } => {
                let (old_path, is_dir) = {
                    let Some(rec) = self.by_id.get_mut(&id) else {
                        return;
                    };
                    let old_path = rec.path.clone();
                    let is_dir = rec.is_dir;
                    rec.name = new_name;
                    rec.path = new_path.clone();
                    (old_path, is_dir)
                };
                // Directory rename rewrites descendant path prefixes so children stay under the new location.
                if is_dir {
                    for rec in self.by_id.values_mut() {
                        if rec.id == id {
                            continue;
                        }
                        if let Ok(rel) = rec.path.strip_prefix(&old_path) {
                            rec.path = new_path.join(rel);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn rec(id: u64, name: &str) -> FileRecord {
        FileRecord {
            id,
            parent_id: None,
            name: name.to_string(),
            path: PathBuf::from(format!("C:\\x\\{name}")),
            size: 1,
            modified: SystemTime::UNIX_EPOCH,
            is_dir: false,
        }
    }

    #[test]
    fn apply_create_delete_rename() {
        let mut c = Catalog::new();
        c.apply(CatalogEvent::Create(rec(1, "a.txt")));
        assert_eq!(c.len(), 1);
        c.apply(CatalogEvent::Rename {
            id: 1,
            new_name: "b.txt".into(),
            new_path: PathBuf::from("C:\\x\\b.txt"),
        });
        assert_eq!(c.get(1).unwrap().name, "b.txt");
        c.apply(CatalogEvent::Delete { id: 1 });
        assert!(c.is_empty());
    }
}
