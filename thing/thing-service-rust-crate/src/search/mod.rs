//! Tantivy-backed search facade for Thing records.
//!
//! [`SearchEngine`] wraps a [`ThingIndex`] with the operations the REST layer
//! needs: index/delete a record, exact full-text search, fuzzy (typo-tolerant)
//! search, and the name-only "blocking" query the duplicate detector uses to
//! narrow the candidate set before scoring. The index stores only the `id`
//! (the searchable text fields are indexed for retrieval of ids), so every hit
//! is later hydrated from the database.

use std::path::Path;

use tantivy::{
    TantivyDocument,
    collector::TopDocs,
    doc,
    query::{BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser},
    schema::{Term, Value},
};

use crate::Result;
use crate::models::thing::Thing;

pub mod index;

pub use index::{IndexStats, ThingIndex, ThingIndexSchema};

/// High-level search facade over a [`ThingIndex`].
pub struct SearchEngine {
    /// The underlying Tantivy index wrapper.
    index: ThingIndex,
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
        let index = ThingIndex::create_or_open(p)?;
        Ok(Self {
            index,
            index_path: p.to_string_lossy().into_owned(),
        })
    }

    /// Index (or re-index) one thing.
    pub fn index_thing(&self, thing: &Thing) -> Result<()> {
        // 50 MB writer heap budget — ample for single-document writes.
        let mut writer = self.index.writer(50)?;
        let s = self.index.schema();

        // Flatten the repeating collections into single space-joined text
        // fields so they are searchable as ordinary tokens.
        let alt = thing.alternate_names.join(" ");
        let idents: String = thing
            .identifiers
            .iter()
            .map(|i| i.value.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let doc = doc!(
            s.id => thing.id.to_string(),
            s.name => thing.name.clone(),
            s.alternate_names => alt,
            s.description => thing.description.clone().unwrap_or_default(),
            s.identifiers => idents,
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

    /// Full-text search over name + alternate_names + description +
    /// identifiers.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let parser = QueryParser::for_index(
            self.index.index(),
            vec![s.name, s.alternate_names, s.description, s.identifiers],
        );
        let query = parser
            .parse_query(query_str)
            .map_err(|e| crate::Error::Search(format!("parse query: {e}")))?;
        self.collect_ids(searcher, query.as_ref(), limit)
    }

    /// Fuzzy search — tolerates typos.
    pub fn fuzzy_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let tokens = tokenise(query_str);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let fields = [s.name, s.alternate_names, s.description];
        // Build a Should-of-fuzzy-terms boolean query: each token may match any
        // field within a Levenshtein distance of 2, with prefix matching on
        // (so a missing trailing char still hits). Any one match contributes.
        let mut sub: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for t in &tokens {
            for f in fields {
                let term = Term::from_field_text(f, t);
                // distance = 2 (max edits), transposition_cost_one = true (prefix).
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
                let q: Box<dyn Query> = Box::new(FuzzyTermQuery::new(
                    Term::from_field_text(s.name, t),
                    2,
                    true,
                ));
                (Occur::Should, q)
            })
            .collect();
        let q = BooleanQuery::new(sub);
        self.collect_ids(searcher, &q, limit)
    }

    /// Delete a thing from the index by id.
    pub fn delete_thing(&self, thing_id: &str) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let s = self.index.schema();
        let term = Term::from_field_text(s.id, thing_id);
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

/// Split a query string into lowercase alphanumeric tokens.
fn tokenise(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    //! Unit tests for the Tantivy-backed `SearchEngine` (index + query).
    use super::*;
    use tempfile::TempDir;

    /// Test helper: a minimally-populated `Thing` with just a name, enough
    /// to exercise name indexing and querying.
    fn thing(name: &str) -> Thing {
        Thing::new(name)
    }

    /// A thing is indexed and found by an exact full-text query.
    #[test]
    fn index_and_exact_search() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        let t = thing("Pride and Prejudice");
        eng.index_thing(&t).unwrap();
        let hits = eng.search("Pride and Prejudice", 10).unwrap();
        assert_eq!(hits, vec![t.id.to_string()]);
    }

    /// Fuzzy search finds a thing despite a single-character typo.
    #[test]
    fn fuzzy_search_tolerates_typo() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        let t = thing("Algorithms");
        eng.index_thing(&t).unwrap();
        let hits = eng.fuzzy_search("Algoritms", 10).unwrap();
        assert_eq!(hits, vec![t.id.to_string()]);
    }

    /// Deleting a thing removes it from the index document count.
    #[test]
    fn delete_removes_from_index() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        let t = thing("War and Peace");
        eng.index_thing(&t).unwrap();
        assert_eq!(eng.stats().unwrap().num_docs, 1);
        eng.delete_thing(&t.id.to_string()).unwrap();
        assert_eq!(eng.stats().unwrap().num_docs, 0);
    }

    /// `tokenise` splits on punctuation and drops blanks.
    #[test]
    fn tokenise_handles_punctuation() {
        assert_eq!(tokenise("War-and-Peace"), vec!["war", "and", "peace"]);
        assert_eq!(tokenise("   "), Vec::<String>::new());
    }
}
