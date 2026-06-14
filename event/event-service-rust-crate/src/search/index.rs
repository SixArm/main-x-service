//! Tantivy search index schema and lifecycle.
//!
//! Fields are tuned for the event domain: full-text on title /
//! description / keywords; exact/string facets for status, type,
//! attendance mode, language, location URL, and identifier values;
//! a date_string for the event's start date (yyyy-mm-dd) so range
//! filters degrade to lexicographic comparisons.

use std::path::Path;
use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy,
    schema::{FAST, Field, STORED, STRING, Schema, TEXT},
};

use crate::Result;

/// Strongly-typed handle to the event index's schema fields.
#[derive(Clone)]
pub struct EventIndexSchema {
    /// The built Tantivy schema.
    pub schema: Schema,
    /// Stored string field for the event UUID (the search "primary key").
    pub id: Field,
    /// Full-text title field.
    pub name: Field,
    /// Full-text alternate-names field.
    pub alternate_names: Field,
    /// Full-text description field.
    pub description: Field,
    /// Full-text keywords field.
    pub keywords: Field,
    /// `yyyy-mm-dd` string field for the start date (lexicographic range).
    pub start_date: Field,
    /// `yyyy-mm-dd` string field for the end date.
    pub end_date: Field,
    /// Exact-string facet for event status.
    pub event_status: Field,
    /// Exact-string facet for attendance mode.
    pub event_attendance_mode: Field,
    /// Exact-string facet for event type.
    pub event_type: Field,
    /// Exact-string facet for language codes.
    pub in_language: Field,
    /// Full-text venue/location name field.
    pub location_name: Field,
    /// Exact-string facet for location city.
    pub location_city: Field,
    /// Exact-string facet for location country.
    pub location_country: Field,
    /// Exact-string field for location URL.
    pub location_url: Field,
    /// Full-text organizer-name field.
    pub organizer_name: Field,
    /// Full-text performer-name field.
    pub performer_name: Field,
    /// Exact-string field for identifier values.
    pub identifier_value: Field,
    /// Fast string field for the active/soft-delete flag.
    pub active: Field,
}

impl EventIndexSchema {
    /// Build the event schema, registering every field with its
    /// indexing options (TEXT for full-text, STRING for exact facets).
    #[must_use]
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
    /// Same as [`EventIndexSchema::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Tantivy index, schema, and reader for events.
pub struct EventIndex {
    /// The Tantivy index.
    index: Index,
    /// The typed schema-field handles.
    schema: EventIndexSchema,
    /// The reader used to build searchers.
    reader: IndexReader,
}

impl EventIndex {
    /// Create a brand-new index in `index_path` (errors if one exists).
    ///
    /// # Errors
    ///
    /// Returns an error if the index or its reader cannot be created.
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

    /// Open an existing index in `index_path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the index cannot be opened or its reader
    /// cannot be created.
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

    /// Open the index if `meta.json` exists, otherwise create it.
    ///
    /// # Errors
    ///
    /// Returns an error if opening or creating the index fails.
    pub fn create_or_open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let path = index_path.as_ref();
        if path.join("meta.json").exists() {
            Self::open(index_path)
        } else {
            Self::create(index_path)
        }
    }

    /// Build a writer with a heap budget of `heap_size_mb` megabytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the writer cannot be created.
    pub fn writer(&self, heap_size_mb: usize) -> Result<IndexWriter> {
        self.index
            .writer(heap_size_mb * 1_000_000)
            .map_err(|e| crate::Error::Search(format!("Failed to create writer: {e}")))
    }

    /// Borrow the underlying Tantivy [`Index`].
    #[must_use]
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Borrow the typed schema-field handles.
    #[must_use]
    pub fn schema(&self) -> &EventIndexSchema {
        &self.schema
    }

    /// Borrow the index reader.
    #[must_use]
    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }

    /// Reload the reader so newly committed documents become visible.
    ///
    /// # Errors
    ///
    /// Returns an error if the reader cannot be reloaded.
    pub fn reload(&self) -> Result<()> {
        self.reader
            .reload()
            .map_err(|e| crate::Error::Search(format!("Failed to reload reader: {e}")))
    }

    /// Return current index statistics (document and segment counts).
    ///
    /// # Errors
    ///
    /// Returns an error if the index cannot be read.
    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        Ok(IndexStats {
            num_docs: usize::try_from(searcher.num_docs()).unwrap_or(usize::MAX),
            num_segments: searcher.segment_readers().len(),
        })
    }

    /// Wait for background segment merges to finish, compacting the index.
    ///
    /// # Errors
    ///
    /// Returns an error if waiting for the merge threads fails.
    pub fn optimize(&self) -> Result<()> {
        let writer = self.writer(50)?;
        writer
            .wait_merging_threads()
            .map_err(|e| crate::Error::Search(format!("Failed to optimize index: {e}")))?;
        Ok(())
    }
}

/// Snapshot of index size metrics.
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Number of (live) documents in the index.
    pub num_docs: usize,
    /// Number of on-disk segments.
    pub num_segments: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A freshly created index has zero documents.
    #[test]
    fn create_empty_index() {
        let tmp = TempDir::new().unwrap();
        let i = EventIndex::create(tmp.path()).unwrap();
        assert_eq!(i.stats().unwrap().num_docs, 0);
    }

    /// The schema exposes the expected event fields.
    #[test]
    fn schema_has_event_fields() {
        let s = EventIndexSchema::new();
        let _ = (
            s.name,
            s.start_date,
            s.event_status,
            s.event_type,
            s.organizer_name,
        );
    }

    /// Calling `create_or_open` twice on the same path is safe.
    #[test]
    fn create_or_open_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let a = EventIndex::create_or_open(tmp.path()).unwrap();
        assert_eq!(a.stats().unwrap().num_docs, 0);
        let b = EventIndex::create_or_open(tmp.path()).unwrap();
        assert_eq!(b.stats().unwrap().num_docs, 0);
    }
}
