//! Tantivy index schema + lifecycle for Course records.
//!
//! Field-set: `id` (stored), `name`, `alternate_names`, `course_code`,
//! `provider_id`, `provider_name`, `keywords`, `teaches`, `identifiers`,
//! `active`. The `provider_id` and `course_code` fields are STRING so
//! the duplicate detector can filter by exact value; everything else is
//! TEXT so fuzzy + prefix queries work.

use std::path::Path;

use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy,
    schema::{FAST, Field, STORED, STRING, Schema, TEXT},
};

use crate::Result;

/// The Tantivy [`Schema`] plus typed handles to each [`Field`], built
/// once and reused for every index/query operation.
#[derive(Clone)]
pub struct CourseIndexSchema {
    /// The built Tantivy schema.
    pub schema: Schema,
    /// Stored course UUID (STRING) — the retrievable primary key.
    pub id: Field,
    /// Course name (TEXT) — fuzzy/full-text searchable.
    pub name: Field,
    /// Alternate names (TEXT).
    pub alternate_names: Field,
    /// Provider catalog code (STRING) — exact-match filterable.
    pub course_code: Field,
    /// Provider UUID (STRING) — exact-match filter for blocking.
    pub provider_id: Field,
    /// Provider name (TEXT).
    pub provider_name: Field,
    /// Joined keyword tokens (TEXT).
    pub keywords: Field,
    /// Joined `teaches` competency tokens (TEXT).
    pub teaches: Field,
    /// Joined identifier values (TEXT).
    pub identifiers: Field,
    /// Active flag as `"true"`/`"false"` (STRING, FAST).
    pub active: Field,
}

impl CourseIndexSchema {
    /// Build the schema, registering every field with its index options
    /// (STRING for exact-match fields, TEXT for full-text fields).
    #[must_use]
    pub fn new() -> Self {
        let mut b = Schema::builder();
        let id = b.add_text_field("id", STRING | STORED);
        let name = b.add_text_field("name", TEXT | STORED);
        let alternate_names = b.add_text_field("alternate_names", TEXT | STORED);
        let course_code = b.add_text_field("course_code", STRING | STORED);
        let provider_id = b.add_text_field("provider_id", STRING | STORED);
        let provider_name = b.add_text_field("provider_name", TEXT | STORED);
        let keywords = b.add_text_field("keywords", TEXT | STORED);
        let teaches = b.add_text_field("teaches", TEXT | STORED);
        let identifiers = b.add_text_field("identifiers", TEXT | STORED);
        let active = b.add_text_field("active", STRING | FAST);
        let schema = b.build();
        Self {
            schema,
            id,
            name,
            alternate_names,
            course_code,
            provider_id,
            provider_name,
            keywords,
            teaches,
            identifiers,
            active,
        }
    }
}

impl Default for CourseIndexSchema {
    /// Same as [`CourseIndexSchema::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// An open Tantivy index together with its schema and a live reader.
pub struct CourseIndex {
    /// The underlying Tantivy index.
    index: Index,
    /// Cached schema + field handles.
    schema: CourseIndexSchema,
    /// Reader configured to reload on commit.
    reader: IndexReader,
}

impl CourseIndex {
    /// Create a brand-new index in an empty directory at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`enum@crate::Error`] if Tantivy cannot create the index
    /// directory or build a reader over it.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = CourseIndexSchema::new();
        let index = Index::create_in_dir(path, schema.schema.clone())
            .map_err(|e| crate::Error::Search(format!("create index: {e}")))?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| crate::Error::Search(format!("create reader: {e}")))?;
        Ok(Self {
            index,
            schema,
            reader,
        })
    }

    /// Open an existing index previously created at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`enum@crate::Error`] if Tantivy cannot open the index at
    /// `path` or build a reader over it.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = CourseIndexSchema::new();
        let index = Index::open_in_dir(path)
            .map_err(|e| crate::Error::Search(format!("open index: {e}")))?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| crate::Error::Search(format!("create reader: {e}")))?;
        Ok(Self {
            index,
            schema,
            reader,
        })
    }

    /// Open the index if a `meta.json` already exists at `path`,
    /// otherwise create a fresh one. The boot-time entry point.
    ///
    /// # Errors
    ///
    /// Returns [`enum@crate::Error`] if the underlying
    /// [`open`](Self::open) / [`create`](Self::create) call fails.
    pub fn create_or_open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        if p.join("meta.json").exists() {
            Self::open(p)
        } else {
            Self::create(p)
        }
    }

    /// Acquire a writer with a `heap_mb`-megabyte budget for the
    /// in-memory indexing buffer.
    ///
    /// # Errors
    ///
    /// Returns [`enum@crate::Error`] if Tantivy cannot allocate the writer.
    pub fn writer(&self, heap_mb: usize) -> Result<IndexWriter> {
        self.index
            .writer(heap_mb * 1_000_000)
            .map_err(|e| crate::Error::Search(format!("create writer: {e}")))
    }

    /// Borrow the underlying Tantivy index (for query-parser setup).
    #[must_use]
    pub fn index(&self) -> &Index {
        &self.index
    }
    /// Borrow the schema + field handles.
    #[must_use]
    pub fn schema(&self) -> &CourseIndexSchema {
        &self.schema
    }
    /// Borrow the live reader.
    #[must_use]
    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }

    /// Force the reader to pick up the latest committed segments.
    ///
    /// # Errors
    ///
    /// Returns [`enum@crate::Error`] if the reader fails to reload.
    pub fn reload(&self) -> Result<()> {
        self.reader
            .reload()
            .map_err(|e| crate::Error::Search(format!("reload: {e}")))
    }

    /// Document and segment counts for the current searcher.
    ///
    /// # Errors
    ///
    /// Returns [`enum@crate::Error`] if the searcher cannot be acquired.
    /// (Currently infallible, but the `Result` is kept for API stability.)
    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        Ok(IndexStats {
            num_docs: usize::try_from(searcher.num_docs()).unwrap_or(usize::MAX),
            num_segments: searcher.segment_readers().len(),
        })
    }
}

/// Lightweight snapshot of index size, returned by
/// [`CourseIndex::stats`] / [`SearchEngine::stats`](super::SearchEngine::stats).
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Number of live (non-deleted) documents.
    pub num_docs: usize,
    /// Number of on-disk segments.
    pub num_segments: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A freshly-created index reports zero documents.
    #[test]
    fn empty_index_has_zero_docs() {
        let dir = TempDir::new().unwrap();
        let idx = CourseIndex::create(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }

    /// `create_or_open` creates then re-opens the same directory cleanly.
    #[test]
    fn create_or_open_round_trips() {
        let dir = TempDir::new().unwrap();
        let _ = CourseIndex::create_or_open(dir.path()).unwrap();
        // second call opens existing index
        let idx = CourseIndex::create_or_open(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }
}
