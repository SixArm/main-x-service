//! Search index management with Tantivy

use tantivy::{
    schema::{Schema, Field, STORED, TEXT, STRING, FAST},
    Index, IndexWriter, IndexReader, ReloadPolicy,
};
use std::path::Path;

use crate::Result;

/// Fields in the worker search index
#[derive(Clone)]
pub struct WorkerIndexSchema {
    pub schema: Schema,
    pub id: Field,
    pub family_name: Field,
    pub given_names: Field,
    pub full_name: Field,
    pub birth_date: Field,
    pub gender: Field,
    pub postal_code: Field,
    pub city: Field,
    pub state: Field,
    pub identifiers: Field,
    pub worker_type: Field,
    pub active: Field,
}

impl WorkerIndexSchema {
    /// Create the worker index schema
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
    fn default() -> Self {
        Self::new()
    }
}

/// Worker search index
pub struct WorkerIndex {
    index: Index,
    schema: WorkerIndexSchema,
    reader: IndexReader,
}

impl WorkerIndex {
    /// Create a new index at the given path
    pub fn create<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let schema_def = WorkerIndexSchema::new();
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

    /// Open an existing index at the given path
    pub fn open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let schema_def = WorkerIndexSchema::new();
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

    /// Create or open an index
    pub fn create_or_open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let path = index_path.as_ref();
        let meta_path = path.join("meta.json");

        if meta_path.exists() {
            Self::open(index_path)
        } else {
            Self::create(index_path)
        }
    }

    /// Get an index writer
    pub fn writer(&self, heap_size_mb: usize) -> Result<IndexWriter> {
        self.index
            .writer(heap_size_mb * 1_000_000)
            .map_err(|e| crate::Error::Search(format!("Failed to create writer: {}", e)))
    }

    /// Get the index
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Get the schema
    pub fn schema(&self) -> &WorkerIndexSchema {
        &self.schema
    }

    /// Get the reader
    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }

    /// Manually reload the reader (useful for tests)
    pub fn reload(&self) -> Result<()> {
        self.reader.reload()
            .map_err(|e| crate::Error::Search(format!("Failed to reload reader: {}", e)))
    }

    /// Get index statistics
    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        let num_docs = searcher.num_docs() as usize;
        let num_segments = searcher.segment_readers().len();

        Ok(IndexStats {
            num_docs,
            num_segments,
        })
    }

    /// Optimize the index (wait for merges to complete)
    pub fn optimize(&self) -> Result<()> {
        let writer = self.writer(50)?;
        writer
            .wait_merging_threads()
            .map_err(|e| crate::Error::Search(format!("Failed to optimize index: {}", e)))?;
        Ok(())
    }
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub num_docs: usize,
    pub num_segments: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_index() {
        let temp_dir = TempDir::new().unwrap();
        let index = WorkerIndex::create(temp_dir.path()).unwrap();

        let stats = index.stats().unwrap();
        assert_eq!(stats.num_docs, 0);
    }

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
        doc.add_text(schema.id, &uuid::Uuid::new_v4().to_string());
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
        assert_eq!(top_docs.len(), 1, "Fuzzy search should find 'johnson' with typo 'jonson'");
    }

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
        assert_eq!(worker_index.stats().unwrap().num_docs, 0, "Document should be deleted");
    }

    #[test]
    fn test_search_no_results() {
        use tantivy::collector::TopDocs;
        use tantivy::query::TermQuery;
        use tantivy::schema::{Term, IndexRecordOption};

        let temp_dir = TempDir::new().unwrap();
        let worker_index = WorkerIndex::create(temp_dir.path()).unwrap();
        let schema = worker_index.schema();

        // Don't add any documents, search should return nothing
        let searcher = worker_index.reader().searcher();
        let term = Term::from_field_text(schema.family_name, "nonexistent");
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();
        assert_eq!(top_docs.len(), 0, "Search on empty index should return 0 results");
    }

    #[test]
    fn test_search_by_name_and_year_filter() {
        use tantivy::collector::TopDocs;
        use tantivy::query::{BooleanQuery, TermQuery};
        use tantivy::schema::{Term, IndexRecordOption};

        let temp_dir = TempDir::new().unwrap();
        let worker_index = WorkerIndex::create(temp_dir.path()).unwrap();
        let schema = worker_index.schema();

        let mut writer = worker_index.writer(50).unwrap();

        // Add two workers: same name, different birth years
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
        assert_eq!(top_docs.len(), 1, "Should find exactly 1 worker with matching name+DOB");
    }
}
