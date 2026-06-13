//! Tantivy index schema + lifecycle for Place records.
//!
//! Field-set: `id` (stored STRING), `name`, `alternate_name`, `keywords`,
//! `locality`, `identifiers` (TEXT), and `gln` (STRING, exact-match).

use std::path::Path;

use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy,
    schema::{Field, STORED, STRING, Schema, TEXT},
};

use crate::Result;

/// The Tantivy [`Schema`] plus typed handles to each [`Field`].
#[derive(Clone)]
pub struct PlaceIndexSchema {
    /// The built Tantivy schema.
    pub schema: Schema,
    /// Stored place UUID (STRING) — the retrievable primary key.
    pub id: Field,
    /// Place name (TEXT) — fuzzy/full-text searchable.
    pub name: Field,
    /// Alternate name (TEXT).
    pub alternate_name: Field,
    /// Joined keyword tokens (TEXT).
    pub keywords: Field,
    /// Address locality (TEXT).
    pub locality: Field,
    /// Joined identifier values (TEXT).
    pub identifiers: Field,
    /// Global Location Number (STRING) — exact-match filter.
    pub gln: Field,
}

impl PlaceIndexSchema {
    /// Build the schema, registering every field with its index options.
    pub fn new() -> Self {
        let mut b = Schema::builder();
        let id = b.add_text_field("id", STRING | STORED);
        let name = b.add_text_field("name", TEXT | STORED);
        let alternate_name = b.add_text_field("alternate_name", TEXT | STORED);
        let keywords = b.add_text_field("keywords", TEXT | STORED);
        let locality = b.add_text_field("locality", TEXT | STORED);
        let identifiers = b.add_text_field("identifiers", TEXT | STORED);
        let gln = b.add_text_field("gln", STRING | STORED);
        let schema = b.build();
        Self {
            schema,
            id,
            name,
            alternate_name,
            keywords,
            locality,
            identifiers,
            gln,
        }
    }
}

impl Default for PlaceIndexSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// An open Tantivy index together with its schema and a live reader.
pub struct PlaceIndex {
    /// The underlying Tantivy index.
    index: Index,
    /// Cached schema + field handles.
    schema: PlaceIndexSchema,
    /// Reader configured to reload on commit.
    reader: IndexReader,
}

impl PlaceIndex {
    /// Create a brand-new index in an empty directory at `path`.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = PlaceIndexSchema::new();
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
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = PlaceIndexSchema::new();
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

    /// Open the index if a `meta.json` already exists, otherwise create.
    pub fn create_or_open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        if p.join("meta.json").exists() {
            Self::open(p)
        } else {
            Self::create(p)
        }
    }

    /// Acquire a writer with a `heap_mb`-megabyte budget.
    pub fn writer(&self, heap_mb: usize) -> Result<IndexWriter> {
        self.index
            .writer(heap_mb * 1_000_000)
            .map_err(|e| crate::Error::Search(format!("create writer: {e}")))
    }

    /// Borrow the underlying Tantivy index (for query-parser setup).
    pub fn index(&self) -> &Index {
        &self.index
    }
    /// Borrow the schema + field handles.
    pub fn schema(&self) -> &PlaceIndexSchema {
        &self.schema
    }
    /// Borrow the live reader.
    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }

    /// Force the reader to pick up the latest committed segments.
    pub fn reload(&self) -> Result<()> {
        self.reader
            .reload()
            .map_err(|e| crate::Error::Search(format!("reload: {e}")))
    }

    /// Document and segment counts for the current searcher.
    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        Ok(IndexStats {
            num_docs: searcher.num_docs() as usize,
            num_segments: searcher.segment_readers().len(),
        })
    }
}

/// Lightweight snapshot of index size.
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
        let idx = PlaceIndex::create(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }

    /// `create_or_open` creates then re-opens the same directory cleanly.
    #[test]
    fn create_or_open_round_trips() {
        let dir = TempDir::new().unwrap();
        let _ = PlaceIndex::create_or_open(dir.path()).unwrap();
        let idx = PlaceIndex::create_or_open(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }
}
