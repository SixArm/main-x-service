//! Tantivy index schema + lifecycle for plan records.
//!
//! Field set: `pid` (stored — the service's public id, the only value
//! read back out), `name`, `alternate_names`, `name_phonetic`,
//! `identifiers`, `keywords`, `tags`, `goals`, `owner_org_name`
//! (full-text), plus `code`, `owner_org_id`, `kind`, `status` and
//! `active` (exact-match).
//!
//! STRING fields are indexed verbatim so the duplicate detector can
//! filter on an exact value; TEXT fields are tokenised so fuzzy and
//! multi-token queries work over them.
//!
//! `kind` is indexed as an exact-match field so a caller **may** narrow
//! a search to one kind (`?kind=project`) — it is deliberately **not**
//! applied as a duplicate-detection gate: the matcher and this service
//! are kind-agnostic (`project_portfolio_management_matcher`'s "no kind
//! gate" rule — two plans with different `kind` labels may still be the
//! same identity), so [`super::SearchEngine::candidates`] never filters
//! on it.
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
pub struct PlanIndexSchema {
    /// The built Tantivy schema.
    pub schema: Schema,
    /// Stored plan public id (STRING) — the retrievable key.
    pub pid: Field,
    /// The plan name (TEXT) — fuzzy/full-text searchable.
    pub name: Field,
    /// Alternative names / former titles / codenames, space-joined (TEXT).
    pub alternate_names: Field,
    /// Soundex codes of every name token (TEXT) — phonetic search.
    pub name_phonetic: Field,
    /// Identifier values, space-joined (TEXT) — so a caller can search
    /// by Jira key / Asana GID / URI without knowing the scheme.
    pub identifiers: Field,
    /// Descriptive keywords, space-joined (TEXT).
    pub keywords: Field,
    /// Operator-applied tags, space-joined (TEXT).
    pub tags: Field,
    /// Goal titles, space-joined (TEXT) — the defining attribute of a
    /// plan, so searchable by what it is trying to achieve.
    pub goals: Field,
    /// Owning organisation's display name (TEXT).
    pub owner_org_name: Field,
    /// Owner-scoped code (STRING) — exact-match filter; the value is
    /// owner-scoped, matched alongside `owner_org_id`.
    pub code: Field,
    /// Owning organisation id (STRING) — exact-match filter.
    pub owner_org_id: Field,
    /// Descriptive kind label, lowercased (STRING) — an optional exact
    /// filter on search, never a duplicate-detection gate (see the
    /// module doc).
    pub kind: Field,
    /// Lifecycle status (STRING) — exact-match filter.
    pub status: Field,
    /// Active flag as `"true"`/`"false"` (STRING, FAST).
    pub active: Field,
}

impl PlanIndexSchema {
    /// Build the schema, registering every field with its index options
    /// (STRING for exact-match fields, TEXT for full-text fields).
    #[must_use]
    pub fn new() -> Self {
        let mut b = Schema::builder();
        let pid = b.add_text_field("pid", STRING | STORED);
        let name = b.add_text_field("name", TEXT);
        let alternate_names = b.add_text_field("alternate_names", TEXT);
        let name_phonetic = b.add_text_field("name_phonetic", TEXT);
        let identifiers = b.add_text_field("identifiers", TEXT);
        let keywords = b.add_text_field("keywords", TEXT);
        let tags = b.add_text_field("tags", TEXT);
        let goals = b.add_text_field("goals", TEXT);
        let owner_org_name = b.add_text_field("owner_org_name", TEXT);
        let code = b.add_text_field("code", STRING);
        let owner_org_id = b.add_text_field("owner_org_id", STRING);
        let kind = b.add_text_field("kind", STRING);
        let status = b.add_text_field("status", STRING);
        let active = b.add_text_field("active", STRING | FAST);
        let schema = b.build();
        Self {
            schema,
            pid,
            name,
            alternate_names,
            name_phonetic,
            identifiers,
            keywords,
            tags,
            goals,
            owner_org_name,
            code,
            owner_org_id,
            kind,
            status,
            active,
        }
    }
}

impl Default for PlanIndexSchema {
    /// Same as [`PlanIndexSchema::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// An open Tantivy index together with its schema and a live reader.
pub struct PlanIndex {
    /// The underlying Tantivy index.
    index: Index,
    /// Cached schema + field handles.
    schema: PlanIndexSchema,
    /// Reader configured to reload on commit.
    reader: IndexReader,
}

impl PlanIndex {
    /// Create a brand-new index in an empty directory at `path`.
    ///
    /// # Errors
    ///
    /// When Tantivy cannot create the index or build a reader over it.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = PlanIndexSchema::new();
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
        let schema = PlanIndexSchema::new();
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
    fn with_reader(index: Index, schema: PlanIndexSchema) -> Result<Self> {
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
    pub fn schema(&self) -> &PlanIndexSchema {
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

/// Lightweight snapshot of index size, returned by [`PlanIndex::stats`]
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
        let idx = PlanIndex::create(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }

    /// `create_or_open` creates, then re-opens the same directory.
    #[test]
    fn create_or_open_round_trips() {
        let dir = TempDir::new().unwrap();
        let _ = PlanIndex::create_or_open(dir.path()).unwrap();
        let idx = PlanIndex::create_or_open(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }
}
