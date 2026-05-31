//! Tantivy search index schema and lifecycle.
//!
//! Fields are tuned for the event domain: full-text on title /
//! description / keywords; exact/string facets for status, type,
//! attendance mode, language, location URL, and identifier values;
//! a date_string for the event's start date (yyyy-mm-dd) so range
//! filters degrade to lexicographic comparisons.

use std::path::Path;
use tantivy::{
    schema::{Field, Schema, FAST, STORED, STRING, TEXT},
    Index, IndexReader, IndexWriter, ReloadPolicy,
};

use crate::Result;

/// Strongly-typed handle to the event index's schema fields.
#[derive(Clone)]
pub struct EventIndexSchema {
    pub schema: Schema,
    pub id: Field,
    pub name: Field,
    pub alternate_names: Field,
    pub description: Field,
    pub keywords: Field,
    pub start_date: Field,
    pub end_date: Field,
    pub event_status: Field,
    pub event_attendance_mode: Field,
    pub event_type: Field,
    pub in_language: Field,
    pub location_name: Field,
    pub location_city: Field,
    pub location_country: Field,
    pub location_url: Field,
    pub organizer_name: Field,
    pub performer_name: Field,
    pub identifier_value: Field,
    pub active: Field,
}

impl EventIndexSchema {
    pub fn new() -> Self {
        let mut b = Schema::builder();
        let id = b.add_text_field("id", STRING | STORED);
        let name = b.add_text_field("name", TEXT | STORED);
        let alternate_names = b.add_text_field("alternate_names", TEXT | STORED);
        let description = b.add_text_field("description", TEXT | STORED);
        let keywords = b.add_text_field("keywords", TEXT | STORED);
        let start_date = b.add_text_field("start_date", STRING | STORED);
        let end_date = b.add_text_field("end_date", STRING | STORED);
        let event_status = b.add_text_field("event_status", STRING | STORED);
        let event_attendance_mode = b.add_text_field("event_attendance_mode", STRING | STORED);
        let event_type = b.add_text_field("event_type", STRING | STORED);
        let in_language = b.add_text_field("in_language", STRING | STORED);
        let location_name = b.add_text_field("location_name", TEXT | STORED);
        let location_city = b.add_text_field("location_city", STRING | STORED);
        let location_country = b.add_text_field("location_country", STRING | STORED);
        let location_url = b.add_text_field("location_url", STRING | STORED);
        let organizer_name = b.add_text_field("organizer_name", TEXT | STORED);
        let performer_name = b.add_text_field("performer_name", TEXT | STORED);
        let identifier_value = b.add_text_field("identifier_value", STRING | STORED);
        let active = b.add_text_field("active", STRING | FAST);
        let schema = b.build();
        Self {
            schema,
            id,
            name,
            alternate_names,
            description,
            keywords,
            start_date,
            end_date,
            event_status,
            event_attendance_mode,
            event_type,
            in_language,
            location_name,
            location_city,
            location_country,
            location_url,
            organizer_name,
            performer_name,
            identifier_value,
            active,
        }
    }
}

impl Default for EventIndexSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// Tantivy index, schema, and reader for events.
pub struct EventIndex {
    index: Index,
    schema: EventIndexSchema,
    reader: IndexReader,
}

impl EventIndex {
    pub fn create<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let s = EventIndexSchema::new();
        let index = Index::create_in_dir(index_path, s.schema.clone())
            .map_err(|e| crate::Error::Search(format!("Failed to create index: {e}")))?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| crate::Error::Search(format!("Failed to create reader: {e}")))?;
        Ok(Self {
            index,
            schema: s,
            reader,
        })
    }

    pub fn open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let s = EventIndexSchema::new();
        let index = Index::open_in_dir(index_path)
            .map_err(|e| crate::Error::Search(format!("Failed to open index: {e}")))?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| crate::Error::Search(format!("Failed to create reader: {e}")))?;
        Ok(Self {
            index,
            schema: s,
            reader,
        })
    }

    pub fn create_or_open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let path = index_path.as_ref();
        if path.join("meta.json").exists() {
            Self::open(index_path)
        } else {
            Self::create(index_path)
        }
    }

    pub fn writer(&self, heap_size_mb: usize) -> Result<IndexWriter> {
        self.index
            .writer(heap_size_mb * 1_000_000)
            .map_err(|e| crate::Error::Search(format!("Failed to create writer: {e}")))
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    pub fn schema(&self) -> &EventIndexSchema {
        &self.schema
    }

    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }

    pub fn reload(&self) -> Result<()> {
        self.reader
            .reload()
            .map_err(|e| crate::Error::Search(format!("Failed to reload reader: {e}")))
    }

    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        Ok(IndexStats {
            num_docs: searcher.num_docs() as usize,
            num_segments: searcher.segment_readers().len(),
        })
    }

    pub fn optimize(&self) -> Result<()> {
        let writer = self.writer(50)?;
        writer
            .wait_merging_threads()
            .map_err(|e| crate::Error::Search(format!("Failed to optimize index: {e}")))?;
        Ok(())
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
    fn create_empty_index() {
        let tmp = TempDir::new().unwrap();
        let i = EventIndex::create(tmp.path()).unwrap();
        assert_eq!(i.stats().unwrap().num_docs, 0);
    }

    #[test]
    fn schema_has_event_fields() {
        let s = EventIndexSchema::new();
        let _ = (s.name, s.start_date, s.event_status, s.event_type, s.organizer_name);
    }

    #[test]
    fn create_or_open_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let a = EventIndex::create_or_open(tmp.path()).unwrap();
        assert_eq!(a.stats().unwrap().num_docs, 0);
        let b = EventIndex::create_or_open(tmp.path()).unwrap();
        assert_eq!(b.stats().unwrap().num_docs, 0);
    }
}
