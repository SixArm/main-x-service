//! Tantivy index schema + lifecycle for case records.
//!
//! Field set: `pid` (stored — the service's public id, the only value
//! read back out), `title`, `alternate_titles`, `title_phonetic`,
//! `identifiers`, `keywords`, `subjects`, `agency_name` (full-text),
//! plus `case_number`, `agency_id`, `case_type`, `status` and `active`
//! (exact-match).
//!
//! STRING fields are indexed verbatim so the duplicate detector can
//! filter on an exact value; TEXT fields are tokenised so fuzzy and
//! multi-token queries work over them.
//!
//! Only `pid` is `STORED`: every hit is resolved against Postgres
//! anyway (the database is the source of truth, the index is a
//! candidate generator), so storing the rest would duplicate the
//! payload for no reader.

use std::path::Path;

use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy,
    schema::{FAST, Field, STORED, STRING, Schema, TEXT},
};

use super::{Result, err};

/// The Tantivy [`Schema`] plus typed handles to each [`Field`], built
/// once and reused for every index/query operation.
#[derive(Clone)]
pub struct CaseIndexSchema {
    /// The built Tantivy schema.
    pub schema: Schema,
    /// Stored case public id (STRING) — the retrievable key.
    pub pid: Field,
    /// The case title (TEXT) — fuzzy/full-text searchable.
    pub title: Field,
    /// Alternative titles, space-joined (TEXT).
    pub alternate_titles: Field,
    /// Soundex codes of every title token (TEXT) — phonetic search.
    pub title_phonetic: Field,
    /// Identifier values, space-joined (TEXT) — so a caller can search
    /// by docket number / external case id / URI without knowing the
    /// scheme.
    pub identifiers: Field,
    /// Descriptive keywords, space-joined (TEXT).
    pub keywords: Field,
    /// Involved-party (subject) identifiers, space-joined (TEXT) — the
    /// defining attribute of a case, so an operator can search by who
    /// it is about.
    pub subjects: Field,
    /// Handling agency's name (TEXT).
    pub agency_name: Field,
    /// Agency's local case number (STRING) — exact-match filter; the
    /// value is agency-scoped, matched alongside `agency_id`.
    pub case_number: Field,
    /// Handling agency id (STRING) — exact-match filter.
    pub agency_id: Field,
    /// Case type (STRING) — exact-match filter.
    pub case_type: Field,
    /// Case lifecycle status (STRING) — exact-match filter.
    pub status: Field,
    /// Active flag as `"true"`/`"false"` (STRING, FAST).
    pub active: Field,
}

impl CaseIndexSchema {
    /// Build the schema, registering every field with its index options
    /// (STRING for exact-match fields, TEXT for full-text fields).
    #[must_use]
    pub fn new() -> Self {
        let mut b = Schema::builder();
        let pid = b.add_text_field("pid", STRING | STORED);
        let title = b.add_text_field("title", TEXT);
        let alternate_titles = b.add_text_field("alternate_titles", TEXT);
        let title_phonetic = b.add_text_field("title_phonetic", TEXT);
        let identifiers = b.add_text_field("identifiers", TEXT);
        let keywords = b.add_text_field("keywords", TEXT);
        let subjects = b.add_text_field("subjects", TEXT);
        let agency_name = b.add_text_field("agency_name", TEXT);
        let case_number = b.add_text_field("case_number", STRING);
        let agency_id = b.add_text_field("agency_id", STRING);
        let case_type = b.add_text_field("case_type", STRING);
        let status = b.add_text_field("status", STRING);
        let active = b.add_text_field("active", STRING | FAST);
        let schema = b.build();
        Self {
            schema,
            pid,
            title,
            alternate_titles,
            title_phonetic,
            identifiers,
            keywords,
            subjects,
            agency_name,
            case_number,
            agency_id,
            case_type,
            status,
            active,
        }
    }
}

impl Default for CaseIndexSchema {
    /// Same as [`CaseIndexSchema::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// An open Tantivy index together with its schema and a live reader.
pub struct CaseIndex {
    /// The underlying Tantivy index.
    index: Index,
    /// Cached schema + field handles.
    schema: CaseIndexSchema,
    /// Reader configured to reload on commit.
    reader: IndexReader,
}

impl CaseIndex {
    /// Create a brand-new index in an empty directory at `path`.
    ///
    /// # Errors
    ///
    /// When Tantivy cannot create the index or build a reader over it.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = CaseIndexSchema::new();
        let index = Index::create_in_dir(path, schema.schema.clone())
            .map_err(|e| err(&format!("create index: {e}")))?;
        Self::with_reader(index, schema)
    }

    /// Open an existing index previously created at `path`.
    ///
    /// # Errors
    ///
    /// When Tantivy cannot open the index or build a reader over it.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = CaseIndexSchema::new();
        let index = Index::open_in_dir(path).map_err(|e| err(&format!("open index: {e}")))?;
        Self::with_reader(index, schema)
    }

    /// Open the index if a `meta.json` already exists at `path`,
    /// otherwise create a fresh one. The boot-time entry point.
    ///
    /// # Errors
    ///
    /// When the underlying [`open`](Self::open) / [`create`](Self::create)
    /// call fails.
    pub fn create_or_open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        if p.join("meta.json").exists() {
            Self::open(p)
        } else {
            Self::create(p)
        }
    }

    /// Attach a commit-reloading reader to an opened index.
    fn with_reader(index: Index, schema: CaseIndexSchema) -> Result<Self> {
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| err(&format!("create reader: {e}")))?;
        Ok(Self {
            index,
            schema,
            reader,
        })
    }

    /// Acquire a writer with a `heap_mb`-megabyte in-memory buffer.
    ///
    /// # Errors
    ///
    /// When Tantivy cannot allocate the writer.
    pub fn writer(&self, heap_mb: usize) -> Result<IndexWriter> {
        self.index
            .writer(heap_mb * 1_000_000)
            .map_err(|e| err(&format!("create writer: {e}")))
    }

    /// Borrow the underlying Tantivy index (for query-parser setup).
    #[must_use]
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Borrow the schema + field handles.
    #[must_use]
    pub fn schema(&self) -> &CaseIndexSchema {
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
    /// When the reader fails to reload.
    pub fn reload(&self) -> Result<()> {
        self.reader
            .reload()
            .map_err(|e| err(&format!("reload: {e}")))
    }

    /// Document and segment counts for the current searcher.
    ///
    /// # Errors
    ///
    /// Never today; the `Result` is kept for API stability with the
    /// sibling services' index wrappers.
    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        Ok(IndexStats {
            num_docs: usize::try_from(searcher.num_docs()).unwrap_or(usize::MAX),
            num_segments: searcher.segment_readers().len(),
        })
    }
}

/// Lightweight snapshot of index size, returned by [`CaseIndex::stats`]
/// and [`SearchEngine::stats`](super::SearchEngine::stats).
#[derive(Debug, Clone, Copy)]
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
        let idx = CaseIndex::create(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }

    /// `create_or_open` creates, then re-opens the same directory.
    #[test]
    fn create_or_open_round_trips() {
        let dir = TempDir::new().unwrap();
        let _ = CaseIndex::create_or_open(dir.path()).unwrap();
        let idx = CaseIndex::create_or_open(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }
}
