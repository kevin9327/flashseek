use std::collections::HashMap;
use std::path::Path;

use crate::types::{CatalogEvent, FileRecord};

/// First 1–3 lowercase chars of a name; used as prefix-map keys.
const PREFIX_MAX_CHARS: usize = 3;

#[derive(Debug, Default, Clone)]
pub struct Catalog {
    by_id: HashMap<u64, FileRecord>,
    /// Optional name index: lowercase prefixes (1–3 chars) → record ids.
    by_prefix: HashMap<String, Vec<u64>>,
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
        if let Some(old) = self.by_id.remove(&rec.id) {
            self.unindex_name(old.id, &old.name);
        }
        self.index_name(rec.id, &rec.name);
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

    /// Candidates whose names start with `prefix` (1–3 chars, case-insensitive).
    /// Longer prefixes still filter against the full string after a 1–3 char lookup.
    pub fn names_with_prefix(&self, prefix: &str) -> Vec<&FileRecord> {
        let needle = prefix.to_ascii_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let key: String = needle.chars().take(PREFIX_MAX_CHARS).collect();
        let Some(ids) = self.by_prefix.get(&key) else {
            return Vec::new();
        };
        ids.iter()
            .filter_map(|id| self.by_id.get(id))
            .filter(|rec| rec.name.to_ascii_lowercase().starts_with(&needle))
            .collect()
    }

    /// Live updates: after create/delete/rename the next query sees the new catalog.
    pub fn apply(&mut self, event: CatalogEvent) {
        match event {
            CatalogEvent::Create(rec) => {
                self.insert(rec);
            }
            CatalogEvent::Delete { id } => {
                if let Some(old) = self.by_id.remove(&id) {
                    self.unindex_name(old.id, &old.name);
                }
            }
            CatalogEvent::Rename {
                id,
                new_name,
                new_path,
            } => {
                let (old_path, old_name, is_dir) = {
                    let Some(rec) = self.by_id.get_mut(&id) else {
                        return;
                    };
                    let old_path = rec.path.clone();
                    let old_name = rec.name.clone();
                    let is_dir = rec.is_dir;
                    rec.name = new_name.clone();
                    rec.path = new_path.clone();
                    (old_path, old_name, is_dir)
                };
                if old_name != new_name {
                    self.unindex_name(id, &old_name);
                    self.index_name(id, &new_name);
                }
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

    fn index_name(&mut self, id: u64, name: &str) {
        for prefix in name_prefixes(name) {
            let bucket = self.by_prefix.entry(prefix).or_default();
            if !bucket.contains(&id) {
                bucket.push(id);
            }
        }
    }

    fn unindex_name(&mut self, id: u64, name: &str) {
        for prefix in name_prefixes(name) {
            let empty = match self.by_prefix.get_mut(&prefix) {
                Some(bucket) => {
                    bucket.retain(|&existing| existing != id);
                    bucket.is_empty()
                }
                None => false,
            };
            if empty {
                self.by_prefix.remove(&prefix);
            }
        }
    }
}

fn name_prefixes(name: &str) -> Vec<String> {
    let lower = name.to_ascii_lowercase();
    let chars: Vec<char> = lower.chars().take(PREFIX_MAX_CHARS).collect();
    (1..=chars.len())
        .map(|n| chars[..n].iter().collect())
        .collect()
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
            attributes: 0,
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
