//! Tantivy-backed full-text search over stored cases.
//!
//! Replaces the earlier Postgres `ILIKE '%q%'` title search (spec §13
//! T-6), adding tokenised full-text, fuzzy (typo-tolerant), and
//! phonetic (Soundex) retrieval, plus the **blocking** query that gives
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
//!
//! Case data is personal data (§10 governance,
//! `agents/share/cross-service-linking.md`), but the index carries no
//! more than the record-level ABAC concealment already lets a search hit
//! resolve to: every `pid` this module returns is still filtered through
//! `crate::auth::read_visibility` by the controller before it reaches a
//! caller.

use std::path::Path;
use std::sync::OnceLock;

use case_matcher::{Case, phonetic::soundex};
use loco_rs::Error;
use tantivy::{
    TantivyDocument,
    collector::TopDocs,
    doc,
    query::{BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser, TermQuery},
    schema::{IndexRecordOption, Term, Value},
};
use uuid::Uuid;

pub mod index;

pub use index::{CaseIndex, CaseIndexSchema, IndexStats};

/// Result alias for the search layer (loco's error type, so handlers can
/// propagate with `?`).
pub type Result<T> = std::result::Result<T, Error>;

/// Build a search error. Kept in one place so every message carries the
/// same `search:` prefix in logs.
pub(crate) fn err(msg: &str) -> Error {
    Error::Message(format!("search: {msg}"))
}

/// Environment variable naming the index directory.
pub const INDEX_PATH_ENV: &str = "CASE_SEARCH_INDEX_PATH";

/// Index directory used when [`INDEX_PATH_ENV`] is unset.
pub const DEFAULT_INDEX_PATH: &str = "data/search-index";

/// Maximum Levenshtein distance for fuzzy term matching. Two edits is
/// the family-wide setting, chosen so a transposed or doubled letter
/// still matches while unrelated short titles do not.
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
    /// Soundex codes — titles that sound alike.
    Phonetic,
}

/// High-level search facade over a [`CaseIndex`].
pub struct SearchEngine {
    /// The underlying Tantivy index wrapper.
    index: CaseIndex,
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
        let index = CaseIndex::create_or_open(p)?;
        Ok(Self {
            index,
            index_path: p.to_string_lossy().into_owned(),
        })
    }

    /// Index (or re-index) one case under its public id.
    ///
    /// Idempotent: the prior document for `pid` is deleted in the same
    /// writer batch, so an update replaces rather than duplicates. That
    /// matters because a duplicate document would let a *former* title
    /// go on matching for ever.
    ///
    /// # Errors
    ///
    /// When the writer cannot be acquired or the commit fails.
    pub fn index_case(&self, pid: Uuid, case: &Case) -> Result<()> {
        let mut writer = self.index.writer(WRITER_HEAP_MB)?;
        let s = self.index.schema();
        let pid_str = pid.to_string();
        // Replace-in-place: same batch, so a crash between the two
        // cannot leave the record indexed twice.
        writer.delete_term(Term::from_field_text(s.pid, &pid_str));
        writer
            .add_document(doc!(
                s.pid => pid_str,
                s.title => case.title.clone(),
                s.alternate_titles => case.alternate_titles.join(" "),
                s.title_phonetic => phonetic_codes(&title_text(case)),
                s.identifiers => identifier_text(case),
                s.keywords => case.keywords.join(" "),
                s.subjects => case.subjects.join(" "),
                s.agency_name => case.agency_name.clone().unwrap_or_default(),
                s.case_number => case.case_number.clone().unwrap_or_default(),
                s.agency_id => case.agency_id.clone().unwrap_or_default(),
                s.case_type => case
                    .case_type
                    .as_ref()
                    .map(|t| format!("{t:?}").to_lowercase())
                    .unwrap_or_default(),
                s.status => case
                    .status
                    .as_ref()
                    .map(|t| format!("{t:?}").to_lowercase())
                    .unwrap_or_default(),
                s.active => "true",
            ))
            .map_err(|e| err(&format!("add document: {e}")))?;
        writer.commit().map_err(|e| err(&format!("commit: {e}")))?;
        self.index.reload()
    }

    /// Remove a case from the index by public id. Called on soft-delete
    /// (and for the duplicate side of a merge), because a soft-deleted
    /// row must stop being a search hit.
    ///
    /// # Errors
    ///
    /// When the delete commit fails.
    pub fn delete_case(&self, pid: Uuid) -> Result<()> {
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

    /// Full-text search over the title, identifier, keyword, subject,
    /// and agency-name fields, returning matching `pid`s ranked by
    /// relevance.
    ///
    /// A query Tantivy's parser rejects (unbalanced quotes, a bare
    /// operator) is **not** an error: it falls back to an OR over the
    /// query's tokens. A caller typing `"housing` into a search box
    /// should get results, not a 500.
    ///
    /// # Errors
    ///
    /// When the search itself fails.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let fields = vec![
            s.title,
            s.alternate_titles,
            s.agency_name,
            s.identifiers,
            s.keywords,
            s.subjects,
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
    /// [`FuzzyTermQuery`] per title-ish field, combined with `Should`.
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
        let fields = [s.title, s.alternate_titles, s.agency_name, s.keywords];
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

    /// Phonetic (Soundex) search — finds titles that *sound* like the
    /// query even when spelled differently. Exact term matching over
    /// the indexed Soundex codes, so it is cheap compared with fuzzy
    /// scanning.
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
                    Term::from_field_text(s.title_phonetic, &c.to_lowercase()),
                    IndexRecordOption::Basic,
                ));
                (Occur::Should, q)
            })
            .collect();
        self.collect_pids(&searcher, &BooleanQuery::new(sub), limit)
    }

    /// **Blocking query** for duplicate detection: the candidate set an
    /// incoming case should be scored against.
    ///
    /// Union of three retrieval routes, because a duplicate can present
    /// as any one of them:
    ///
    /// 1. fuzzy title / alternate-title tokens (a re-titled or
    ///    abbreviated case),
    /// 2. exact identifier values (docket / external-case-id / URI /
    ///    UUID — the matcher's deterministic short-circuits; a record
    ///    filed under a completely different title is *only* reachable
    ///    this way),
    /// 3. phonetic codes of the title (a title transcribed by ear).
    ///
    /// Deliberately **not** filtered by agency: the matcher scores the
    /// agency-scoped case number itself, and a missing or wrong agency
    /// on one side is exactly the data-entry error deduplication exists
    /// to catch. The field is indexed for a future explicit filter.
    ///
    /// # Errors
    ///
    /// When the search fails.
    pub fn candidates(&self, case: &Case, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let mut sub: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        let title_tokens = tokenise(&title_text(case));
        for t in &title_tokens {
            for f in [s.title, s.alternate_titles, s.agency_name] {
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
                        Term::from_field_text(s.title_phonetic, &code.to_lowercase()),
                        IndexRecordOption::Basic,
                    )),
                ));
            }
        }
        for t in tokenise(&identifier_text(case)) {
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
                    s.title,
                    s.alternate_titles,
                    s.agency_name,
                    s.identifiers,
                    s.keywords,
                    s.subjects,
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
                let fields = [s.title, s.alternate_titles, s.agency_name, s.keywords];
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
                            Term::from_field_text(s.title_phonetic, &c.to_lowercase()),
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

/// Every title-ish string on the case, space-joined — the text the
/// phonetic field and the blocking query are built from.
fn title_text(case: &Case) -> String {
    let mut parts = vec![case.title.clone()];
    parts.extend(case.alternate_titles.iter().cloned());
    parts.join(" ")
}

/// Identifier values, space-joined. Schemes are not indexed: a caller
/// searching for a docket number should not have to know it is a
/// docket number.
fn identifier_text(case: &Case) -> String {
    case.identifiers
        .iter()
        .map(|i| i.value.as_str())
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
/// Punctuation and underscores separate, so `"Smith, J."` yields
/// `["smith", "j"]`.
fn tokenise(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use case_matcher::{CaseIdentifier, CaseStatus, CaseType, IdentifierScheme};
    use tempfile::TempDir;

    /// An engine over a throwaway directory, plus the directory guard.
    fn engine_in_temp() -> (SearchEngine, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(dir.path()).unwrap();
        (engine, dir)
    }

    /// Index one case and return the pid it was filed under.
    fn index(engine: &SearchEngine, case: &Case) -> Uuid {
        let pid = Uuid::new_v4();
        engine.index_case(pid, case).unwrap();
        pid
    }

    /// A fully-populated case.
    fn housing_case() -> Case {
        Case {
            alternate_titles: vec!["HB Appeal".into()],
            agency_name: Some("Department for Work and Pensions".into()),
            agency_id: Some("dwp".into()),
            case_number: Some("HB-2024-0007".into()),
            case_type: Some(CaseType::Benefit),
            status: Some(CaseStatus::Open),
            subjects: vec!["person:abc".into()],
            keywords: vec!["housing".into(), "benefit".into()],
            identifiers: vec![CaseIdentifier {
                scheme: IdentifierScheme::Docket,
                value: "CV-2024-001234".into(),
            }],
            ..Case::new("Housing benefit appeal")
        }
    }

    /// An indexed case is found by an exact full-text query.
    #[test]
    fn index_and_exact_search() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &housing_case());
        assert_eq!(engine.search("Housing", 10).unwrap(), vec![pid.to_string()]);
    }

    /// Fuzzy search survives a typo; exact search does not.
    #[test]
    fn fuzzy_search_tolerates_typo() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Case::new("Immigration appeal"));
        assert!(engine.search("Immigraton", 10).unwrap().is_empty());
        assert_eq!(
            engine.fuzzy_search("Immigraton", 10).unwrap(),
            vec![pid.to_string()]
        );
    }

    /// The secondary fields are searchable, not just the title — the
    /// involved-party subject especially, since it is who a case is
    /// *about*.
    #[test]
    fn secondary_fields_are_searchable() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &housing_case()).to_string();
        for q in [
            "HB Appeal",
            "Department for Work and Pensions",
            "person:abc",
            "housing",
            "CV-2024-001234",
        ] {
            assert_eq!(
                engine.search(q, 10).unwrap(),
                vec![pid.clone()],
                "query {q}"
            );
        }
    }

    /// Re-indexing the same pid replaces the document.
    #[test]
    fn reindex_replaces_rather_than_duplicates() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Case::new("Old Title"));
        engine.index_case(pid, &Case::new("New Title")).unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 1);
        assert_eq!(engine.search("New", 10).unwrap(), vec![pid.to_string()]);
        assert!(engine.search("Old", 10).unwrap().is_empty());
    }

    /// Deleting removes the document.
    #[test]
    fn delete_removes_from_index() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Case::new("Ephemeral Case"));
        engine.delete_case(pid).unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 0);
    }

    /// `clear` empties the index (the rebuild task's first step).
    #[test]
    fn clear_empties_the_index() {
        let (engine, _dir) = engine_in_temp();
        index(&engine, &Case::new("One"));
        index(&engine, &Case::new("Two"));
        engine.clear().unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 0);
    }

    /// The blocking query reaches a case whose title is entirely
    /// different but whose identifier matches — the deterministic
    /// short-circuit case a title-only block would silently lose.
    #[test]
    fn candidates_block_on_identifier_alone() {
        let (engine, _dir) = engine_in_temp();
        let ident = CaseIdentifier {
            scheme: IdentifierScheme::Docket,
            value: "CV-2024-001234".into(),
        };
        let stored = Case {
            identifiers: vec![ident.clone()],
            ..Case::new("Wholly Unrelated Case")
        };
        let pid = index(&engine, &stored);
        let query = Case {
            identifiers: vec![ident],
            ..Case::new("Housing benefit appeal")
        };
        assert_eq!(
            engine.candidates(&query, 10).unwrap(),
            vec![pid.to_string()]
        );
    }

    /// A query with nothing to block on returns no candidates.
    #[test]
    fn candidates_of_an_empty_query_are_empty() {
        let (engine, _dir) = engine_in_temp();
        index(&engine, &housing_case());
        assert!(engine.candidates(&Case::new("  "), 10).unwrap().is_empty());
    }

    /// A query Tantivy's parser rejects falls back to a token OR rather
    /// than erroring, so a stray quote in a search box is not a 500.
    #[test]
    fn unparseable_query_falls_back_to_tokens() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Case::new("Housing benefit appeal"));
        assert_eq!(
            engine.search("\"housing", 10).unwrap(),
            vec![pid.to_string()]
        );
    }

    /// A zero limit is not a panic (Tantivy's `TopDocs` rejects zero).
    #[test]
    fn zero_limit_returns_nothing() {
        let (engine, _dir) = engine_in_temp();
        index(&engine, &housing_case());
        assert!(engine.search("Housing", 0).unwrap().is_empty());
    }

    /// A page reports the **whole** match count, not the page length —
    /// the number `X-Total-Count` carries.
    #[test]
    fn search_page_reports_the_total_not_the_page() {
        let (engine, _dir) = engine_in_temp();
        for i in 0..5 {
            index(&engine, &Case::new(format!("Paging Case {i}")));
        }
        let (hits, total) = engine
            .search_page("Paging", SearchMode::Exact, 2, 1)
            .unwrap();
        assert_eq!(hits.len(), 2, "the page is two hits");
        assert_eq!(total, 5, "the total ignores the window");
    }

    /// `tokenise` lowercases and splits on punctuation.
    #[test]
    fn tokenise_splits_on_punctuation() {
        assert_eq!(tokenise("Housing, Benefit."), vec!["housing", "benefit"]);
        assert_eq!(tokenise("   "), Vec::<String>::new());
    }
}
