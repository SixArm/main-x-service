//! Tantivy-backed search facade for Place records.
//!
//! Open/create at a directory path, index on every CRUD write, force
//! `reload()` after each commit so reads observe the new segment
//! immediately. The duplicate detector blocks candidates via
//! `search_by_name`.

use std::path::Path;

use tantivy::{
    TantivyDocument,
    collector::{Count, TopDocs},
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
    ///
    /// # Errors
    /// Returns [`crate::Error::Search`] if the directory cannot be created
    /// or the index cannot be opened/created.
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
    ///
    /// # Errors
    /// Returns [`crate::Error::Search`] if acquiring the writer, adding the
    /// document, committing, or reloading the reader fails.
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

    /// Full-text search over name + `alternate_name` + keywords + locality +
    /// identifiers.
    ///
    /// # Errors
    /// Returns [`crate::Error::Search`] if the query fails to parse or the
    /// search itself fails.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let query = self.exact_query(query_str)?;
        self.collect_ids(&searcher, query.as_ref(), limit)
    }

    /// Fuzzy search — tolerates typos. A query that tokenises to nothing
    /// returns an empty result rather than an error.
    ///
    /// # Errors
    /// Returns [`crate::Error::Search`] if the underlying search fails.
    pub fn fuzzy_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let Some(query) = self.fuzzy_query(query_str) else {
            return Ok(Vec::new());
        };
        self.collect_ids(&searcher, query.as_ref(), limit)
    }

    /// One page of hits **plus the true total**: `(ids, total)`.
    ///
    /// A page can never tell a caller how much there is in total — which
    /// is the whole point of `X-Total-Count`
    /// (`agents/share/restful.md`) — so the total comes from Tantivy's
    /// `Count` collector rather than the page length. `fuzzy` selects the
    /// same two retrieval routes [`search`](Self::search) /
    /// [`fuzzy_search`](Self::fuzzy_search) expose.
    ///
    /// # Errors
    /// Returns [`crate::Error::Search`] if the query fails to parse or the
    /// underlying search fails.
    pub fn search_page(
        &self,
        query_str: &str,
        fuzzy: bool,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<String>, usize)> {
        let searcher = self.index.reader().searcher();
        let query = if fuzzy {
            let Some(q) = self.fuzzy_query(query_str) else {
                return Ok((Vec::new(), 0));
            };
            q
        } else {
            self.exact_query(query_str)?
        };
        // Ask for the whole prefix up to the page end, then skip: Tantivy
        // has no "start at N" collector, and the offset is bounded by the
        // caller precisely so this stays finite.
        let wanted = offset.saturating_add(limit).max(1);
        let s = self.index.schema();
        let (top, total) = searcher
            .search(query.as_ref(), &(TopDocs::with_limit(wanted), Count))
            .map_err(|e| crate::Error::Search(format!("search: {e}")))?;
        let mut ids = Vec::new();
        for (_score, addr) in top.into_iter().skip(offset) {
            let doc: TantivyDocument = searcher
                .doc(addr)
                .map_err(|e| crate::Error::Search(format!("retrieve doc: {e}")))?;
            if let Some(v) = doc.get_first(s.id)
                && let Some(t) = v.as_str()
            {
                ids.push(t.to_string());
            }
        }
        Ok((ids, total))
    }

    /// Build the exact/full-text query over name + `alternate_name` +
    /// keywords + locality + identifiers.
    fn exact_query(&self, query_str: &str) -> Result<Box<dyn Query>> {
        let s = self.index.schema();
        let parser = QueryParser::for_index(
            self.index.index(),
            vec![
                s.name,
                s.alternate_name,
                s.keywords,
                s.locality,
                s.identifiers,
            ],
        );
        parser
            .parse_query(query_str)
            .map_err(|e| crate::Error::Search(format!("parse query: {e}")))
    }

    /// Build the fuzzy (typo-tolerant) query, or `None` when the input
    /// tokenises to nothing.
    fn fuzzy_query(&self, query_str: &str) -> Option<Box<dyn Query>> {
        let s = self.index.schema();
        let tokens = tokenise(query_str);
        if tokens.is_empty() {
            return None;
        }
        let fields = [s.name, s.alternate_name, s.keywords, s.locality];
        let mut sub: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for t in &tokens {
            for f in fields {
                let term = Term::from_field_text(f, t);
                sub.push((Occur::Should, Box::new(FuzzyTermQuery::new(term, 2, true))));
            }
        }
        Some(Box::new(BooleanQuery::new(sub)))
    }

    /// Blocking query used by the duplicate detector: fuzzy name match.
    ///
    /// # Errors
    /// Returns [`crate::Error::Search`] if the underlying search fails.
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
        self.collect_ids(&searcher, &q, limit)
    }

    /// Delete a place from the index by id.
    ///
    /// # Errors
    /// Returns [`crate::Error::Search`] if acquiring the writer, committing
    /// the delete, or reloading the reader fails.
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
    ///
    /// # Errors
    /// Returns [`crate::Error::Search`] if reading the index stats fails.
    pub fn stats(&self) -> Result<IndexStats> {
        self.index.stats()
    }

    /// Force the reader to observe the latest committed segments.
    ///
    /// # Errors
    /// Returns [`crate::Error::Search`] if the reader reload fails.
    pub fn reload(&self) -> Result<()> {
        self.index.reload()
    }

    /// Run `query` and project the top `limit` hits to their `id` strings.
    fn collect_ids(
        &self,
        searcher: &tantivy::Searcher,
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
            if let Some(v) = doc.get_first(s.id)
                && let Some(t) = v.as_str()
            {
                ids.push(t.to_string());
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
        .map(str::to_lowercase)
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

    /// `search_page` reports the true total (ignoring the page window)
    /// A single "word" (no internal separator) longer than Tantivy's
    /// default tokenizer's 40-character `RemoveLongFilter` cutoff is
    /// dropped at index time — silently, exactly like a stop word — so
    /// it can never be found even by an exact query for the same
    /// string. Pinned because it is a sharp edge for anyone building a
    /// "unique token" test fixture by concatenating a prefix directly
    /// against a 32-hex-character UUID with no separator (an easy
    /// mistake: `format!("Prefix{}", Uuid::new_v4().simple())` reliably
    /// produces a >40-char single token).
    #[test]
    fn overlong_single_token_is_not_indexed() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        // 11 + 32 = 43 chars, no separator: one token, over the cutoff.
        let overlong = format!("Offsetville{}", "a1b2c3d4e5f60708090a0b0c0d0e0f10");
        assert_eq!(overlong.len(), 43);
        eng.index_place(&place(&overlong)).unwrap();
        assert_eq!(
            eng.search(&overlong, 10).unwrap(),
            Vec::<String>::new(),
            "an over-length single token must not be findable — it was never indexed"
        );
        // The same prefix, separated so it tokenises into two ≤40-char
        // terms, IS findable — the fix a caller should reach for.
        let dir2 = TempDir::new().unwrap();
        let eng2 = SearchEngine::new(dir2.path()).unwrap();
        let separated = format!("Offsetville-{}", "a1b2c3d4e5f60708090a0b0c0d0e0f10");
        let p = place(&separated);
        eng2.index_place(&p).unwrap();
        assert_eq!(eng2.search(&separated, 10).unwrap(), vec![p.id.to_string()]);
    }

    /// and `offset` skips the requested number of results.
    #[test]
    fn search_page_offset_skips_and_reports_total() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        for n in ["Park One", "Park Two", "Park Three"] {
            eng.index_place(&place(n)).unwrap();
        }
        let (page0, total0) = eng.search_page("Park", false, 2, 0).unwrap();
        assert_eq!(page0.len(), 2);
        assert_eq!(total0, 3);
        let (page1, total1) = eng.search_page("Park", false, 2, 2).unwrap();
        assert_eq!(page1.len(), 1);
        assert_eq!(total1, 3);
        // The two pages together cover every hit exactly once (no overlap,
        // nothing missed).
        let mut all: Vec<String> = page0.into_iter().chain(page1).collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 3);
    }

    /// An offset at or past the total returns an empty page, not an error.
    #[test]
    fn search_page_offset_past_total_is_empty() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        eng.index_place(&place("Solo Park")).unwrap();
        let (page, total) = eng.search_page("Park", false, 10, 5).unwrap();
        assert!(page.is_empty());
        assert_eq!(total, 1);
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
