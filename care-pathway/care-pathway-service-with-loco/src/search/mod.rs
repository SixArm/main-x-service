//! Tantivy-backed full-text search over stored care pathways.
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

use care_pathway_matcher::{CarePathway, phonetic::soundex};
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

pub use index::{IndexStats, PathwayIndex, PathwayIndexSchema};

/// Result alias for the search layer (loco's error type, so handlers can
/// propagate with `?`).
pub type Result<T> = std::result::Result<T, Error>;

/// Build a search error. Kept in one place so every message carries the
/// same `search:` prefix in logs.
pub(crate) fn err(msg: &str) -> Error {
    Error::Message(format!("search: {msg}"))
}

/// Environment variable naming the index directory.
pub const INDEX_PATH_ENV: &str = "CARE_PATHWAY_SEARCH_INDEX_PATH";

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

/// High-level search facade over an [`PathwayIndex`].
pub struct SearchEngine {
    /// The underlying Tantivy index wrapper.
    index: PathwayIndex,
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
        let index = PathwayIndex::create_or_open(p)?;
        Ok(Self {
            index,
            index_path: p.to_string_lossy().into_owned(),
        })
    }

    /// Index (or re-index) one care pathway under its public id.
    ///
    /// Idempotent: the prior document for `pid` is deleted in the same
    /// writer batch, so an update replaces rather than duplicates. That
    /// matters because a duplicate document would let a *former* name go
    /// on matching for ever.
    ///
    /// # Errors
    ///
    /// When the writer cannot be acquired or the commit fails.
    pub fn index_pathway(&self, pid: Uuid, pathway: &CarePathway) -> Result<()> {
        let mut writer = self.index.writer(WRITER_HEAP_MB)?;
        let s = self.index.schema();
        let pid_str = pid.to_string();
        // Replace-in-place: same batch, so a crash between the two
        // cannot leave the record indexed twice.
        writer.delete_term(Term::from_field_text(s.pid, &pid_str));
        writer
            .add_document(doc!(
                s.pid => pid_str,
                s.name => pathway.name.clone(),
                s.provider_name => pathway.provider_name.clone().unwrap_or_default(),
                s.alternate_names => pathway.alternate_names.join(" "),
                s.name_phonetic => phonetic_codes(&name_text(pathway)),
                s.identifiers => identifier_text(pathway),
                s.keywords => pathway.keywords.join(" "),
                s.condition_codes => condition_text(pathway),
                s.interventions => pathway.interventions.join(" "),
                s.pathway_code => pathway.pathway_code.clone().unwrap_or_default(),
                s.provider_id => pathway.provider_id.clone().unwrap_or_default(),
                s.care_setting => pathway
                    .care_setting
                    .as_ref()
                    .map(|c| format!("{c:?}").to_lowercase())
                    .unwrap_or_default(),
                s.active => "true",
            ))
            .map_err(|e| err(&format!("add document: {e}")))?;
        writer.commit().map_err(|e| err(&format!("commit: {e}")))?;
        self.index.reload()
    }

    /// Remove a care pathway from the index by public id. Called on
    /// soft-delete (and for the duplicate side of a merge), because a
    /// soft-deleted row must stop being a search hit.
    ///
    /// # Errors
    ///
    /// When the delete commit fails.
    pub fn delete_pathway(&self, pid: Uuid) -> Result<()> {
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
            s.alternate_names,
            s.provider_name,
            s.identifiers,
            s.keywords,
            s.condition_codes,
            s.interventions,
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
        let fields = [s.name, s.alternate_names, s.provider_name, s.keywords];
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
    pub fn candidates(&self, pathway: &CarePathway, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let mut sub: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        let name_tokens = tokenise(&name_text(pathway));
        for t in &name_tokens {
            for f in [s.name, s.alternate_names, s.provider_name] {
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
        for t in tokenise(&identifier_text(pathway)) {
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
                    s.alternate_names,
                    s.provider_name,
                    s.identifiers,
                    s.keywords,
                    s.condition_codes,
                    s.interventions,
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
                let fields = [s.name, s.alternate_names, s.provider_name, s.keywords];
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

/// Every name-ish string on the pathway, space-joined — the text the
/// phonetic field and the blocking query are built from.
fn name_text(pathway: &CarePathway) -> String {
    let mut parts = vec![pathway.name.clone()];
    parts.extend(pathway.alternate_names.iter().cloned());
    parts.join(" ")
}

/// Identifier values, space-joined. Schemes are not indexed: a caller
/// searching for `NICE-NG128` should not have to know it is a guideline
/// id.
fn identifier_text(pathway: &CarePathway) -> String {
    pathway
        .identifiers
        .iter()
        .map(|i| i.value.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Target condition codes, space-joined as `system:code` pairs.
///
/// The system is included so `I63` under ICD-10 and `I63` under a local
/// scheme are distinguishable tokens, while the tokeniser still splits
/// them so a caller searching the bare code finds both.
fn condition_text(pathway: &CarePathway) -> String {
    pathway
        .condition_codes
        .iter()
        .map(|c| format!("{:?}:{}", c.system, c.code).to_lowercase())
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
    use care_pathway_matcher::{CodeSystem, ConditionCode, PathwayIdentifier};
    use tempfile::TempDir;

    /// An engine over a throwaway directory, plus the directory guard.
    fn engine_in_temp() -> (SearchEngine, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(dir.path()).unwrap();
        (engine, dir)
    }

    /// Index one pathway and return the pid it was filed under.
    fn index(engine: &SearchEngine, pathway: &CarePathway) -> Uuid {
        let pid = Uuid::new_v4();
        engine.index_pathway(pid, pathway).unwrap();
        pid
    }

    /// A fully-populated pathway.
    fn stroke() -> CarePathway {
        CarePathway {
            alternate_names: vec!["Hyperacute Stroke".into()],
            provider_name: Some("Royal Infirmary".into()),
            provider_id: Some("trust-1".into()),
            pathway_code: Some("STROKE-01".into()),
            condition_codes: vec![ConditionCode {
                system: CodeSystem::Icd10,
                code: "I63".into(),
            }],
            interventions: vec!["thrombolysis".into()],
            keywords: vec!["hyperacute".into()],
            identifiers: vec![PathwayIdentifier {
                scheme: care_pathway_matcher::IdentifierScheme::GuidelineId,
                value: "NICE-NG128".into(),
            }],
            ..CarePathway::new("Acute Stroke Care Pathway")
        }
    }

    /// An indexed pathway is found by an exact full-text query.
    #[test]
    fn index_and_exact_search() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &stroke());
        assert_eq!(engine.search("Stroke", 10).unwrap(), vec![pid.to_string()]);
    }

    /// Fuzzy search survives a typo; exact search does not.
    #[test]
    fn fuzzy_search_tolerates_typo() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &CarePathway::new("Sepsis Care Pathway"));
        assert!(engine.search("Sepsus", 10).unwrap().is_empty());
        assert_eq!(
            engine.fuzzy_search("Sepsus", 10).unwrap(),
            vec![pid.to_string()]
        );
    }

    /// The secondary fields are searchable, not just the title — the
    /// condition code especially, since it is what a pathway is *about*.
    #[test]
    fn secondary_fields_are_searchable() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &stroke()).to_string();
        for q in [
            "Hyperacute",
            "Royal Infirmary",
            "I63",
            "thrombolysis",
            "NICE-NG128",
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
        let pid = index(&engine, &CarePathway::new("Old Pathway"));
        engine
            .index_pathway(pid, &CarePathway::new("New Pathway"))
            .unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 1);
        assert_eq!(engine.search("New", 10).unwrap(), vec![pid.to_string()]);
        assert!(engine.search("Old", 10).unwrap().is_empty());
    }

    /// Deleting removes the document.
    #[test]
    fn delete_removes_from_index() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &CarePathway::new("Ephemeral Pathway"));
        engine.delete_pathway(pid).unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 0);
    }

    /// `clear` empties the index (the rebuild task's first step).
    #[test]
    fn clear_empties_the_index() {
        let (engine, _dir) = engine_in_temp();
        index(&engine, &CarePathway::new("One"));
        index(&engine, &CarePathway::new("Two"));
        engine.clear().unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 0);
    }

    /// The blocking query reaches a pathway whose title is entirely
    /// different but whose identifier matches — the deterministic
    /// short-circuit case a name-only block would silently lose.
    #[test]
    fn candidates_block_on_identifier_alone() {
        let (engine, _dir) = engine_in_temp();
        let ident = PathwayIdentifier {
            scheme: care_pathway_matcher::IdentifierScheme::GuidelineId,
            value: "NICE-NG128".into(),
        };
        let stored = CarePathway {
            identifiers: vec![ident.clone()],
            ..CarePathway::new("Wholly Unrelated Pathway")
        };
        let pid = index(&engine, &stored);
        let query = CarePathway {
            identifiers: vec![ident],
            ..CarePathway::new("Acute Stroke Care Pathway")
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
        index(&engine, &stroke());
        assert!(
            engine
                .candidates(&CarePathway::new("  "), 10)
                .unwrap()
                .is_empty()
        );
    }

    /// A query Tantivy's parser rejects falls back to a token OR rather
    /// than erroring, so a stray quote in a search box is not a 500.
    #[test]
    fn unparseable_query_falls_back_to_tokens() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &CarePathway::new("Acute Stroke Care Pathway"));
        assert_eq!(
            engine.search("\"stroke", 10).unwrap(),
            vec![pid.to_string()]
        );
    }

    /// A zero limit is not a panic (Tantivy's `TopDocs` rejects zero).
    #[test]
    fn zero_limit_returns_nothing() {
        let (engine, _dir) = engine_in_temp();
        index(&engine, &stroke());
        assert!(engine.search("Stroke", 0).unwrap().is_empty());
    }

    /// A page reports the **whole** match count, not the page length —
    /// the number `X-Total-Count` carries.
    #[test]
    fn search_page_reports_the_total_not_the_page() {
        let (engine, _dir) = engine_in_temp();
        for i in 0..5 {
            index(&engine, &CarePathway::new(format!("Paging Pathway {i}")));
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
        assert_eq!(tokenise("Stroke, Acute."), vec!["stroke", "acute"]);
        assert_eq!(tokenise("   "), Vec::<String>::new());
    }

    /// Condition codes are indexed as `system:code`, lowercased, so the
    /// tokeniser yields both the system and the bare code.
    #[test]
    fn condition_text_pairs_system_and_code() {
        assert_eq!(condition_text(&stroke()), "icd10:i63");
        assert_eq!(condition_text(&CarePathway::new("Bare")), "");
    }
}
