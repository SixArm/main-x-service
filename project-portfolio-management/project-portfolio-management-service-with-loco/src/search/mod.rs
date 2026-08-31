//! Tantivy-backed full-text search over stored plans.
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
//! Indexing is wired into `src/streaming.rs` — the one seam the plan
//! controller writes through — so no write path can quietly skip it.
//!
//! ## `kind` is a search filter, never a dedup gate
//!
//! `kind` is indexed (`src/search/index.rs`) so `search_page` can accept
//! an optional exact filter — a caller narrowing a search to one kind
//! (`?kind=project`). [`SearchEngine::candidates`] (the `check-duplicates`
//! blocking query) deliberately never applies it: the embedded matcher is
//! kind-agnostic by design (two plans with different `kind` labels may
//! still be the same identity), and gating duplicate detection by kind
//! would silently reintroduce the per-kind collection boundary the
//! service's data model removed. See `AGENTS.md` golden rule 5.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use loco_rs::Error;
use project_portfolio_management_matcher::{Plan, phonetic::soundex};
use tantivy::{
    IndexWriter, TantivyDocument,
    collector::TopDocs,
    doc,
    query::{BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser, TermQuery},
    schema::{IndexRecordOption, Term, Value},
};
use uuid::Uuid;

pub mod index;

pub use index::{IndexStats, PlanIndex, PlanIndexSchema};

/// Result alias for the search layer (loco's error type, so handlers can
/// propagate with `?`).
pub type Result<T> = std::result::Result<T, Error>;

/// Build a search error. Kept in one place so every message carries the
/// same `search:` prefix in logs.
pub(crate) fn err(msg: &str) -> Error {
    Error::Message(format!("search: {msg}"))
}

/// Environment variable naming the index directory.
pub const INDEX_PATH_ENV: &str = "PROJECT_PORTFOLIO_MANAGEMENT_SEARCH_INDEX_PATH";

/// Index directory used when [`INDEX_PATH_ENV`] is unset.
pub const DEFAULT_INDEX_PATH: &str = "data/search-index";

/// Maximum Levenshtein distance for fuzzy term matching. Two edits is
/// the family-wide setting, chosen so a transposed or doubled letter
/// still matches while unrelated short names do not.
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

/// High-level search facade over a [`PlanIndex`].
pub struct SearchEngine {
    /// The underlying Tantivy index wrapper.
    index: PlanIndex,
    /// The single long-lived Tantivy writer.
    ///
    /// Held for the process rather than created per call. Tantivy's
    /// `IndexWriter` allocates its whole `WRITER_HEAP_MB` arena and
    /// spawns merge threads on construction, so building one per
    /// indexed document put **~150 ms of pure setup on every create,
    /// update, merge, and delete** — measured in `benches/`, not
    /// guessed. It also took and released the index directory's
    /// exclusive writer lock each time, so two concurrent writes could
    /// collide on it; one owner cannot.
    writer: Mutex<IndexWriter>,
    /// Filesystem path the index lives at (diagnostics / reopen).
    pub index_path: String,
}

impl SearchEngine {
    /// The shared writer.
    ///
    /// A poisoned lock means an earlier write panicked. We recover the
    /// guard rather than failing for ever: the only operations held
    /// across it are `delete_term` / `add_document` / `commit`, and a
    /// permanently dead index would be a worse outcome than a retry.
    fn writer(&self) -> std::sync::MutexGuard<'_, IndexWriter> {
        self.writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
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
        let index = PlanIndex::create_or_open(p)?;
        // One writer for the process (see the field's docs).
        let writer = Mutex::new(index.writer(WRITER_HEAP_MB)?);
        Ok(Self {
            index,
            writer,
            index_path: p.to_string_lossy().into_owned(),
        })
    }

    /// Index (or re-index) one plan under its public id.
    ///
    /// Idempotent: the prior document for `pid` is deleted in the same
    /// writer batch, so an update replaces rather than duplicates. That
    /// matters because a duplicate document would let a *former* name go
    /// on matching for ever.
    ///
    /// # Errors
    ///
    /// When the writer cannot be acquired or the commit fails.
    pub fn index_plan(&self, pid: Uuid, plan: &Plan) -> Result<()> {
        let mut writer = self.writer();
        let s = self.index.schema();
        let pid_str = pid.to_string();
        // Replace-in-place: same batch, so a crash between the two
        // cannot leave the record indexed twice.
        writer.delete_term(Term::from_field_text(s.pid, &pid_str));
        writer
            .add_document(doc!(
                s.pid => pid_str,
                s.name => plan.name.clone(),
                s.alternate_names => plan.alternate_names.join(" "),
                s.name_phonetic => phonetic_codes(&name_text(plan)),
                s.identifiers => identifier_text(plan),
                s.keywords => plan.keywords.join(" "),
                s.tags => plan.tags.join(" "),
                s.goals => goal_text(plan),
                s.owner_org_name => plan.owner_org_name.clone().unwrap_or_default(),
                s.code => plan.code.clone().unwrap_or_default(),
                s.owner_org_id => plan.owner_org_id.clone().unwrap_or_default(),
                s.kind => plan
                    .kind
                    .map(|k| format!("{k:?}").to_lowercase())
                    .unwrap_or_default(),
                s.status => plan
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

    /// Remove a plan from the index by public id. Called on soft-delete
    /// (and for the duplicate side of a merge), because a soft-deleted
    /// row must stop being a search hit.
    ///
    /// # Errors
    ///
    /// When the delete commit fails.
    pub fn delete_plan(&self, pid: Uuid) -> Result<()> {
        let mut writer = self.writer();
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
        let mut writer = self.writer();
        writer
            .delete_all_documents()
            .map_err(|e| err(&format!("delete all: {e}")))?;
        writer
            .commit()
            .map_err(|e| err(&format!("commit clear: {e}")))?;
        self.index.reload()
    }

    /// **Blocking query** for duplicate detection: the candidate set an
    /// incoming plan should be scored against.
    ///
    /// Union of three retrieval routes, because a duplicate can present
    /// as any one of them:
    ///
    /// 1. fuzzy name / alternate-name tokens (a re-named or abbreviated
    ///    plan),
    /// 2. exact identifier values (Jira key / Asana GID / URI / UUID —
    ///    the matcher's deterministic short-circuits; a record filed
    ///    under a completely different name is *only* reachable this
    ///    way),
    /// 3. phonetic codes of the name (a name transcribed by ear).
    ///
    /// Deliberately **not** filtered by `kind`: the matcher is
    /// kind-agnostic (see the module doc and `AGENTS.md` golden rule 5)
    /// — a `Portfolio`-labelled plan and a `Program`-labelled plan may
    /// still be the same identity, so gating candidates by kind would
    /// silently reintroduce a boundary the data model does not have.
    ///
    /// # Errors
    ///
    /// When the search fails.
    pub fn candidates(&self, plan: &Plan, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let mut sub: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        let name_tokens = tokenise(&name_text(plan));
        for t in &name_tokens {
            for f in [s.name, s.alternate_names] {
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
        for t in tokenise(&identifier_text(plan)) {
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
    /// retrieval routes the un-paged methods expose; `kind_filter`, when
    /// set, additionally requires the exact (lowercased) `kind` label —
    /// an opt-in narrowing the caller requests, not a gate the service
    /// imposes (see the module doc).
    ///
    /// # Errors
    ///
    /// When the search fails.
    pub fn search_page(
        &self,
        query_str: &str,
        mode: SearchMode,
        kind_filter: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<String>, usize)> {
        let searcher = self.index.reader().searcher();
        let Some(query) = self.build_query(query_str, mode, kind_filter) else {
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
                    tantivy::collector::TopDocs::with_limit(wanted.max(1)).order_by_score(),
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

    /// Build the query for one retrieval mode, optionally `Must`-combined
    /// with an exact `kind` filter, or `None` when the input tokenises to
    /// nothing (an empty query matches nothing rather than everything).
    fn build_query(
        &self,
        query_str: &str,
        mode: SearchMode,
        kind_filter: Option<&str>,
    ) -> Option<Box<dyn Query>> {
        let s = self.index.schema();
        let base: Box<dyn Query> = match mode {
            SearchMode::Exact => {
                let fields = vec![
                    s.name,
                    s.alternate_names,
                    s.owner_org_name,
                    s.identifiers,
                    s.keywords,
                    s.tags,
                    s.goals,
                ];
                let parser = QueryParser::for_index(self.index.index(), fields.clone());
                if let Ok(query) = parser.parse_query(query_str) {
                    query
                } else {
                    // A query the parser rejects (unbalanced quotes, a
                    // bare operator) becomes an OR over its tokens rather
                    // than an error — a stray quote in a search box must
                    // not be a 500.
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
                    Box::new(BooleanQuery::new(sub))
                }
            }
            SearchMode::Fuzzy => {
                let tokens = tokenise(query_str);
                if tokens.is_empty() {
                    return None;
                }
                let fields = [s.name, s.alternate_names, s.owner_org_name, s.keywords];
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
                Box::new(BooleanQuery::new(sub))
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
                Box::new(BooleanQuery::new(sub))
            }
        };
        match kind_filter {
            None => Some(base),
            Some(kind) => {
                let kind_term: Box<dyn Query> = Box::new(TermQuery::new(
                    Term::from_field_text(s.kind, &kind.to_lowercase()),
                    IndexRecordOption::Basic,
                ));
                Some(Box::new(BooleanQuery::new(vec![
                    (Occur::Must, base),
                    (Occur::Must, kind_term),
                ])))
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
            .search(query, &TopDocs::with_limit(limit).order_by_score())
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

/// Every name-ish string on the plan, space-joined — the text the
/// phonetic field and the blocking query are built from.
fn name_text(plan: &Plan) -> String {
    let mut parts = vec![plan.name.clone()];
    parts.extend(plan.alternate_names.iter().cloned());
    parts.join(" ")
}

/// Identifier values, space-joined. Schemes are not indexed: a caller
/// searching for a Jira key should not have to know it is a Jira key.
fn identifier_text(plan: &Plan) -> String {
    plan.identifiers
        .iter()
        .map(|i| i.value.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Goal titles, space-joined — the defining attribute of a plan: what it
/// is trying to achieve.
fn goal_text(plan: &Plan) -> String {
    plan.goals
        .iter()
        .map(|g| g.title.as_str())
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
/// Punctuation and underscores separate, so `"Apollo, Inc."` yields
/// `["apollo", "inc"]`.
fn tokenise(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_portfolio_management_matcher::{Goal, IdentifierScheme, PlanIdentifier, PlanKind};
    use tempfile::TempDir;

    /// An engine over a throwaway directory, plus the directory guard.
    fn engine_in_temp() -> (SearchEngine, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(dir.path()).unwrap();
        (engine, dir)
    }

    /// Index one plan and return the pid it was filed under.
    fn index(engine: &SearchEngine, plan: &Plan) -> Uuid {
        let pid = Uuid::new_v4();
        engine.index_plan(pid, plan).unwrap();
        pid
    }

    /// A fully-populated plan.
    fn apollo_plan() -> Plan {
        Plan {
            kind: Some(PlanKind::Project),
            alternate_names: vec!["Project Apollo".into()],
            owner_org_name: Some("Acme Astronautics".into()),
            owner_org_id: Some("organization:9a2f".into()),
            code: Some("PROJ-2026".into()),
            goals: vec![Goal {
                title: "Cut launch latency".into(),
                ..Default::default()
            }],
            keywords: vec!["migration".into()],
            identifiers: vec![PlanIdentifier {
                scheme: IdentifierScheme::JiraProjectKey,
                value: "APOLLO".into(),
            }],
            ..Plan::new("Apollo platform migration")
        }
    }

    /// An indexed plan is found by an exact full-text query.
    #[test]
    fn index_and_exact_search() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &apollo_plan());
        let (hits, total) = engine
            .search_page("Apollo", SearchMode::Exact, None, 10, 0)
            .unwrap();
        assert_eq!(hits, vec![pid.to_string()]);
        assert_eq!(total, 1);
    }

    /// Fuzzy search survives a typo; exact search does not.
    #[test]
    fn fuzzy_search_tolerates_typo() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Plan::new("Delivery programme"));
        assert!(
            engine
                .search_page("Delivry", SearchMode::Exact, None, 10, 0)
                .unwrap()
                .0
                .is_empty()
        );
        assert_eq!(
            engine
                .search_page("Delivry", SearchMode::Fuzzy, None, 10, 0)
                .unwrap()
                .0,
            vec![pid.to_string()]
        );
    }

    /// The secondary fields are searchable, not just the name — the goal
    /// title especially, since it is what a plan is *trying to achieve*.
    #[test]
    fn secondary_fields_are_searchable() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &apollo_plan()).to_string();
        for q in [
            "Project Apollo",
            "Acme Astronautics",
            "Cut launch latency",
            "migration",
            "APOLLO",
        ] {
            let (hits, _) = engine
                .search_page(q, SearchMode::Exact, None, 10, 0)
                .unwrap();
            assert_eq!(hits, vec![pid.clone()], "query {q}");
        }
    }

    /// `?kind=` narrows a search to one kind label — an opt-in filter,
    /// not the dedup gate the matcher deliberately lacks.
    #[test]
    fn kind_filter_narrows_search_but_is_optional() {
        let (engine, _dir) = engine_in_temp();
        let project_pid = index(&engine, &apollo_plan());
        let mut program = Plan::new("Apollo platform migration");
        program.kind = Some(PlanKind::Program);
        index(&engine, &program);

        // Unfiltered: both hit.
        let (hits, total) = engine
            .search_page("Apollo", SearchMode::Exact, None, 10, 0)
            .unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(total, 2);

        // Filtered to "project": only the project-kind plan.
        let (hits, total) = engine
            .search_page("Apollo", SearchMode::Exact, Some("project"), 10, 0)
            .unwrap();
        assert_eq!(hits, vec![project_pid.to_string()]);
        assert_eq!(total, 1);
    }

    /// The blocking query does **not** filter by kind — a project-kind
    /// and a program-kind plan with the same identifying name/identifier
    /// still block against each other, matching the matcher's
    /// kind-agnostic dedup rule.
    #[test]
    fn candidates_ignore_kind() {
        let (engine, _dir) = engine_in_temp();
        let mut stored = apollo_plan();
        stored.kind = Some(PlanKind::Program);
        let pid = index(&engine, &stored);

        let mut query = apollo_plan();
        query.kind = Some(PlanKind::Project);
        assert_eq!(
            engine.candidates(&query, 10).unwrap(),
            vec![pid.to_string()],
            "candidates must reach a differently-kinded plan"
        );
    }

    /// Re-indexing the same pid replaces the document.
    #[test]
    fn reindex_replaces_rather_than_duplicates() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Plan::new("Old Name"));
        engine.index_plan(pid, &Plan::new("New Name")).unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 1);
        let (hits, _) = engine
            .search_page("New", SearchMode::Exact, None, 10, 0)
            .unwrap();
        assert_eq!(hits, vec![pid.to_string()]);
        assert!(
            engine
                .search_page("Old", SearchMode::Exact, None, 10, 0)
                .unwrap()
                .0
                .is_empty()
        );
    }

    /// Deleting removes the document.
    #[test]
    fn delete_removes_from_index() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Plan::new("Ephemeral Plan"));
        engine.delete_plan(pid).unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 0);
    }

    /// `clear` empties the index (the rebuild task's first step).
    #[test]
    fn clear_empties_the_index() {
        let (engine, _dir) = engine_in_temp();
        index(&engine, &Plan::new("One"));
        index(&engine, &Plan::new("Two"));
        engine.clear().unwrap();
        assert_eq!(engine.stats().unwrap().num_docs, 0);
    }

    /// The blocking query reaches a plan whose name is entirely
    /// different but whose identifier matches — the deterministic
    /// short-circuit case a name-only block would silently lose.
    #[test]
    fn candidates_block_on_identifier_alone() {
        let (engine, _dir) = engine_in_temp();
        let ident = PlanIdentifier {
            scheme: IdentifierScheme::JiraProjectKey,
            value: "APOLLO".into(),
        };
        let stored = Plan {
            identifiers: vec![ident.clone()],
            ..Plan::new("Wholly Unrelated Plan")
        };
        let pid = index(&engine, &stored);
        let query = Plan {
            identifiers: vec![ident],
            ..Plan::new("Apollo platform migration")
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
        index(&engine, &apollo_plan());
        assert!(engine.candidates(&Plan::new("  "), 10).unwrap().is_empty());
    }

    /// A query Tantivy's parser rejects falls back to a token OR rather
    /// than erroring, so a stray quote in a search box is not a 500.
    #[test]
    fn unparseable_query_falls_back_to_tokens() {
        let (engine, _dir) = engine_in_temp();
        let pid = index(&engine, &Plan::new("Apollo platform migration"));
        let (hits, _) = engine
            .search_page("\"apollo", SearchMode::Exact, None, 10, 0)
            .unwrap();
        assert_eq!(hits, vec![pid.to_string()]);
    }

    /// A page reports the **whole** match count, not the page length —
    /// the number `X-Total-Count` carries.
    #[test]
    fn search_page_reports_the_total_not_the_page() {
        let (engine, _dir) = engine_in_temp();
        for i in 0..5 {
            index(&engine, &Plan::new(format!("Paging Plan {i}")));
        }
        let (hits, total) = engine
            .search_page("Paging", SearchMode::Exact, None, 2, 1)
            .unwrap();
        assert_eq!(hits.len(), 2, "the page is two hits");
        assert_eq!(total, 5, "the total ignores the window");
    }

    /// `tokenise` lowercases and splits on punctuation.
    #[test]
    fn tokenise_splits_on_punctuation() {
        assert_eq!(tokenise("Apollo, Inc."), vec!["apollo", "inc"]);
        assert_eq!(tokenise("   "), Vec::<String>::new());
    }
}
