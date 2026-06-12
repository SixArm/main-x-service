//! Tantivy index schema, lifecycle, and statistics.
//!
//! [`PersonIndexSchema`] declares the eleven indexed fields and how each
//! is analyzed (`TEXT` = tokenized/lowercased for fuzzy name search,
//! `STRING` = verbatim for exact lookups like ID and postal code).
//! [`PersonIndex`] owns the on-disk [`Index`](tantivy::Index), a cached
//! [`PersonIndexSchema`], and a long-lived [`IndexReader`](tantivy::IndexReader); it offers
//! create/open, writer/reader accessors, manual reload, stats, and
//! optimize. [`SearchEngine`](crate::search::SearchEngine) is the higher-level
//! wrapper most code should use.

use std::path::Path;
use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy,
    schema::{FAST, Field, STORED, STRING, Schema, TEXT},
};

use crate::Result;

/// The person index schema plus a handle to each [`Field`].
///
/// Cloning is cheap (fields are integer handles); the underlying
/// [`Schema`] is reference-counted by Tantivy.
#[derive(Clone)]
pub struct PersonIndexSchema {
    /// The built Tantivy schema.
    pub schema: Schema,
    /// Person UUID (verbatim string, stored, exact-match only).
    pub id: Field,
    /// Family/last name (tokenized for fuzzy search).
    pub family_name: Field,
    /// Space-joined given names (tokenized).
    pub given_names: Field,
    /// Full "Given Family" name (tokenized).
    pub full_name: Field,
    /// Birth date as `YYYY-MM-DD` (verbatim string).
    pub birth_date: Field,
    /// Lowercased gender label (verbatim string).
    pub gender: Field,
    /// Primary-address postal code (verbatim string).
    pub postal_code: Field,
    /// Primary-address city (tokenized).
    pub city: Field,
    /// Primary-address state/region (verbatim string).
    pub state: Field,
    /// Space-joined `type:value` identifier strings (tokenized).
    pub identifiers: Field,
    /// Active flag as `"true"`/`"false"` (FAST, for filtering).
    pub active: Field,
}

impl PersonIndexSchema {
    /// Build the schema and resolve every field handle.
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
            active,
        }
    }
}

impl Default for PersonIndexSchema {
    /// Same as [`PersonIndexSchema::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// An open Tantivy person index: the index, its schema, and a reader.
pub struct PersonIndex {
    /// The on-disk Tantivy index.
    index: Index,
    /// Cached schema and field handles.
    schema: PersonIndexSchema,
    /// Long-lived reader (reloaded after writes).
    reader: IndexReader,
}

impl PersonIndex {
    /// Create a brand-new index in the (empty) directory `index_path`.
    pub fn create<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let schema_def = PersonIndexSchema::new();
        let index = Index::create_in_dir(index_path, schema_def.schema.clone())
            .map_err(|e| crate::Error::Search(format!("Failed to create index: {}", e)))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| crate::Error::Search(format!("Failed to create reader: {}", e)))?;

        Ok(Self {
            index,
            schema: schema_def,
            reader,
        })
    }

    /// Open an existing index already present at `index_path`.
    pub fn open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let schema_def = PersonIndexSchema::new();
        let index = Index::open_in_dir(index_path)
            .map_err(|e| crate::Error::Search(format!("Failed to open index: {}", e)))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| crate::Error::Search(format!("Failed to create reader: {}", e)))?;

        Ok(Self {
            index,
            schema: schema_def,
            reader,
        })
    }

    /// Open the index if `meta.json` exists, otherwise create it.
    pub fn create_or_open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let path = index_path.as_ref();
        let meta_path = path.join("meta.json");

        if meta_path.exists() {
            Self::open(index_path)
        } else {
            Self::create(index_path)
        }
    }

    /// Create an index writer with a `heap_size_mb` MiB write buffer.
    pub fn writer(&self, heap_size_mb: usize) -> Result<IndexWriter> {
        self.index
            .writer(heap_size_mb * 1_000_000)
            .map_err(|e| crate::Error::Search(format!("Failed to create writer: {}", e)))
    }

    /// Borrow the underlying Tantivy [`Index`] (for query parsers).
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Borrow the cached schema and field handles.
    pub fn schema(&self) -> &PersonIndexSchema {
        &self.schema
    }

    /// Borrow the shared [`IndexReader`].
    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }

    /// Force the reader to observe the latest committed segments.
    pub fn reload(&self) -> Result<()> {
        self.reader
            .reload()
            .map_err(|e| crate::Error::Search(format!("Failed to reload reader: {}", e)))
    }

    /// Return document/segment counts for the current reader view.
    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        let num_docs = searcher.num_docs() as usize;
        let num_segments = searcher.segment_readers().len();

        Ok(IndexStats {
            num_docs,
            num_segments,
        })
    }

    /// Optimize the index by waiting for pending background merges.
    ///
    /// Opens a short-lived writer and blocks on
    /// [`wait_merging_threads`](IndexWriter::wait_merging_threads), so on
    /// return segment merges are settled and the on-disk layout is
    /// compact. Useful after a large bulk index before serving queries.
    pub fn optimize(&self) -> Result<()> {
        let writer = self.writer(50)?;
        writer
            .wait_merging_threads()
            .map_err(|e| crate::Error::Search(format!("Failed to optimize index: {}", e)))?;
        Ok(())
    }
}

/// A point-in-time snapshot of index size, returned by
/// [`PersonIndex::stats`].
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Number of live (non-deleted) documents visible to the reader.
    pub num_docs: usize,
    /// Number of on-disk segments backing the current reader view.
    pub num_segments: usize,
}

#[cfg(test)]
mod tests {
    //! Unit tests for the Tantivy schema, index lifecycle, and the
    //! low-level query primitives the [`SearchEngine`](super::super::SearchEngine)
    //! builds on. Each test uses a fresh [`TempDir`] so runs are isolated.
    use super::*;
    use tempfile::TempDir;

    /// A freshly created index has zero documents.
    #[test]
    fn test_create_index() {
        let temp_dir = TempDir::new().unwrap();
        let index = PersonIndex::create(temp_dir.path()).unwrap();

        let stats = index.stats().unwrap();
        assert_eq!(stats.num_docs, 0);
    }

    /// Every declared field handle resolves on a freshly built schema.
    #[test]
    fn test_schema_fields() {
        let schema = PersonIndexSchema::new();

        // Verify fields exist
        let _ = schema.id;
        let _ = schema.family_name;
        let _ = schema.given_names;
        let _ = schema.full_name;
        let _ = schema.birth_date;
        let _ = schema.gender;
    }

    /// `create_or_open` creates on first call and re-opens on the second,
    /// both yielding a usable index over the same directory.
    #[test]
    fn test_create_or_open() {
        let temp_dir = TempDir::new().unwrap();

        // First call creates
        let index1 = PersonIndex::create_or_open(temp_dir.path()).unwrap();
        assert_eq!(index1.stats().unwrap().num_docs, 0);

        // Second call opens
        let index2 = PersonIndex::create_or_open(temp_dir.path()).unwrap();
        assert_eq!(index2.stats().unwrap().num_docs, 0);
    }

    /// After adding and committing a document, a reload makes it visible
    /// in the document count.
    #[test]
    fn test_index_person_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let person_index = PersonIndex::create(temp_dir.path()).unwrap();
        let schema = person_index.schema();

        let mut writer = person_index.writer(50).unwrap();
        let person_id = uuid::Uuid::new_v4().to_string();

        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(schema.id, &person_id);
        doc.add_text(schema.family_name, "Smith");
        doc.add_text(schema.given_names, "John");
        doc.add_text(schema.full_name, "John Smith");
        doc.add_text(schema.birth_date, "1980-01-15");
        doc.add_text(schema.gender, "male");
        doc.add_text(schema.active, "true");

        writer.add_document(doc).unwrap();
        writer.commit().unwrap();

        person_index.reload().unwrap();

        let stats = person_index.stats().unwrap();
        assert_eq!(stats.num_docs, 1, "Index should contain 1 document");
    }

    /// A fuzzy term query with edit distance 1 matches "johnson" given
    /// the typo "jonson".
    #[test]
    fn test_fuzzy_search_typo() {
        use tantivy::collector::TopDocs;
        use tantivy::query::FuzzyTermQuery;
        use tantivy::schema::Term;

        let temp_dir = TempDir::new().unwrap();
        let person_index = PersonIndex::create(temp_dir.path()).unwrap();
        let schema = person_index.schema();

        let mut writer = person_index.writer(50).unwrap();
        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(schema.id, &uuid::Uuid::new_v4().to_string());
        doc.add_text(schema.family_name, "johnson");
        doc.add_text(schema.given_names, "robert");
        doc.add_text(schema.full_name, "robert johnson");
        doc.add_text(schema.active, "true");
        writer.add_document(doc).unwrap();
        writer.commit().unwrap();

        person_index.reload().unwrap();

        let searcher = person_index.reader().searcher();
        let term = Term::from_field_text(schema.family_name, "jonson"); // typo
        let query = FuzzyTermQuery::new(term, 1, true);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();
        assert_eq!(
            top_docs.len(),
            1,
            "Fuzzy search should find 'johnson' with typo 'jonson'"
        );
    }

    /// Deleting by the `id` term and committing removes the document,
    /// dropping the live count back to zero after a reload.
    #[test]
    fn test_delete_person_from_index() {
        use tantivy::schema::Term;

        let temp_dir = TempDir::new().unwrap();
        let person_index = PersonIndex::create(temp_dir.path()).unwrap();
        let schema = person_index.schema();

        let person_id = uuid::Uuid::new_v4().to_string();

        {
            let mut writer = person_index.writer(50).unwrap();
            let mut doc = tantivy::TantivyDocument::default();
            doc.add_text(schema.id, &person_id);
            doc.add_text(schema.family_name, "Smith");
            doc.add_text(schema.given_names, "John");
            doc.add_text(schema.full_name, "John Smith");
            doc.add_text(schema.active, "true");
            writer.add_document(doc).unwrap();
            writer.commit().unwrap();
            // writer dropped here, releasing the lock
        }

        person_index.reload().unwrap();
        assert_eq!(person_index.stats().unwrap().num_docs, 1);

        {
            let mut writer = person_index.writer(50).unwrap();
            let term = Term::from_field_text(schema.id, &person_id);
            writer.delete_term(term);
            writer.commit().unwrap();
        }

        person_index.reload().unwrap();
        assert_eq!(
            person_index.stats().unwrap().num_docs,
            0,
            "Document should be deleted"
        );
    }

    /// A term query against an empty index returns no hits.
    #[test]
    fn test_search_no_results() {
        use tantivy::collector::TopDocs;
        use tantivy::query::TermQuery;
        use tantivy::schema::{IndexRecordOption, Term};

        let temp_dir = TempDir::new().unwrap();
        let person_index = PersonIndex::create(temp_dir.path()).unwrap();
        let schema = person_index.schema();

        // Don't add any documents, search should return nothing
        let searcher = person_index.reader().searcher();
        let term = Term::from_field_text(schema.family_name, "nonexistent");
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();
        assert_eq!(
            top_docs.len(),
            0,
            "Search on empty index should return 0 results"
        );
    }

    /// An intersection of name + exact birth-date terms isolates one of
    /// two same-name records, pinning the name+year filter behavior.
    #[test]
    fn test_search_by_name_and_year_filter() {
        use tantivy::collector::TopDocs;
        use tantivy::query::{BooleanQuery, TermQuery};
        use tantivy::schema::{IndexRecordOption, Term};

        let temp_dir = TempDir::new().unwrap();
        let person_index = PersonIndex::create(temp_dir.path()).unwrap();
        let schema = person_index.schema();

        let mut writer = person_index.writer(50).unwrap();

        // Add two persons: same name, different birth years
        for birth_date in &["1980-01-15", "1990-06-20"] {
            let mut doc = tantivy::TantivyDocument::default();
            doc.add_text(schema.id, &uuid::Uuid::new_v4().to_string());
            doc.add_text(schema.family_name, "smith");
            doc.add_text(schema.given_names, "john");
            doc.add_text(schema.full_name, "john smith");
            doc.add_text(schema.birth_date, birth_date);
            doc.add_text(schema.active, "true");
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();
        person_index.reload().unwrap();

        assert_eq!(person_index.stats().unwrap().num_docs, 2);

        // Search filtering by exact birth_date
        let searcher = person_index.reader().searcher();
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
            "Should find exactly 1 person with matching name+DOB"
        );
    }
}
