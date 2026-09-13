pub mod catalog;
pub mod content;
pub mod engine;
pub mod extract;
pub mod mft;
pub mod nl;
pub mod preview;
pub mod query;
pub mod rank;
pub mod search;
pub mod types;
pub mod usn;
pub mod walk;
pub mod watch;

#[cfg(feature = "ui")]
pub mod ui;

pub use catalog::Catalog;
pub use content::ContentIndex;
pub use engine::{parse_engine_config, Engine, EngineConfig};
pub use extract::extract_text;
pub use nl::compile_nl;
pub use query::{parse_query, Query};
pub use search::{search, search_text};
pub use types::{
    CatalogEvent, FileRecord, Hit, HitJson, Snippet, ATTR_DIRECTORY, ATTR_HIDDEN, ATTR_READONLY,
    ATTR_SYSTEM,
};
