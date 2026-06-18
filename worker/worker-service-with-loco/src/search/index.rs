//! Tantivy index lifecycle and schema for the worker search engine.
//!
//! Defines [`WorkerIndexSchema`] (the field set and their Tantivy options),
//! [`WorkerIndex`] (the index, schema, and reader bundled together with
//! create/open/writer/reader/stats helpers), and [`IndexStats`] (a small
//! count summary). [`crate::search::SearchEngine`] sits on top of this module
//! and is the type application code normally uses; this module owns the
//! lower-level Tantivy plumbing.

use std::path::Path;
use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy,
    schema::{FAST, Field, STORED, STRING, Schema, TEXT},
};

use crate::Result;

/// The worker search index schema: the built [`Schema`] plus a [`Field`]
/// handle for each indexed column.
///
/// Holding the field handles alongside the schema avoids repeated name
/// look-ups when building documents and queries. `TEXT` fields are tokenized
/// for full-text search; `STRING` fields are stored verbatim for exact-match
/// filtering; `STORED` fields are retrievable from hits; `FAST` fields support
/// fast filtering.
#[derive(Clone)]
pub struct WorkerIndexSchema {
    /// The compiled Tantivy schema describing all fields below.
    pub schema: Schema,
    /// Worker UUID string — stored, exact-match only (not tokenized).
    pub id: Field,
    /// Family/last name — tokenized full-text and stored.
    pub family_name: Field,
    /// Space-joined given names — tokenized full-text and stored.
    pub given_names: Field,
    /// "Given Family" combined name — tokenized full-text and stored.
    pub full_name: Field,
    /// Birth date as an ISO string — exact-match and stored.
    pub birth_date: Field,
    /// Lower-cased gender label — exact-match and stored.
    pub gender: Field,
    /// Primary-address postal code — exact-match and stored.
    pub postal_code: Field,
    /// Primary-address city — tokenized full-text and stored.
    pub city: Field,
    /// Primary-address state/region — exact-match and stored.
    pub state: Field,
    /// Space-joined `TYPE:value` identifier tokens — tokenized and stored.
    pub identifiers: Field,
    /// Worker type/role — exact-match and stored (for role filtering).
    pub worker_type: Field,
    /// Active flag ("true"/"false") — exact-match, FAST for filtering.
    pub active: Field,
}

impl WorkerIndexSchema {
    /// Builds the worker index schema, registering every field with its
    /// Tantivy options and capturing the resulting [`Field`] handles.
    #[must_use]
    pub fn new() -> Self {
        let mut schema_builder = Schema::builder();

        // ID field (stored, not indexed for search)
        let id = schema_builder.add_text_field("id", STRING | STORED);

        // Name fields (indexed and stored)
        let family_name = schema_builder.add_text_field("family_name", TEXT | STORED);
        let given_names = schema_builder.add_text_field("given_names", TEXT | STORED);
        let full_name = schema_builder.add_text_field("full_name", TEXT | STORED);

        // Demographics (indexed and stored)
        let birth_date = schema_builder.add_text_field("birth_date", STRING | STORED);
        let gender = schema_builder.add_text_field("gender", STRING | STORED);

        // Address fields (indexed and stored)
        let postal_code = schema_builder.add_text_field("postal_code", STRING | STORED);
        let city = schema_builder.add_text_field("city", TEXT | STORED);
        let state = schema_builder.add_text_field("state", STRING | STORED);

        // Identifiers (indexed and stored)
        let identifiers = schema_builder.add_text_field("identifiers", TEXT | STORED);

        // Worker type (indexed and stored, for filtering by role)
        let worker_type = schema_builder.add_text_field("worker_type", STRING | STORED);

        // Active status (for filtering)
        let active = schema_builder.add_text_field("active", STRING | FAST);

        let schema = schema_builder.build();

        Self {
            schema,
            id,
            family_name,
            given_names,
            full_name,
            birth_date,
            gender,
            postal_code,
            city,
            state,
            identifiers,
            worker_type,
            active,
        }
    }
}

impl Default for WorkerIndexSchema {
    /// Equivalent to [`WorkerIndexSchema::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// A worker search index: the Tantivy [`Index`], its [`WorkerIndexSchema`],
/// and a long-lived [`IndexReader`].
///
/// The reader reloads on commit (with a small delay), so searches see new
/// documents shortly after a writer commits; call [`reload`](Self::reload) to
/// force visibility immediately (used in tests). Writers are created on demand
/// per write via [`writer`](Self::writer).
pub struct WorkerIndex {
    /// The underlying Tantivy index (segments on disk).
    index: Index,
    /// Field handles and compiled schema for this index.
    schema: WorkerIndexSchema,
    /// Shared reader that serves searchers; reloads on commit.
    reader: IndexReader,
}

impl WorkerIndex {
    /// Creates a brand-new index in `index_path` (which must not already hold
    /// one) using the worker schema, and builds a commit-reloading reader.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Search`] if the directory already holds an
    /// index, cannot be written, or the reader cannot be built.
    pub fn create<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let schema_def = WorkerIndexSchema::new();
        let index = Index::create_in_dir(index_path, schema_def.schema.clone())
            .map_err(|e| crate::Error::Search(format!("Failed to create index: {e}")))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| crate::Error::Search(format!("Failed to create reader: {e}")))?;

        Ok(Self {
            index,
            schema: schema_def,
            reader,
        })
    }

    /// Opens an existing index in `index_path`, reconstructing the field
    /// handles and a commit-reloading reader. Errors if no index is present.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Search`] if no index is present at the path or
    /// the reader cannot be built.
    pub fn open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let schema_def = WorkerIndexSchema::new();
        let index = Index::open_in_dir(index_path)
            .map_err(|e| crate::Error::Search(format!("Failed to open index: {e}")))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| crate::Error::Search(format!("Failed to create reader: {e}")))?;

        Ok(Self {
            index,
            schema: schema_def,
            reader,
        })
    }

    /// Opens the index at `index_path` if one exists there, otherwise creates
    /// it. Existence is detected by the presence of Tantivy's `meta.json`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Search`] if the existing index cannot be opened
    /// or a new one cannot be created.
    pub fn create_or_open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let path = index_path.as_ref();
        // Tantivy writes a meta.json into the index dir; its presence means an
        // index already exists here.
        let meta_path = path.join("meta.json");

        if meta_path.exists() {
            Self::open(index_path)
        } else {
            Self::create(index_path)
        }
    }

    /// Creates a new [`IndexWriter`] with a buffer of `heap_size_mb` megabytes.
    /// Callers are responsible for committing and dropping the writer.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Search`] if the writer cannot be allocated (for
    /// example if another writer already holds the index lock).
    pub fn writer(&self, heap_size_mb: usize) -> Result<IndexWriter> {
        // Tantivy's writer heap is specified in bytes (megabytes * 1e6).
        self.index
            .writer(heap_size_mb * 1_000_000)
            .map_err(|e| crate::Error::Search(format!("Failed to create writer: {e}")))
    }

    /// Returns the underlying Tantivy [`Index`] (needed to build query parsers).
    #[must_use]
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Returns the [`WorkerIndexSchema`] with the field handles.
    #[must_use]
    pub fn schema(&self) -> &WorkerIndexSchema {
        &self.schema
    }

    /// Returns the shared [`IndexReader`]; call `.searcher()` on it to run a
    /// query.
    #[must_use]
    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }

    /// Forces the reader to reload so the latest committed documents become
    /// visible immediately, rather than after the on-commit delay.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Search`] if the reader fails to reload.
    pub fn reload(&self) -> Result<()> {
        self.reader
            .reload()
            .map_err(|e| crate::Error::Search(format!("Failed to reload reader: {e}")))
    }

    /// Returns the live document count and segment count for the index.
    ///
    /// # Errors
    ///
    /// Returns `Ok` in practice; the [`Result`] is kept for signature
    /// uniformity with the other index operations.
    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        // `num_docs` counts live (non-deleted) documents across all segments.
        let num_docs = usize::try_from(searcher.num_docs()).unwrap_or(usize::MAX);
        let num_segments = searcher.segment_readers().len();

        Ok(IndexStats {
            num_docs,
            num_segments,
        })
    }

    /// Waits for background segment merges to finish, consolidating the index
    /// for faster queries and reclaimed space.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Search`] if the writer cannot be created or the
    /// merge threads fail to join.
    pub fn optimize(&self) -> Result<()> {
        // Creating then immediately blocking on the writer's merge threads
        // forces pending merges to complete.
        let writer = self.writer(50)?;
        writer
            .wait_merging_threads()
            .map_err(|e| crate::Error::Search(format!("Failed to optimize index: {e}")))?;
        Ok(())
    }
}

/// A snapshot of index size: how many live documents and segments it holds.
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Number of live (non-deleted) documents.
    pub num_docs: usize,
    /// Number of on-disk segments (lower is better after optimization).
    pub num_segments: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A freshly created index starts with zero documents.
    #[test]
    fn test_create_index() {
        let temp_dir = TempDir::new().unwrap();
        let index = WorkerIndex::create(temp_dir.path()).unwrap();

        let stats = index.stats().unwrap();
        assert_eq!(stats.num_docs, 0);
    }

    /// The schema exposes the expected field handles.
    #[test]
    fn test_schema_fields() {
        let schema = WorkerIndexSchema::new();

        // Verify fields exist
        let _ = schema.id;
        let _ = schema.family_name;
        let _ = schema.given_names;
        let _ = schema.full_name;
        let _ = schema.birth_date;
        let _ = schema.gender;
    }

    /// The first call creates the index; a second call opens the same one.
    #[test]
    fn test_create_or_open() {
        let temp_dir = TempDir::new().unwrap();

        // First call creates
        let index1 = WorkerIndex::create_or_open(temp_dir.path()).unwrap();
        assert_eq!(index1.stats().unwrap().num_docs, 0);

        // Second call opens
        let index2 = WorkerIndex::create_or_open(temp_dir.path()).unwrap();
        assert_eq!(index2.stats().unwrap().num_docs, 0);
    }

    /// Adding and committing a document raises the live count to one.
    #[test]
    fn test_index_worker_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let worker_index = WorkerIndex::create(temp_dir.path()).unwrap();
        let schema = worker_index.schema();

        let mut writer = worker_index.writer(50).unwrap();
        let worker_id = uuid::Uuid::new_v4().to_string();

        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(schema.id, &worker_id);
        doc.add_text(schema.family_name, "Smith");
        doc.add_text(schema.given_names, "John");
        doc.add_text(schema.full_name, "John Smith");
        doc.add_text(schema.birth_date, "1980-01-15");
        doc.add_text(schema.gender, "male");
        doc.add_text(schema.active, "true");

        writer.add_document(doc).unwrap();
        writer.commit().unwrap();

        worker_index.reload().unwrap();

        let stats = worker_index.stats().unwrap();
        assert_eq!(stats.num_docs, 1, "Index should contain 1 document");
    }

    /// A fuzzy term query finds "johnson" from the typo "jonson".
    #[test]
    fn test_fuzzy_search_typo() {
        use tantivy::collector::TopDocs;
        use tantivy::query::FuzzyTermQuery;
        use tantivy::schema::Term;

        let temp_dir = TempDir::new().unwrap();
        let worker_index = WorkerIndex::create(temp_dir.path()).unwrap();
        let schema = worker_index.schema();

        let mut writer = worker_index.writer(50).unwrap();
        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(schema.id, uuid::Uuid::new_v4().to_string());
        doc.add_text(schema.family_name, "johnson");
        doc.add_text(schema.given_names, "robert");
        doc.add_text(schema.full_name, "robert johnson");
        doc.add_text(schema.active, "true");
        writer.add_document(doc).unwrap();
        writer.commit().unwrap();

        worker_index.reload().unwrap();

        let searcher = worker_index.reader().searcher();
        let term = Term::from_field_text(schema.family_name, "jonson"); // typo
        let query = FuzzyTermQuery::new(term, 1, true);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();
        assert_eq!(
            top_docs.len(),
            1,
            "Fuzzy search should find 'johnson' with typo 'jonson'"
        );
    }

    /// Deleting by the `id` term drops the document count back to zero.
    #[test]
    fn test_delete_worker_from_index() {
        use tantivy::schema::Term;

        let temp_dir = TempDir::new().unwrap();
        let worker_index = WorkerIndex::create(temp_dir.path()).unwrap();
        let schema = worker_index.schema();

        let worker_id = uuid::Uuid::new_v4().to_string();

        {
            let mut writer = worker_index.writer(50).unwrap();
            let mut doc = tantivy::TantivyDocument::default();
            doc.add_text(schema.id, &worker_id);
            doc.add_text(schema.family_name, "Smith");
            doc.add_text(schema.given_names, "John");
            doc.add_text(schema.full_name, "John Smith");
            doc.add_text(schema.active, "true");
            writer.add_document(doc).unwrap();
            writer.commit().unwrap();
            // writer dropped here, releasing the lock
        }

        worker_index.reload().unwrap();
        assert_eq!(worker_index.stats().unwrap().num_docs, 1);

        {
            let mut writer = worker_index.writer(50).unwrap();
            let term = Term::from_field_text(schema.id, &worker_id);
            writer.delete_term(term);
            writer.commit().unwrap();
        }

        worker_index.reload().unwrap();
        assert_eq!(
            worker_index.stats().unwrap().num_docs,
            0,
            "Document should be deleted"
        );
    }

    /// Searching an empty index returns no hits.
    #[test]
    fn test_search_no_results() {
        use tantivy::collector::TopDocs;
        use tantivy::query::TermQuery;
        use tantivy::schema::{IndexRecordOption, Term};

        let temp_dir = TempDir::new().unwrap();
        let worker_index = WorkerIndex::create(temp_dir.path()).unwrap();
        let schema = worker_index.schema();

        // Don't add any documents, search should return nothing
        let searcher = worker_index.reader().searcher();
        let term = Term::from_field_text(schema.family_name, "nonexistent");
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();
        assert_eq!(
            top_docs.len(),
            0,
            "Search on empty index should return 0 results"
        );
    }

    /// An AND of name + exact birth date selects one of two same-name workers.
    #[test]
    fn test_search_by_name_and_year_filter() {
        use tantivy::collector::TopDocs;
        use tantivy::query::{BooleanQuery, TermQuery};
        use tantivy::schema::{IndexRecordOption, Term};

        let temp_dir = TempDir::new().unwrap();
        let worker_index = WorkerIndex::create(temp_dir.path()).unwrap();
        let schema = worker_index.schema();

        let mut writer = worker_index.writer(50).unwrap();

        // Add two workers: same name, different birth years
        for birth_date in &["1980-01-15", "1990-06-20"] {
            let mut doc = tantivy::TantivyDocument::default();
            doc.add_text(schema.id, uuid::Uuid::new_v4().to_string());
            doc.add_text(schema.family_name, "smith");
            doc.add_text(schema.given_names, "john");
            doc.add_text(schema.full_name, "john smith");
            doc.add_text(schema.birth_date, birth_date);
            doc.add_text(schema.active, "true");
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();
        worker_index.reload().unwrap();

        assert_eq!(worker_index.stats().unwrap().num_docs, 2);

        // Search filtering by exact birth_date
        let searcher = worker_index.reader().searcher();
        let name_term = Term::from_field_text(schema.family_name, "smith");
        let dob_term = Term::from_field_text(schema.birth_date, "1980-01-15");
        let query = BooleanQuery::intersection(vec![
            Box::new(TermQuery::new(name_term, IndexRecordOption::Basic)),
            Box::new(TermQuery::new(dob_term, IndexRecordOption::Basic)),
        ]);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();
        assert_eq!(
            top_docs.len(),
            1,
            "Should find exactly 1 worker with matching name+DOB"
        );
    }
}
