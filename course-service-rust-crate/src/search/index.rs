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

#[derive(Clone)]
pub struct CourseIndexSchema {
    pub schema: Schema,
    pub id: Field,
    pub name: Field,
    pub alternate_names: Field,
    pub course_code: Field,
    pub provider_id: Field,
    pub provider_name: Field,
    pub keywords: Field,
    pub teaches: Field,
    pub identifiers: Field,
    pub active: Field,
}

impl CourseIndexSchema {
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
    fn default() -> Self {
        Self::new()
    }
}

pub struct CourseIndex {
    index: Index,
    schema: CourseIndexSchema,
    reader: IndexReader,
}

impl CourseIndex {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = CourseIndexSchema::new();
        let index = Index::create_in_dir(path, schema.schema.clone())
            .map_err(|e| crate::Error::Search(format!("create index: {e}")))?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| crate::Error::Search(format!("create reader: {e}")))?;
        Ok(Self { index, schema, reader })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let schema = CourseIndexSchema::new();
        let index = Index::open_in_dir(path)
            .map_err(|e| crate::Error::Search(format!("open index: {e}")))?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| crate::Error::Search(format!("create reader: {e}")))?;
        Ok(Self { index, schema, reader })
    }

    pub fn create_or_open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        if p.join("meta.json").exists() {
            Self::open(p)
        } else {
            Self::create(p)
        }
    }

    pub fn writer(&self, heap_mb: usize) -> Result<IndexWriter> {
        self.index
            .writer(heap_mb * 1_000_000)
            .map_err(|e| crate::Error::Search(format!("create writer: {e}")))
    }

    pub fn index(&self) -> &Index { &self.index }
    pub fn schema(&self) -> &CourseIndexSchema { &self.schema }
    pub fn reader(&self) -> &IndexReader { &self.reader }

    pub fn reload(&self) -> Result<()> {
        self.reader
            .reload()
            .map_err(|e| crate::Error::Search(format!("reload: {e}")))
    }

    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        Ok(IndexStats {
            num_docs: searcher.num_docs() as usize,
            num_segments: searcher.segment_readers().len(),
        })
    }
}

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
    fn empty_index_has_zero_docs() {
        let dir = TempDir::new().unwrap();
        let idx = CourseIndex::create(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }

    #[test]
    fn create_or_open_round_trips() {
        let dir = TempDir::new().unwrap();
        let _ = CourseIndex::create_or_open(dir.path()).unwrap();
        // second call opens existing index
        let idx = CourseIndex::create_or_open(dir.path()).unwrap();
        assert_eq!(idx.stats().unwrap().num_docs, 0);
    }
}
