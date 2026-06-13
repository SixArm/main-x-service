//! Event full-text search over an embedded Tantivy index.
//!
//! [`SearchEngine`](crate::search::SearchEngine) is the high-level
//! facade used by the API and matching layers. It wraps an
//! [`EventIndex`](crate::search::index::EventIndex) (the low-level
//! schema + reader/writer in [`index`](crate::search::index)) and offers
//! free-text, fuzzy, date-range, and combined name+date queries plus
//! indexing and deletion. Documents are flattened from the rich
//! [`Event`](crate::models::Event) aggregate by the private `build_doc`
//! helper; queries return matching event-id strings.
//!
//! Heavy I/O setup (a Tantivy index lives on disk) means the examples
//! here are described in prose rather than as runnable doctests; see the
//! `#[cfg(test)]` module for usage against a `tempfile::TempDir`.

use std::path::Path;
use tantivy::{
    collector::TopDocs,
    doc,
    query::{BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser},
    schema::{Term, Value},
};

use crate::Result;
use crate::models::{Event, Location, Party};

/// Low-level Tantivy index: schema, reader, writer, stats.
pub mod index;
/// Query-building helpers layered on the index.
pub mod query;

pub use index::{EventIndex, EventIndexSchema, IndexStats};

/// High-level facade over the Tantivy [`EventIndex`].
pub struct SearchEngine {
    /// The underlying index (schema + reader/writer).
    index: EventIndex,
}

impl SearchEngine {
    /// Open (or create) the index at `index_path` and wrap it.
    pub fn new<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        Ok(Self {
            index: EventIndex::create_or_open(index_path)?,
        })
    }

    /// Add (or re-add) one event to the index.
    pub fn index_event(&self, event: &Event) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let s = self.index.schema();
        let d = build_doc(event, s);
        writer
            .add_document(d)
            .map_err(|e| crate::Error::Search(format!("add_document: {e}")))?;
        writer
            .commit()
            .map_err(|e| crate::Error::Search(format!("commit: {e}")))?;
        Ok(())
    }

    /// Bulk index. Single commit at the end.
    pub fn index_events(&self, events: &[Event]) -> Result<()> {
        let mut writer = self.index.writer(100)?;
        let s = self.index.schema();
        for event in events {
            writer
                .add_document(build_doc(event, s))
                .map_err(|e| crate::Error::Search(format!("add_document: {e}")))?;
        }
        writer
            .commit()
            .map_err(|e| crate::Error::Search(format!("commit: {e}")))?;
        Ok(())
    }

    /// Free-text search across name / description / keywords /
    /// alternate_names / organizer / performer / identifier_value.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let parser = QueryParser::for_index(
            self.index.index(),
            vec![
                s.name,
                s.alternate_names,
                s.description,
                s.keywords,
                s.organizer_name,
                s.performer_name,
                s.identifier_value,
            ],
        );
        let query = parser
            .parse_query(query_str)
            .map_err(|e| crate::Error::Search(format!("parse_query: {e}")))?;
        let top = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("search: {e}")))?;
        Ok(extract_ids(&searcher, s.id, &top))
    }

    /// Fuzzy search on the event title.
    pub fn fuzzy_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let term = Term::from_field_text(s.name, query_str);
        let query = FuzzyTermQuery::new(term, 2, true);
        let top = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("fuzzy_search: {e}")))?;
        Ok(extract_ids(&searcher, s.id, &top))
    }

    /// Filter by a `start_date` range (inclusive). Either bound may
    /// be `None` for unbounded.
    pub fn search_by_date_range(
        &self,
        from_yyyy_mm_dd: Option<&str>,
        to_yyyy_mm_dd: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        // We approximate a range filter by parsing a query string of
        // the form `start_date:[from TO to]`.
        let from = from_yyyy_mm_dd.unwrap_or("*");
        let to = to_yyyy_mm_dd.unwrap_or("*");
        let query_str = format!("start_date:[{from} TO {to}]");
        let parser = QueryParser::for_index(self.index.index(), vec![s.start_date]);
        let query = parser
            .parse_query(&query_str)
            .map_err(|e| crate::Error::Search(format!("parse range: {e}")))?;
        let top = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("search range: {e}")))?;
        Ok(extract_ids(&searcher, s.id, &top))
    }

    /// Combined name + date filter (used by the matching layer as a
    /// blocking step).
    pub fn search_by_name_and_date(
        &self,
        name: &str,
        date_yyyy_mm_dd: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let name_term = Term::from_field_text(s.name, name);
        let name_q: Box<dyn Query> = Box::new(FuzzyTermQuery::new(name_term, 2, true));
        let final_q: Box<dyn Query> = if let Some(date) = date_yyyy_mm_dd {
            let parser = QueryParser::for_index(self.index.index(), vec![s.start_date]);
            if let Ok(date_q) = parser.parse_query(date) {
                Box::new(BooleanQuery::new(vec![
                    (Occur::Must, name_q),
                    (Occur::Should, date_q),
                ]))
            } else {
                name_q
            }
        } else {
            name_q
        };
        let top = searcher
            .search(final_q.as_ref(), &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("search name+date: {e}")))?;
        Ok(extract_ids(&searcher, s.id, &top))
    }

    /// Remove an event from the index by its id string.
    pub fn delete_event(&self, event_id: &str) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let s = self.index.schema();
        let term = Term::from_field_text(s.id, event_id);
        writer.delete_term(term);
        writer
            .commit()
            .map_err(|e| crate::Error::Search(format!("commit delete: {e}")))?;
        Ok(())
    }

    /// Return index statistics (document count, …).
    pub fn stats(&self) -> Result<IndexStats> {
        self.index.stats()
    }

    /// Merge index segments to reclaim space and speed up reads.
    pub fn optimize(&self) -> Result<()> {
        self.index.optimize()
    }

    /// Reload the reader so recently committed writes become visible.
    pub fn reload(&self) -> Result<()> {
        self.index.reload()
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Flatten an [`Event`] aggregate into a Tantivy document, joining
/// multi-valued fields into space-separated strings and summarizing
/// locations/parties into searchable text fields.
fn build_doc(event: &Event, s: &EventIndexSchema) -> tantivy::TantivyDocument {
    let alternate_names = event.alternate_names.join(" ");
    let keywords = event.keywords.join(" ");
    let languages = event.in_language.join(" ");
    let identifier_values = event
        .identifiers
        .iter()
        .map(|id| id.value.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let (loc_name, loc_city, loc_country, loc_url) = summarize_locations(&event.location);
    let organizer_name = parties_names(&event.organizers);
    let performer_name = parties_names(&event.performers);
    let start_date = event.start_date.strftime("%Y-%m-%d").to_string();
    let end_date = event
        .end_date
        .map(|d| d.strftime("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let description = event.description.clone().unwrap_or_default();

    doc!(
        s.id => event.id.to_string(),
        s.name => event.name.clone(),
        s.alternate_names => alternate_names,
        s.description => description,
        s.keywords => keywords,
        s.start_date => start_date,
        s.end_date => end_date,
        s.event_status => serde_plain(&event.event_status),
        s.event_attendance_mode => serde_plain(&event.event_attendance_mode),
        s.event_type => serde_plain(&event.event_type),
        s.in_language => languages,
        s.location_name => loc_name,
        s.location_city => loc_city,
        s.location_country => loc_country,
        s.location_url => loc_url,
        s.organizer_name => organizer_name,
        s.performer_name => performer_name,
        s.identifier_value => identifier_values,
        s.active => if event.active { "true" } else { "false" },
    )
}

/// Join all party names into one space-separated, searchable string.
fn parties_names(parties: &[Party]) -> String {
    parties
        .iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Collapse a `Vec<Location>` into four space-joined searchable strings:
/// `(names, cities, countries, urls)`, dispatched per variant.
fn summarize_locations(locs: &[Location]) -> (String, String, String, String) {
    let mut names = Vec::new();
    let mut cities = Vec::new();
    let mut countries = Vec::new();
    let mut urls = Vec::new();
    for l in locs {
        match l {
            Location::Place(p) => {
                names.push(p.name.clone());
                if let Some(ref addr) = p.address {
                    if let Some(c) = &addr.city {
                        cities.push(c.clone());
                    }
                    if let Some(c) = &addr.country {
                        countries.push(c.clone());
                    }
                }
                if let Some(u) = &p.url {
                    urls.push(u.clone());
                }
            }
            Location::PostalAddress(addr) => {
                if let Some(c) = &addr.city {
                    cities.push(c.clone());
                }
                if let Some(c) = &addr.country {
                    countries.push(c.clone());
                }
            }
            Location::Virtual(v) => {
                if let Some(n) = &v.name {
                    names.push(n.clone());
                }
                urls.push(v.url.clone());
            }
            Location::Text { value } => names.push(value.clone()),
        }
    }
    (
        names.join(" "),
        cities.join(" "),
        countries.join(" "),
        urls.join(" "),
    )
}

/// Serialize an enum to its serde-default lowercase string form.
fn serde_plain<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|x| x.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// Pull the stored `id` string from each scored hit, skipping any
/// document that fails to load or lacks the field.
fn extract_ids(
    searcher: &tantivy::Searcher,
    id_field: tantivy::schema::Field,
    docs: &[(f32, tantivy::DocAddress)],
) -> Vec<String> {
    let mut ids = Vec::new();
    for (_, addr) in docs {
        let doc: tantivy::TantivyDocument = match searcher.doc(*addr) {
            Ok(d) => d,
            Err(_) => continue,
        };
        if let Some(v) = doc.get_first(id_field) {
            if let Some(s) = v.as_str() {
                ids.push(s.to_string());
            }
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Event, EventType, Party, PartyKind};
    use jiff::Timestamp;
    use tempfile::TempDir;

    /// Build a minimal event for indexing tests.
    fn evt(name: &str, when: Timestamp) -> Event {
        Event::new(name, when)
    }

    /// An indexed event is found by a token from its name.
    #[test]
    fn index_and_search_event_by_name() {
        let tmp = TempDir::new().unwrap();
        let engine = SearchEngine::new(tmp.path()).unwrap();
        let when = jiff::civil::datetime(2026, 3, 1, 9, 0, 0, 0)
            .in_tz("UTC")
            .unwrap()
            .timestamp();
        let event = evt("Annual Conference", when);
        engine.index_event(&event).unwrap();
        engine.reload().unwrap();
        let ids = engine.search("Conference", 10).unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], event.id.to_string());
    }

    /// Fuzzy search tolerates a one-character typo in the title.
    #[test]
    fn fuzzy_search_finds_typo() {
        let tmp = TempDir::new().unwrap();
        let engine = SearchEngine::new(tmp.path()).unwrap();
        let when = jiff::civil::datetime(2026, 3, 1, 9, 0, 0, 0)
            .in_tz("UTC")
            .unwrap()
            .timestamp();
        let event = evt("Hackathon", when);
        engine.index_event(&event).unwrap();
        engine.reload().unwrap();
        let ids = engine.fuzzy_search("Hakathon", 10).unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], event.id.to_string());
    }

    /// Free-text search matches against an indexed organizer name.
    #[test]
    fn search_finds_organizer_name() {
        let tmp = TempDir::new().unwrap();
        let engine = SearchEngine::new(tmp.path()).unwrap();
        let when = jiff::civil::datetime(2026, 3, 1, 9, 0, 0, 0)
            .in_tz("UTC")
            .unwrap()
            .timestamp();
        let mut event = evt("Talk", when);
        event.event_type = EventType::Conference;
        event.organizers.push(Party {
            kind: PartyKind::Organization,
            id: None,
            name: "Cal Performances".into(),
            email: None,
            url: None,
        });
        engine.index_event(&event).unwrap();
        engine.reload().unwrap();
        let ids = engine.search("Performances", 10).unwrap();
        assert_eq!(ids.len(), 1);
    }

    /// Deleting an event removes it from subsequent search results.
    #[test]
    fn delete_event_drops_from_index() {
        let tmp = TempDir::new().unwrap();
        let engine = SearchEngine::new(tmp.path()).unwrap();
        let when = jiff::civil::datetime(2026, 3, 1, 9, 0, 0, 0)
            .in_tz("UTC")
            .unwrap()
            .timestamp();
        let event = evt("Workshop", when);
        engine.index_event(&event).unwrap();
        engine.reload().unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 1);
        engine.delete_event(&event.id.to_string()).unwrap();
        engine.reload().unwrap();
        assert_eq!(engine.search("Workshop", 10).unwrap().len(), 0);
    }

    /// Bulk indexing commits all events in a single batch.
    #[test]
    fn bulk_index() {
        let tmp = TempDir::new().unwrap();
        let engine = SearchEngine::new(tmp.path()).unwrap();
        let when = jiff::civil::datetime(2026, 3, 1, 9, 0, 0, 0)
            .in_tz("UTC")
            .unwrap()
            .timestamp();
        let events = vec![evt("One", when), evt("Two", when), evt("Three", when)];
        engine.index_events(&events).unwrap();
        engine.reload().unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 3);
    }
}
