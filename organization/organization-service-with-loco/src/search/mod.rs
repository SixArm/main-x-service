//! Tantivy-backed full-text search over stored organizations.
//!
//! Replaces the earlier Postgres `ILIKE '%q%'` name search (spec §13),
//! adding tokenised full-text, fuzzy (typo-tolerant), and phonetic
//! (Soundex) retrieval, plus the **blocking** query that gives
//! `check-duplicates` a candidate set instead of a capped full scan.
//!
//! ## The index is a candidate generator, not a source of truth
//!
//! Every hit is a `pid` that the caller resolves against Postgres, and
//! soft-deleted rows are invisible to that lookup. So an index that has
//! drifted **stale** degrades gracefully: a document for a since-deleted
//! row simply fails to resolve and is dropped from the response — it can
//! never resurrect a record or leak a deleted one. The failure mode that
//! does matter is the opposite one (a *missing* document hides a live
//! record from search), which is why indexing failures are logged loudly
//! and [`crate::tasks::search::reindex`] exists to rebuild from the
//! database.
//!
//! Indexing is wired into `src/streaming.rs` — the one seam both the
//! native and the FHIR controllers write through — so no write path can
//! quietly skip it.

use std::path::Path;
use std::sync::OnceLock;

use loco_rs::Error;
use organization_matcher::{Organization, phonetic::soundex};
use tantivy::{
    TantivyDocument,
    collector::TopDocs,
    doc,
    query::{BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser, TermQuery},
    schema::{IndexRecordOption, Term, Value},
};
use uuid::Uuid;

pub mod index;

pub use index::{IndexStats, OrgIndex, OrgIndexSchema};

/// Result alias for the search layer (loco's error type, so handlers can
/// propagate with `?`).
pub type Result<T> = std::result::Result<T, Error>;

/// Build a search error. Kept in one place so every message carries the
/// same `search:` prefix in logs.
pub(crate) fn err(msg: &str) -> Error {
    Error::Message(format!("search: {msg}"))
}

/// Environment variable naming the index directory.
pub const INDEX_PATH_ENV: &str = "ORGANIZATION_SEARCH_INDEX_PATH";

/// Index directory used when [`INDEX_PATH_ENV`] is unset.
pub const DEFAULT_INDEX_PATH: &str = "data/search-index";

/// Maximum Levenshtein distance for fuzzy term matching. Two edits is
/// the family-wide setting (person / course), chosen so a transposed or
/// doubled letter still matches while unrelated short names do not.
const FUZZY_DISTANCE: u8 = 2;

/// Writer heap budget, in megabytes. Writes here are single-record, so
/// the buffer only has to hold one document plus Tantivy's overhead.
const WRITER_HEAP_MB: usize = 50;

/// Process-wide engine, initialised on first use.
///
/// `None` means the index could not be opened; that is recorded once
/// (not per request) and callers surface it rather than silently
/// returning no results — an empty result set and a broken index must
/// not look the same to an operator.
static ENGINE: OnceLock<Option<SearchEngine>> = OnceLock::new();

/// The configured index directory: [`INDEX_PATH_ENV`], else
/// [`DEFAULT_INDEX_PATH`].
#[must_use]
pub fn index_path() -> String {
    std::env::var(INDEX_PATH_ENV).unwrap_or_else(|_| DEFAULT_INDEX_PATH.to_string())
}

/// The process-wide search engine, or `None` when the index could not be
/// opened (the failure is logged once, at first use).
pub fn engine() -> Option<&'static SearchEngine> {
    ENGINE
        .get_or_init(|| {
            let path = index_path();
            match SearchEngine::new(&path) {
                Ok(engine) => {
                    tracing::info!(path = %path, "search index opened");
                    Some(engine)
                }
                Err(e) => {
                    tracing::error!(path = %path, error = %e, "search index unavailable");
                    None
                }
            }
        })
        .as_ref()
}

/// Which retrieval route a search uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// Tokenised full-text over every indexed field.
    Exact,
    /// Typo-tolerant term matching (Levenshtein ≤ [`FUZZY_DISTANCE`]).
    Fuzzy,
    /// Soundex codes — names that sound alike.
    Phonetic,
}

/// High-level search facade over an [`OrgIndex`].
pub struct SearchEngine {
    /// The underlying Tantivy index wrapper.
    index: OrgIndex,
    /// Filesystem path the index lives at (diagnostics / reopen).
    pub index_path: String,
}

impl SearchEngine {
    /// Open or create the index under `path`, creating the directory if
    /// it does not yet exist.
    ///
    /// # Errors
    ///
    /// When the directory cannot be created or the index cannot be
    /// opened/created.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        std::fs::create_dir_all(p).map_err(|e| err(&format!("ensure index dir: {e}")))?;
        let index = OrgIndex::create_or_open(p)?;
        Ok(Self {
            index,
            index_path: p.to_string_lossy().into_owned(),
        })
    }

    /// Index (or re-index) one organization under its public id.
    ///
    /// Idempotent: the prior document for `pid` is deleted in the same
    /// writer batch, so an update replaces rather than duplicates. That
    /// matters because a duplicate document would let a *former* name go
    /// on matching for ever.
    ///
    /// # Errors
    ///
    /// When the writer cannot be acquired or the commit fails.
    pub fn index_organization(&self, pid: Uuid, org: &Organization) -> Result<()> {
        let mut writer = self.index.writer(WRITER_HEAP_MB)?;
        let s = self.index.schema();
        let pid_str = pid.to_string();
        // Replace-in-place: same batch, so a crash between the two
        // cannot leave the record indexed twice.
        writer.delete_term(Term::from_field_text(s.pid, &pid_str));
        writer
            .add_document(doc!(
                s.pid => pid_str,
                s.name => org.name.clone(),
                s.legal_name => org.legal_name.clone().unwrap_or_default(),
                s.alternate_names => org.alternate_names.join(" "),
                s.name_phonetic => phonetic_codes(&name_text(org)),
                s.identifiers => identifier_text(org),
                s.keywords => org.keywords.join(" "),
                s.address => address_text(org),
                s.url => org.url.clone().unwrap_or_default(),
                s.jurisdiction => org.jurisdiction.clone().unwrap_or_default(),
                s.active => "true",
            ))
            .map_err(|e| err(&format!("add document: {e}")))?;
        writer.commit().map_err(|e| err(&format!("commit: {e}")))?;
        self.index.reload()
    }

    /// Remove an organization from the index by public id. Called on
    /// soft-delete (and for the duplicate side of a merge), because a
    /// soft-deleted row must stop being a search hit.
    ///
    /// # Errors
    ///
    /// When the delete commit fails.
    pub fn delete_organization(&self, pid: Uuid) -> Result<()> {
        let mut writer = self.index.writer(WRITER_HEAP_MB)?;
        let s = self.index.schema();
        writer.delete_term(Term::from_field_text(s.pid, &pid.to_string()));
        writer
            .commit()
            .map_err(|e| err(&format!("commit delete: {e}")))?;
        self.index.reload()
    }

    /// Drop every document. Used by the rebuild task before replaying
    /// the database, so a record deleted while the index was offline
    /// does not survive the rebuild.
    ///
    /// # Errors
    ///
    /// When the delete-all commit fails.
    pub fn clear(&self) -> Result<()> {
        let mut writer = self.index.writer(WRITER_HEAP_MB)?;
        writer
            .delete_all_documents()
            .map_err(|e| err(&format!("delete all: {e}")))?;
        writer
            .commit()
            .map_err(|e| err(&format!("commit clear: {e}")))?;
        self.index.reload()
    }

    /// Full-text search over the name, identifier, keyword, address, and
    /// URL fields, returning matching `pid`s ranked by relevance.
    ///
    /// A query Tantivy's parser rejects (unbalanced quotes, a bare
    /// operator) is **not** an error: it falls back to an OR over the
    /// query's tokens. A caller typing `"acme` into a search box should
    /// get results, not a 500.
    ///
    /// # Errors
    ///
    /// When the search itself fails.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let fields = vec![
            s.name,
            s.legal_name,
            s.alternate_names,
            s.identifiers,
            s.keywords,
            s.address,
            s.url,
        ];
        let parser = QueryParser::for_index(self.index.index(), fields.clone());
        match parser.parse_query(query_str) {
            Ok(query) => self.collect_pids(&searcher, query.as_ref(), limit),
            Err(e) => {
                tracing::debug!(error = %e, "query parse failed; falling back to token OR");
                let tokens = tokenise(query_str);
                if tokens.is_empty() {
                    return Ok(Vec::new());
                }
                let sub: Vec<(Occur, Box<dyn Query>)> = tokens
                    .iter()
                    .flat_map(|t| {
                        fields.iter().map(move |f| {
                            let q: Box<dyn Query> = Box::new(TermQuery::new(
                                Term::from_field_text(*f, t),
                                IndexRecordOption::Basic,
                            ));
                            (Occur::Should, q)
                        })
                    })
                    .collect();
                self.collect_pids(&searcher, &BooleanQuery::new(sub), limit)
            }
        }
    }

    /// Fuzzy search — tolerates typos. Each query token becomes one
    /// [`FuzzyTermQuery`] per name-ish field, combined with `Should`.
    /// A query that tokenises to nothing returns an empty result rather
    /// than an error (family behaviour).
    ///
    /// # Errors
    ///
    /// When the search fails.
    pub fn fuzzy_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let tokens = tokenise(query_str);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let fields = [s.name, s.legal_name, s.alternate_names, s.keywords];
        let mut sub: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for t in &tokens {
            for f in fields {
                let term = Term::from_field_text(f, t);
                sub.push((
                    Occur::Should,
                    Box::new(FuzzyTermQuery::new(term, FUZZY_DISTANCE, true)),
                ));
            }
        }
        self.collect_pids(&searcher, &BooleanQuery::new(sub), limit)
    }

    /// Phonetic (Soundex) search — finds names that *sound* like the
    /// query even when spelled differently ("Kwik" ↔ "Quick"). Exact
    /// term matching over the indexed Soundex codes, so it is cheap
    /// compared with fuzzy scanning.
    ///
    /// # Errors
    ///
    /// When the search fails.
    pub fn phonetic_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let codes = tokenise(query_str)
            .iter()
            .filter_map(|t| soundex(t))
            .collect::<Vec<_>>();
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        let sub: Vec<(Occur, Box<dyn Query>)> = codes
            .iter()
            .map(|c| {
                let q: Box<dyn Query> = Box::new(TermQuery::new(
                    // Soundex codes are indexed through the default
                    // tokeniser, which lowercases — match that here or
                    // the term never resolves.
                    Term::from_field_text(s.name_phonetic, &c.to_lowercase()),
                    IndexRecordOption::Basic,
                ));
                (Occur::Should, q)
            })
            .collect();
        self.collect_pids(&searcher, &BooleanQuery::new(sub), limit)
    }

    /// **Blocking query** for duplicate detection: the candidate set an
    /// incoming organization should be scored against.
    ///
    /// Union of three retrieval routes, because a duplicate can present
    /// as any one of them:
    ///
    /// 1. fuzzy name / legal-name / alternate-name tokens (a re-spelled
    ///    or abbreviated name),
    /// 2. exact identifier values (LEI / DUNS / VAT / tax id — the
    ///    matcher's deterministic short-circuits; a record filed under a
    ///    completely different name is *only* reachable this way),
    /// 3. phonetic codes of the name (a name transcribed by ear).
    ///
    /// Deliberately **not** filtered by jurisdiction: the matcher scores
    /// jurisdiction itself, and a missing or wrong jurisdiction on one
    /// side is exactly the data-entry error deduplication exists to
    /// catch. The field is indexed for a future explicit filter.
    ///
    /// # Errors
    ///
    /// When the search fails.
    pub fn candidates(&self, org: &Organization, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let mut sub: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        let name_tokens = tokenise(&name_text(org));
        for t in &name_tokens {
            for f in [s.name, s.legal_name, s.alternate_names] {
                sub.push((
                    Occur::Should,
                    Box::new(FuzzyTermQuery::new(
                        Term::from_field_text(f, t),
                        FUZZY_DISTANCE,
                        true,
                    )),
                ));
            }
            if let Some(code) = soundex(t) {
                sub.push((
                    Occur::Should,
                    Box::new(TermQuery::new(
                        Term::from_field_text(s.name_phonetic, &code.to_lowercase()),
                        IndexRecordOption::Basic,
                    )),
                ));
            }
        }
        for t in tokenise(&identifier_text(org)) {
            sub.push((
                Occur::Should,
                Box::new(TermQuery::new(
                    Term::from_field_text(s.identifiers, &t),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if sub.is_empty() {
            return Ok(Vec::new());
        }
        self.collect_pids(&searcher, &BooleanQuery::new(sub), limit)
    }

    /// Document and segment counts for the live index.
    ///
    /// # Errors
    ///
    /// When index stats cannot be read.
    pub fn stats(&self) -> Result<IndexStats> {
        self.index.stats()
    }

    /// Force the reader to observe the latest committed segments.
    ///
    /// # Errors
    ///
    /// When the reader fails to reload.
    pub fn reload(&self) -> Result<()> {
        self.index.reload()
    }

    /// One page of hits **plus the true total**: `(pids, total)`.
    ///
    /// The total comes from Tantivy's `Count` collector rather than the
    /// page length, because a page can never tell a caller how much
    /// there is — which is the whole point of `X-Total-Count`
    /// (`agents/share/restful.md`). `mode` picks the same three
    /// retrieval routes the un-paged methods expose.
    ///
    /// # Errors
    ///
    /// When the search fails.
    pub fn search_page(
        &self,
        query_str: &str,
        mode: SearchMode,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<String>, usize)> {
        let searcher = self.index.reader().searcher();
        let Some(query) = self.build_query(query_str, mode) else {
            return Ok((Vec::new(), 0));
        };
        // Ask for the whole prefix up to the page end, then skip: Tantivy
        // has no "start at N" collector, and the offset is bounded by the
        // caller precisely so this stays finite.
        let wanted = offset.saturating_add(limit);
        let (top, total) = searcher
            .search(
                query.as_ref(),
                &(
                    tantivy::collector::TopDocs::with_limit(wanted.max(1)),
                    tantivy::collector::Count,
                ),
            )
            .map_err(|e| err(&format!("search: {e}")))?;
        let s = self.index.schema();
        let mut pids = Vec::new();
        for (_score, addr) in top.into_iter().skip(offset) {
            let doc: TantivyDocument = searcher
                .doc(addr)
                .map_err(|e| err(&format!("retrieve doc: {e}")))?;
            if let Some(v) = doc.get_first(s.pid)
                && let Some(t) = v.as_str()
            {
                pids.push(t.to_string());
            }
        }
        Ok((pids, total))
    }

    /// Build the query for one retrieval mode, or `None` when the input
    /// tokenises to nothing (an empty query matches nothing rather than
    /// everything).
    fn build_query(&self, query_str: &str, mode: SearchMode) -> Option<Box<dyn Query>> {
        let s = self.index.schema();
        match mode {
            SearchMode::Exact => {
                let fields = vec![
                    s.name,
                    s.legal_name,
                    s.alternate_names,
                    s.identifiers,
                    s.keywords,
                    s.address,
                    s.url,
                ];
                let parser = QueryParser::for_index(self.index.index(), fields.clone());
                if let Ok(query) = parser.parse_query(query_str) {
                    Some(query)
                } else {
                    // Same fallback as `search`: a query the parser
                    // rejects becomes an OR over its tokens.
                    let tokens = tokenise(query_str);
                    if tokens.is_empty() {
                        return None;
                    }
                    let sub: Vec<(Occur, Box<dyn Query>)> = tokens
                        .iter()
                        .flat_map(|t| {
                            fields.iter().map(move |f| {
                                let q: Box<dyn Query> = Box::new(TermQuery::new(
                                    Term::from_field_text(*f, t),
                                    IndexRecordOption::Basic,
                                ));
                                (Occur::Should, q)
                            })
                        })
                        .collect();
                    Some(Box::new(BooleanQuery::new(sub)))
                }
            }
            SearchMode::Fuzzy => {
                let tokens = tokenise(query_str);
                if tokens.is_empty() {
                    return None;
                }
                let fields = [s.name, s.legal_name, s.alternate_names, s.keywords];
                let mut sub: Vec<(Occur, Box<dyn Query>)> = Vec::new();
                for t in &tokens {
                    for f in fields {
                        sub.push((
                            Occur::Should,
                            Box::new(FuzzyTermQuery::new(
                                Term::from_field_text(f, t),
                                FUZZY_DISTANCE,
                                true,
                            )),
                        ));
                    }
                }
                Some(Box::new(BooleanQuery::new(sub)))
            }
            SearchMode::Phonetic => {
                let codes: Vec<String> = tokenise(query_str)
                    .iter()
                    .filter_map(|t| soundex(t))
                    .collect();
                if codes.is_empty() {
                    return None;
                }
                let sub: Vec<(Occur, Box<dyn Query>)> = codes
                    .iter()
                    .map(|c| {
                        let q: Box<dyn Query> = Box::new(TermQuery::new(
                            Term::from_field_text(s.name_phonetic, &c.to_lowercase()),
                            IndexRecordOption::Basic,
                        ));
                        (Occur::Should, q)
                    })
                    .collect();
                Some(Box::new(BooleanQuery::new(sub)))
            }
        }
    }

    /// Run `query` and project the top `limit` hits down to their stored
    /// `pid` strings, dropping any document missing one.
    fn collect_pids(
        &self,
        searcher: &tantivy::Searcher,
        query: &dyn Query,
        limit: usize,
    ) -> Result<Vec<String>> {
        // `TopDocs::with_limit` panics on zero, so a caller asking for
        // nothing gets nothing rather than a crash.
        if limit == 0 {
            return Ok(Vec::new());
        }
        let s = self.index.schema();
        let top = searcher
            .search(query, &TopDocs::with_limit(limit))
            .map_err(|e| err(&format!("search: {e}")))?;
        let mut pids = Vec::with_capacity(top.len());
        for (_score, addr) in top {
            let doc: TantivyDocument = searcher
                .doc(addr)
                .map_err(|e| err(&format!("retrieve doc: {e}")))?;
            if let Some(v) = doc.get_first(s.pid)
                && let Some(t) = v.as_str()
            {
                pids.push(t.to_string());
            }
        }
        Ok(pids)
    }
}

/// Every name-ish string on the organization, space-joined — the text
/// the phonetic field and the blocking query are built from.
fn name_text(org: &Organization) -> String {
    let mut parts = vec![org.name.clone()];
    if let Some(legal) = &org.legal_name {
        parts.push(legal.clone());
    }
    parts.extend(org.alternate_names.iter().cloned());
    parts.join(" ")
}

/// Identifier values, space-joined. Schemes are not indexed: a caller
/// searching for `5493001KJTIIGC8Y1R12` should not have to know it is an
/// LEI.
fn identifier_text(org: &Organization) -> String {
    org.identifiers
        .iter()
        .map(|i| i.value.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Postal-address parts, space-joined (empty when there is no address).
fn address_text(org: &Organization) -> String {
    let Some(a) = &org.address else {
        return String::new();
    };
    [
        a.street_address.as_deref(),
        a.locality.as_deref(),
        a.region.as_deref(),
        a.postal_code.as_deref(),
        a.country.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
}

/// Soundex code for each token of `text`, space-joined. Tokens with no
/// ASCII letter (a pure number) encode to nothing and are dropped.
fn phonetic_codes(text: &str) -> String {
    tokenise(text)
        .iter()
        .filter_map(|t| soundex(t))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Split a string into lowercase alphanumeric tokens, dropping empties.
/// Punctuation and underscores separate, so `"Acme, Inc."` yields
/// `["acme", "inc"]`.
fn tokenise(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use organization_matcher::{IdentifierScheme, OrgIdentifier, PostalAddress};
    use tempfile::TempDir;

    /// An engine over a throwaway directory, plus the directory guard
    /// (dropping it removes the index).
    fn engine_in_temp() -> (SearchEngine, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(dir.path()).unwrap();
        (engine, dir)
    }

    /// Index one organization and return the pid it was filed under.
    fn index(engine: &SearchEngine, org: &Organization) -> Uuid {
        let pid = Uuid::new_v4();
        engine.index_organization(pid, org).unwrap();
        pid
    }

    /// An indexed organization is found by an exact full-text query.
    #[test]
    fn index_and_exact_search() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Organization::new("Acme Corporation"));
        assert_eq!(
            engine.search("Acme", 10).unwrap(),
            vec![pid.to_string()],
            "exact term must retrieve the record"
        );
    }

    /// Fuzzy search survives a single-character typo; exact search does
    /// not (which is why both exist).
    #[test]
    fn fuzzy_search_tolerates_typo() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Organization::new("Contoso"));
        assert!(engine.search("Contso", 10).unwrap().is_empty());
        assert_eq!(
            engine.fuzzy_search("Contso", 10).unwrap(),
            vec![pid.to_string()]
        );
    }

    /// Phonetic search finds a name spelled differently but sounding
    /// the same. Note Soundex keeps the *first letter* literally, so it
    /// pairs `Smyth`/`Smith` (both `S530`) but never `Kwik`/`Quick`.
    #[test]
    fn phonetic_search_matches_homophone() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Organization::new("Smyth Industries"));
        assert!(
            engine.search("Smith", 10).unwrap().is_empty(),
            "the spelling differs, so exact search cannot find it"
        );
        assert_eq!(
            engine.phonetic_search("Smith", 10).unwrap(),
            vec![pid.to_string()],
            "Smyth and Smith share the Soundex code S530"
        );
    }

    /// Alternate names, identifier values, keywords, and address parts
    /// are all retrievable — not just the primary name.
    #[test]
    fn secondary_fields_are_searchable() {
        let (engine, _dir) = engine_in_temp();
        let org = Organization {
            legal_name: Some("Acme Incorporated".into()),
            alternate_names: vec!["Roadrunner Supplies".into()],
            identifiers: vec![OrgIdentifier {
                scheme: IdentifierScheme::Lei,
                value: "5493001KJTIIGC8Y1R12".into(),
            }],
            keywords: vec!["anvils".into()],
            address: Some(PostalAddress {
                locality: Some("Sedona".into()),
                ..Default::default()
            }),
            ..Organization::new("Acme")
        };
        let pid = index(&engine, &org).to_string();
        for q in [
            "Roadrunner",
            "Incorporated",
            "5493001KJTIIGC8Y1R12",
            "anvils",
            "Sedona",
        ] {
            assert_eq!(
                engine.search(q, 10).unwrap(),
                vec![pid.clone()],
                "query {q}"
            );
        }
    }

    /// Re-indexing the same pid replaces the document: the new name is
    /// found, the old one is not, and the count stays at one.
    #[test]
    fn reindex_replaces_rather_than_duplicates() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Organization::new("Old Name Ltd"));
        engine
            .index_organization(pid, &Organization::new("New Name Ltd"))
            .unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 1);
        assert_eq!(engine.search("New", 10).unwrap(), vec![pid.to_string()]);
        assert!(
            engine.search("Old", 10).unwrap().is_empty(),
            "the superseded name must stop matching"
        );
    }

    /// Deleting removes the document from the index.
    #[test]
    fn delete_removes_from_index() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Organization::new("Ephemeral GmbH"));
        assert_eq!(engine.stats().unwrap().num_docs, 1);
        engine.delete_organization(pid).unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 0);
        assert!(engine.search("Ephemeral", 10).unwrap().is_empty());
    }

    /// `clear` empties the index (the rebuild task's first step).
    #[test]
    fn clear_empties_the_index() {
        let (engine, _dir) = engine_in_temp();
        index(&engine, &Organization::new("One"));
        index(&engine, &Organization::new("Two"));
        assert_eq!(engine.stats().unwrap().num_docs, 2);
        engine.clear().unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 0);
    }

    /// The blocking query reaches a record whose name is entirely
    /// different but whose identifier matches — the deterministic
    /// short-circuit case a name-only block would silently lose.
    #[test]
    fn candidates_block_on_identifier_alone() {
        let (engine, _dir) = engine_in_temp();
        let lei = OrgIdentifier {
            scheme: IdentifierScheme::Lei,
            value: "5493001KJTIIGC8Y1R12".into(),
        };
        let stored = Organization {
            identifiers: vec![lei.clone()],
            ..Organization::new("Wholly Unrelated Trading Company")
        };
        let pid = index(&engine, &stored);
        let query = Organization {
            identifiers: vec![lei],
            ..Organization::new("Acme")
        };
        assert_eq!(
            engine.candidates(&query, 10).unwrap(),
            vec![pid.to_string()]
        );
    }

    /// The blocking query also reaches a misspelled name.
    #[test]
    fn candidates_block_on_fuzzy_name() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Organization::new("Northwind Traders"));
        let query = Organization::new("Northwnd Traders");
        assert!(
            engine
                .candidates(&query, 10)
                .unwrap()
                .contains(&pid.to_string())
        );
    }

    /// A query with nothing to block on returns no candidates rather
    /// than every record (an empty `BooleanQuery` matches nothing, but
    /// the explicit guard makes that a contract rather than a
    /// coincidence).
    #[test]
    fn candidates_of_an_empty_query_are_empty() {
        let (engine, _dir) = engine_in_temp();
        index(&engine, &Organization::new("Acme"));
        assert!(
            engine
                .candidates(&Organization::new("  "), 10)
                .unwrap()
                .is_empty()
        );
    }

    /// A query Tantivy's parser rejects falls back to a token OR rather
    /// than erroring, so a stray quote in a search box is not a 500.
    #[test]
    fn unparseable_query_falls_back_to_tokens() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Organization::new("Acme Corporation"));
        assert_eq!(engine.search("\"acme", 10).unwrap(), vec![pid.to_string()]);
    }

    /// A zero limit is not a panic (Tantivy's `TopDocs` rejects zero).
    #[test]
    fn zero_limit_returns_nothing() {
        let (engine, _dir) = engine_in_temp();
        index(&engine, &Organization::new("Acme"));
        assert!(engine.search("Acme", 0).unwrap().is_empty());
    }

    /// `tokenise` lowercases and splits on punctuation.
    #[test]
    fn tokenise_splits_on_punctuation() {
        assert_eq!(tokenise("Acme, Inc."), vec!["acme", "inc"]);
        assert_eq!(tokenise("   "), Vec::<String>::new());
    }

    /// The phonetic field holds one Soundex code per name token, and
    /// digits (which have no code) are dropped.
    #[test]
    fn phonetic_codes_encode_each_token() {
        assert_eq!(phonetic_codes("Smith Smyth"), "S530 S530");
        assert_eq!(phonetic_codes("123"), "");
    }

    /// Address flattening keeps every populated part and skips the rest.
    #[test]
    fn address_text_joins_populated_parts() {
        let org = Organization {
            address: Some(PostalAddress {
                locality: Some("Sedona".into()),
                country: Some("US".into()),
                ..Default::default()
            }),
            ..Organization::new("Acme")
        };
        assert_eq!(address_text(&org), "Sedona US");
        assert_eq!(address_text(&Organization::new("Acme")), "");
    }

    /// The default index path is used when the env var is unset.
    #[test]
    fn index_path_falls_back_to_the_default() {
        // Only assert the fallback when the variable is genuinely unset,
        // so this passes both locally and under a configured CI.
        if std::env::var(INDEX_PATH_ENV).is_err() {
            assert_eq!(index_path(), DEFAULT_INDEX_PATH);
        }
    }
}
