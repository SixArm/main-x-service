//! Tantivy-backed search index. STUB.
//!
//! The first iteration mirrors `person-service::search::SearchEngine`
//! — open / create at a directory path, index on every CRUD write,
//! force `reload()` after each commit so reads observe the new
//! segment immediately. Search field-set: name + alternate_names +
//! course_code + provider_name + identifier_values + keywords + teaches.

use std::path::Path;

use crate::Result;
use crate::models::Course;

pub struct SearchEngine {
    /// Resolved index directory. Held so consumers can log the path.
    pub index_path: String,
}

impl SearchEngine {
    /// Open (or create) the index at `path`.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        std::fs::create_dir_all(p)
            .map_err(|e| crate::Error::Search(format!("Failed to ensure index dir: {e}")))?;
        Ok(Self {
            index_path: p.to_string_lossy().into_owned(),
        })
    }

    /// Index a single Course. STUB.
    pub fn index_course(&self, _course: &Course) -> Result<()> {
        Err(crate::Error::Search("SearchEngine::index_course not implemented".into()))
    }

    /// Full-text search over the indexed fields. STUB.
    pub fn search(&self, _query: &str, _limit: usize) -> Result<Vec<String>> {
        Err(crate::Error::Search("SearchEngine::search not implemented".into()))
    }

    /// Blocking query used by the duplicate detector. STUB.
    pub fn search_by_name_and_provider(
        &self,
        _name: &str,
        _provider_id: Option<uuid::Uuid>,
        _limit: usize,
    ) -> Result<Vec<String>> {
        Err(crate::Error::Search(
            "SearchEngine::search_by_name_and_provider not implemented".into(),
        ))
    }
}
