//! `Tantivy` index schema + lifecycle for Thing records.
//!
//! Field-set: `id` (stored STRING), `name`, `alternate_names`,
//! `description`, `identifiers` (TEXT).

use std::path::Path;

use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy,
    schema::{Field, STORED, STRING, Schema, TEXT},
};

use crate::Result;

/// The `Tantivy` [`Schema`] plus typed handles to each [`Field`].
#[derive(Clone)]
pub struct ThingIndexSchema {
    /// The built `Tantivy` schema.
    pub schema: Schema,
    /// Stored thing UUID (STRING) — the retrievable primary key.
    pub id: Field,
    /// Thing name (TEXT).
    pub name: Field,
    /// Alternate names (TEXT).
    pub alternate_names: Field,
    /// Description (TEXT).
    pub description: Field,
    /// Joined identifier values (TEXT).
    pub identifiers: Field,
}

impl ThingIndexSchema {
    /// Build the schema, registering every field with its index options.
    #[must_use]
    pub fn new() -> Self {
        let mut b = Schema::builder();
        let id = b.add_text_field("id", STRING | STORED);
        let name = b.add_text_field("name", TEXT | STORED);
        let alternate_names = b.add_text_field("alternate_names", TEXT | STORED);
        let description = b.add_text_field("description", TEXT | STORED);
        let identifiers = b.add_text_field("identifiers", TEXT | STORED);
        let schema = b.build();
        Self {
            schema,
            id,
            name,
            alternate_names,
            description,
            identifiers,
        }
    }
}

impl Default for ThingIndexSchema {
    /// Equivalent to [`ThingIndexSchema::new`]; provided so the schema can be
    /// built with `Default::default()` in generic contexts.
    fn default() -> Self {
        Self::new()
    }
}

/// An open `Tantivy` index together with its schema and a live reader.
pub struct ThingIndex {
    /// The underlying `Tantivy` index.
    index: Index,
    /// Cached schema + field handles.
    schema: ThingIndexSchema,
    /// Reader configured to reload on commit.
    reader: IndexReader,
}

impl ThingIndex {
    /// Create a brand-new index in an empty directory at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the index or its reader cannot be created.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = ThingIndexSchema::new();
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
    /// Returns an error if the index or its reader cannot be opened.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = ThingIndexSchema::new();
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
    ///
    /// # Errors
    ///
    /// Returns an error if the index cannot be opened or created.
    pub fn create_or_open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        if p.join("meta.json").exists() {
            Self::open(p)
        } else {
            Self::create(p)
        }
    }

    /// Acquire a writer with a `heap_mb`-megabyte budget.
    ///
    /// # Errors
    ///
    /// Returns an error if the writer cannot be created.
    pub fn writer(&self, heap_mb: usize) -> Result<IndexWriter> {
        self.index
            .writer(heap_mb * 1_000_000)
            .map_err(|e| crate::Error::Search(format!("create writer: {e}")))
    }

    /// Borrow the underlying `Tantivy` index (for query-parser setup).
    #[must_use]
    pub fn index(&self) -> &Index {
        &self.index
    }
    /// Borrow the schema + field handles.
    #[must_use]
    pub fn schema(&self) -> &ThingIndexSchema {
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
    /// Returns an error if the reader fails to reload.
    pub fn reload(&self) -> Result<()> {
        self.reader
            .reload()
            .map_err(|e| crate::Error::Search(format!("reload: {e}")))
    }

    /// Document and segment counts for the current searcher.
    ///
    /// # Errors
    ///
    /// Returns an error if the searcher cannot read index statistics.
    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        Ok(IndexStats {
            num_docs: usize::try_from(searcher.num_docs()).unwrap_or(usize::MAX),
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
        let idx = ThingIndex::create(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }

    /// `create_or_open` creates then re-opens the same directory cleanly.
    #[test]
    fn create_or_open_round_trips() {
        let dir = TempDir::new().unwrap();
        let _ = ThingIndex::create_or_open(dir.path()).unwrap();
        let idx = ThingIndex::create_or_open(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }
}
