//! Tantivy-backed search facade for Place records.
//!
//! Open/create at a directory path, index on every CRUD write, force
//! `reload()` after each commit so reads observe the new segment
//! immediately. The duplicate detector blocks candidates via
//! `search_by_name`.

use std::path::Path;

use tantivy::{
    TantivyDocument,
    collector::TopDocs,
    doc,
    query::{BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser},
    schema::{Term, Value},
};

use crate::Result;
use crate::models::place::Place;

pub mod index;

pub use index::{IndexStats, PlaceIndex, PlaceIndexSchema};

/// High-level search facade over a [`PlaceIndex`].
pub struct SearchEngine {
    /// The underlying Tantivy index wrapper.
    index: PlaceIndex,
    /// Filesystem path the index lives at (for diagnostics / reopen).
    pub index_path: String,
}

impl SearchEngine {
    /// Open or create the index under `path`, creating the directory if
    /// it does not yet exist.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        std::fs::create_dir_all(p)
            .map_err(|e| crate::Error::Search(format!("ensure index dir: {e}")))?;
        let index = PlaceIndex::create_or_open(p)?;
        Ok(Self {
            index,
            index_path: p.to_string_lossy().into_owned(),
        })
    }

    /// Index (or re-index) one place. Caller should `delete_place` first
    /// when replacing an existing document.
    pub fn index_place(&self, place: &Place) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let s = self.index.schema();

        let kw = place.keywords.join(" ");
        let locality = place
            .address
            .as_ref()
            .and_then(|a| a.address_locality.clone())
            .unwrap_or_default();
        let idents: String = place
            .identifiers
            .iter()
            .map(|i| i.value.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let doc = doc!(
            s.id => place.id.to_string(),
            s.name => place.name.clone(),
            s.alternate_name => place.alternate_name.clone().unwrap_or_default(),
            s.keywords => kw,
            s.locality => locality,
            s.identifiers => idents,
            s.gln => place.global_location_number.clone().unwrap_or_default(),
        );

        writer
            .add_document(doc)
            .map_err(|e| crate::Error::Search(format!("add document: {e}")))?;
        writer
            .commit()
            .map_err(|e| crate::Error::Search(format!("commit: {e}")))?;
        self.index.reload()?;
        Ok(())
    }

    /// Full-text search over name + alternate_name + keywords + locality +
    /// identifiers.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let parser = QueryParser::for_index(
            self.index.index(),
            vec![s.name, s.alternate_name, s.keywords, s.locality, s.identifiers],
        );
        let query = parser
            .parse_query(query_str)
            .map_err(|e| crate::Error::Search(format!("parse query: {e}")))?;
        self.collect_ids(searcher, query.as_ref(), limit)
    }

    /// Fuzzy search — tolerates typos. A query that tokenises to nothing
    /// returns an empty result rather than an error.
    pub fn fuzzy_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let tokens = tokenise(query_str);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let fields = [s.name, s.alternate_name, s.keywords, s.locality];
        let mut sub: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for t in &tokens {
            for f in fields {
                let term = Term::from_field_text(f, t);
                sub.push((Occur::Should, Box::new(FuzzyTermQuery::new(term, 2, true))));
            }
        }
        let q = BooleanQuery::new(sub);
        self.collect_ids(searcher, &q, limit)
    }

    /// Blocking query used by the duplicate detector: fuzzy name match.
    pub fn search_by_name(&self, name: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let tokens = tokenise(name);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let sub: Vec<(Occur, Box<dyn Query>)> = tokens
            .iter()
            .map(|t| {
                let q: Box<dyn Query> =
                    Box::new(FuzzyTermQuery::new(Term::from_field_text(s.name, t), 2, true));
                (Occur::Should, q)
            })
            .collect();
        let q = BooleanQuery::new(sub);
        self.collect_ids(searcher, &q, limit)
    }

    /// Delete a place from the index by id.
    pub fn delete_place(&self, place_id: &str) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let s = self.index.schema();
        let term = Term::from_field_text(s.id, place_id);
        writer.delete_term(term);
        writer
            .commit()
            .map_err(|e| crate::Error::Search(format!("commit delete: {e}")))?;
        self.index.reload()?;
        Ok(())
    }

    /// Document and segment counts for the live index.
    pub fn stats(&self) -> Result<IndexStats> {
        self.index.stats()
    }

    /// Force the reader to observe the latest committed segments.
    pub fn reload(&self) -> Result<()> {
        self.index.reload()
    }

    /// Run `query` and project the top `limit` hits to their `id` strings.
    fn collect_ids(
        &self,
        searcher: tantivy::Searcher,
        query: &dyn Query,
        limit: usize,
    ) -> Result<Vec<String>> {
        let s = self.index.schema();
        let top = searcher
            .search(query, &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("search: {e}")))?;
        let mut ids = Vec::with_capacity(top.len());
        for (_score, addr) in top {
            let doc: TantivyDocument = searcher
                .doc(addr)
                .map_err(|e| crate::Error::Search(format!("retrieve doc: {e}")))?;
            if let Some(v) = doc.get_first(s.id) {
                if let Some(t) = v.as_str() {
                    ids.push(t.to_string());
                }
            }
        }
        Ok(ids)
    }
}

/// Split a query string into lowercase alphanumeric tokens, dropping
/// empties.
fn tokenise(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn place(name: &str) -> Place {
        Place::new(name)
    }

    /// A place is indexed and found by an exact full-text query.
    #[test]
    fn index_and_exact_search() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        let p = place("Central Park");
        eng.index_place(&p).unwrap();
        let hits = eng.search("Central Park", 10).unwrap();
        assert_eq!(hits, vec![p.id.to_string()]);
    }

    /// Fuzzy search finds a place despite a single-character typo.
    #[test]
    fn fuzzy_search_tolerates_typo() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        let p = place("Yellowstone");
        eng.index_place(&p).unwrap();
        let hits = eng.fuzzy_search("Yellowstoen", 10).unwrap();
        assert_eq!(hits, vec![p.id.to_string()]);
    }

    /// Deleting a place removes it from the index document count.
    #[test]
    fn delete_removes_from_index() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        let p = place("Hyde Park");
        eng.index_place(&p).unwrap();
        assert_eq!(eng.stats().unwrap().num_docs, 1);
        eng.delete_place(&p.id.to_string()).unwrap();
        assert_eq!(eng.stats().unwrap().num_docs, 0);
    }

    /// `tokenise` splits on punctuation and drops blanks.
    #[test]
    fn tokenise_handles_punctuation() {
        assert_eq!(tokenise("Central-Park"), vec!["central", "park"]);
        assert_eq!(tokenise("   "), Vec::<String>::new());
    }
}
