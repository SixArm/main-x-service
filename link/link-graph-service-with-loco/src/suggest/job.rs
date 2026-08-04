//! The periodic cross-service `same_identity` suggestion job (spec T-31,
//! `13-tasks.md`; design decisions pinned at `16-open-questions.md` OQ-9,
//! specifically OQ-9(c)/(d)) — the I/O this feature has been building
//! toward across T-29 (the comparator) and T-30 (candidate blocking).
//!
//! Mirrors [`crate::reconcile`]'s shape with the verb flipped: instead of
//! pulling a service's authoritative edges and repairing this aggregator's
//! own read-model, this job pulls **person** and **worker** identity data,
//! scores every same-block pair through [`super::compare_identity`] /
//! [`super::generate_candidates`], and **POSTs** the survivors to person's
//! `POST /api/persons/{id}/links` — a write against a *peer's* API, never
//! against this aggregator's own storage. The aggregator stays read-only to
//! the world (OQ-9(c)): it never gains a link-write endpoint of its own;
//! calling person's write API makes this job an authenticated **client** of
//! a peer, the same shape [`crate::reconcile::HttpAuthoritativeSource`]
//! already uses in reverse (`GET` there, `POST` here).
//!
//! ## Configuration
//!
//! Per OQ-9(c), the write target and its credential get their **own** env
//! vars rather than reusing the reconcile ones (different blast radius —
//! reconcile only reads a peer's outbound edges, this job writes one):
//!
//! - `LINK_GRAPH_SUGGEST_URL_PERSON` — person's **collection base** URL,
//!   e.g. `http://127.0.0.1:5150/api/persons`. Doubles as both the fetch
//!   source (`GET {url}?limit=&offset=`) and the write target
//!   (`{url}/{id}/links`), since person is this job's sole write target
//!   (workers are read-only input — OQ-9's "who writes the edge" already
//!   settled this on the originating-service side, and the originating
//!   side of `same_identity` here is always person).
//! - `LINK_GRAPH_SUGGEST_URL_WORKER` — worker's collection base URL, e.g.
//!   `http://127.0.0.1:5160/api/workers`, used **only** to fetch (never to
//!   write). **Not** pinned by OQ-9(c) verbatim (which names only the
//!   `_PERSON` write target) but required to make the job function at
//!   all — there is exactly one entity to write to and two to read from.
//!   Named to match the established per-entity `LINK_GRAPH_RECONCILE_URL_<ENTITY>`
//!   convention rather than inventing a new shape.
//! - `LINK_GRAPH_SUGGEST_TOKEN` — the dedicated bearer (not
//!   `LINK_GRAPH_RECONCILE_TOKEN`), sent on every outbound call this job
//!   makes (fetch **and** POST) — SEC-B7: a loopback URL may go
//!   token-less, any remote host refuses to start without one
//!   ([`crate::reconcile::source_auth_ok`], reused rather than
//!   reimplemented).
//! - `LINK_GRAPH_SUGGEST_SECS` (default 3600) — the run interval, coarser
//!   than reconcile's 300s default because this job does real `O(pairs)`
//!   scoring work rather than a cheap diff (OQ-9(d)). Same skip-first-tick
//!   pattern as [`crate::reconcile::run_periodic`].
//!
//! ## Why `GET {base_url}?limit=&offset=`, not `search?q=*`
//!
//! The first landed version of this job paged `GET
//! /<plural>/search?q=*&limit=&offset=`, reasoning that the query
//! grammar's dedicated `*` token parses to `UserInputLeaf::All` /
//! `AllQuery`, matching every indexed document — true in isolation
//! (confirmed against the exact pinned `tantivy-query-grammar` 0.22.0
//! source), but **wrong as a foundation for enumeration**: a live
//! investigation (bring up a real person-service, create a real record,
//! query it) found `q=*` could come back **empty** against a real
//! running instance, because the Tantivy index is a separate artefact
//! from the database and can legitimately drift from it — a dev index
//! directory that outlives a database reset, entries surviving a delete
//! the index was never told about, a partial reindex. The search-hit ids
//! that no longer resolve to a database row are silently dropped by the
//! found-in-index-but-not-in-database guard, so a small-`limit` `q=*`
//! page can land entirely on stale entries and return nothing even
//! though matching rows exist.
//!
//! person's and worker's REST APIs now each carry a genuine,
//! database-backed `GET /<plural>?limit=&offset=` collection-list
//! endpoint (`list_persons` / `list_workers`), added specifically to
//! give this job — and any future caller with the same "enumerate
//! everything" need — an answer that is only ever as stale as the
//! database itself, never as stale as a second, independently
//! lifecycled copy of the data. See `person-service`'s `CHANGELOG.md`
//! for the full investigation and root cause. This job now pages that
//! endpoint directly; the Tantivy search index is not consulted at all.
//!
//! ## Scale controls (T-33, OQ-9(d))
//!
//! Two configured caps, on top of this job's own non-configurable
//! [`MAX_FETCH_OFFSET`] (both person's and worker's list endpoints now
//! bound `offset` themselves — `MAX_SEARCH_OFFSET` / `MAX_LIST_OFFSET`,
//! both `10_000` — but this job does not rely on a peer enforcing that;
//! it stops itself either way):
//!
//! - **`LINK_GRAPH_SUGGEST_MAX_CANDIDATES`** (default `50`, same shape as
//!   `BatchDeduplicationRequest::max_candidates`) — read by
//!   [`max_candidates_from_env`], passed straight through to
//!   [`super::generate_candidates_bounded`] (see that module's "Scale
//!   control 1" doc section for the design). Bounds same-block
//!   comparisons per person anchor, so one pathological block cannot make
//!   a single pass do unbounded work.
//! - **`LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN`** (default `200`) — read by
//!   [`max_edges_per_run_from_env`]. When [`super::generate_candidates_bounded`]
//!   returns more candidates than this, [`run_suggestion_pass`] `POST`s
//!   only the **highest-confidence** `max_edges_per_run` of them (sorted
//!   descending on [`super::IdentityMatchScore::confidence`], ties broken
//!   on the `(person, worker)` id pair for full determinism) — an
//!   identifier-ceiling match is stronger evidence than a borderline
//!   probabilistic one, so if a run must be cut short the strongest
//!   suggestions survive it. The rest are simply not `POSTed` this pass;
//!   nothing is lost, because the next pass re-fetches and re-scores the
//!   same data (the fetch is idempotent) and will find them again. The
//!   number dropped is both logged (`tracing::warn!`, so an operator
//!   watching logs sees the cap binding) and included in
//!   [`SuggestionRunStats::dropped`], which is durably recorded per pass
//!   (see "Durable per-run audit" below) — a live gauge or a log line
//!   alone would not survive past the next scrape/restart, and an
//!   operator asking "was the cap binding last week" needs an answer
//!   that does.
//!
//! ## Audit — every POST, and every run's counts (T-33)
//!
//! **Every POST.** This job's only write is the `POST
//! {person}/{id}/links` [`HttpSuggestionSink::post_suggestion`] makes,
//! and that request lands on person's own `create_link` handler
//! (`person-service-with-loco/src/api/rest/links.rs`) — the exact same
//! handler an operator's own link write goes through. `create_link`
//! **unconditionally** writes a best-effort `person_link` audit row
//! (`state.audit_log.log_create("person_link", link.id, new_values,
//! &audit_ctx(&caller))`) for every successful link creation, regardless
//! of `provenance` — a `matcher_suggested` suggestion from this job is
//! not special-cased out of that write. So "the suggestion job audits
//! every POST it makes" is already true of the *existing* T-31/T-32
//! infrastructure on person's side; this job needs no audit trail of its
//! own for the POST itself (that would be a second, redundant audit of
//! the same event from the wrong side of the wire — this job is not the
//! service of record for the edge it just asked person to create). See
//! `person-service-with-loco/tests/cross_service_link_review.rs`'s
//! `matcher_suggested_link_creation_is_audited` test (T-33) for the
//! regression pin proving this rather than merely asserting it in a
//! comment.
//!
//! **Every run's counts.** [`crate::reconcile`]'s periodic pass — the
//! closest sibling to this job — records its one summary number
//! (`link_graph_reconciliation_divergence`) only as a live Prometheus
//! gauge plus a `tracing::info!` line; that is sufficient there because
//! "did the last pass find drift" is answered by the gauge's *current*
//! value. This job's summary is richer (fetch counts on two services,
//! a candidate count, a post/fail/drop split) and OQ-9(d) specifically
//! asks for it to be *auditable after the fact*, which a gauge cannot
//! give — a gauge holds only the latest value and is lost across a
//! process restart or a missed scrape. So [`run_periodic`] does both:
//! it mirrors reconcile's gauge-plus-log pattern (the
//! `link_graph_suggestion_last_run` gauge vec,
//! [`crate::metrics::Metrics`]) for live/alertable visibility, **and**
//! durably records one [`crate::models::suggestion_runs`] row per
//! completed pass (a new small table — reconcile has no equivalent
//! because its own state, the read-model itself, already **is** durable
//! and queryable; this job writes nothing durable of its own to double
//! as that record). A pass that fails at the fetch step (the only error
//! [`run_suggestion_pass`] can return) records nothing, matching
//! `run_periodic`'s existing log-and-retry posture for that case.

use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use entity_ref::{EntityRef, EntityType};
use loco_rs::prelude::*;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use super::{
    DEFAULT_MAX_CANDIDATES, IdentityCandidate, IdentityProbe, ProbeIdentifier, ProbeName,
    generate_candidates_bounded,
};
use crate::models::suggestion_runs::SuggestionRunRecord;

/// A source of one entity's identity data, for the suggestion pass.
/// Injectable so the fetch→block→compare→post pipeline is testable without
/// a live HTTP call, mirroring [`crate::reconcile::AuthoritativeSource`].
#[async_trait]
pub trait IdentitySource: Send + Sync {
    /// A short label for logging (e.g. `"person"`, `"worker"`).
    fn label(&self) -> &'static str;

    /// Fetch every active record as an `(EntityRef, IdentityProbe)` pair.
    ///
    /// # Errors
    ///
    /// When the underlying fetch fails.
    async fn fetch_all(&self) -> ModelResult<Vec<(EntityRef, IdentityProbe)>>;
}

/// Where a scored [`IdentityCandidate`] is `POSTed`. Injectable for the same
/// reason as [`IdentitySource`] — the pipeline is tested against a mock
/// sink, never a live person-service.
#[async_trait]
pub trait SuggestionSink: Send + Sync {
    /// POST one candidate as a `matcher_suggested` `same_identity` edge.
    ///
    /// # Errors
    ///
    /// When the underlying POST fails (network, non-2xx, …).
    async fn post_suggestion(&self, candidate: &IdentityCandidate) -> ModelResult<()>;
}

/// The page size requested per fetch call — the same `100` both person's
/// and worker's `list`/`search` handlers clamp `limit` to, so a page
/// never comes back short of what was actually available.
const PAGE_LIMIT: usize = 100;

/// A defensive cap on the fetch pagination loop's offset, mirroring
/// person's own SEC-G7 `MAX_SEARCH_OFFSET` (`10_000`, also the bound
/// person's and worker's `GET /<plural>` list endpoints themselves
/// enforce). This job applies the same number independent of whatever a
/// peer enforces, so it never loops without limit even against a peer
/// that changes its own bound later.
const MAX_FETCH_OFFSET: usize = 10_000;

/// The `{"success":…, "data": T}` envelope every native REST response on
/// this family wraps its body in.
#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    #[serde(default)]
    data: Option<T>,
}

/// The `data` payload of `GET /<plural>`: person's field is `persons`,
/// worker's is `workers` — both alias onto `records` here so one type
/// parses both services' response shape.
#[derive(Debug, Default, Deserialize)]
struct ListData {
    #[serde(alias = "persons", alias = "workers")]
    records: Vec<WireRecord>,
}

/// The subset of a `Person`/`Worker` wire record this job needs. Both
/// services' domain models carry the same shape for these fields (name,
/// birth date, gender, identifiers), so one struct parses either.
/// Unrecognised fields (everything else a full record carries) are
/// ignored by `serde`'s default struct behaviour.
#[derive(Debug, Deserialize)]
struct WireRecord {
    id: Uuid,
    #[serde(default)]
    name: WireName,
    #[serde(default)]
    birth_date: Option<NaiveDate>,
    #[serde(default)]
    gender: Option<String>,
    #[serde(default)]
    identifiers: Vec<WireIdentifier>,
}

/// The wire shape of `HumanName` — only `family`/`given` matter here.
#[derive(Debug, Default, Deserialize)]
struct WireName {
    #[serde(default)]
    family: String,
    #[serde(default)]
    given: Vec<String>,
}

/// The wire shape of `Identifier` — only `system`/`identifier_type`/`value`
/// matter here.
#[derive(Debug, Deserialize)]
struct WireIdentifier {
    #[serde(default)]
    identifier_type: String,
    #[serde(default)]
    system: String,
    #[serde(default)]
    value: String,
}

/// Map a wire gender token (`"male"`/`"female"`/`"other"`/`"unknown"`, the
/// lowercase form both person's and worker's `Gender` serialize to) to the
/// matcher's [`person_matcher::Gender`]. An unrecognised token maps to
/// `None` (excluded, not guessed) rather than defaulting to `Unknown` —
/// only a wire value that actually says "unknown" should score as the
/// comparator's soft-ambiguity case.
fn wire_gender_to_matcher(token: &str) -> Option<person_matcher::Gender> {
    match token.trim().to_ascii_lowercase().as_str() {
        "male" => Some(person_matcher::Gender::Male),
        "female" => Some(person_matcher::Gender::Female),
        "other" => Some(person_matcher::Gender::Other),
        "unknown" => Some(person_matcher::Gender::Unknown),
        _ => None,
    }
}

/// Map a wire identifier to a [`ProbeIdentifier`], choosing the more
/// stable, cross-service-comparable field as the block/match `scheme`: the
/// FHIR `system` namespace URI when present (both services use the same
/// well-known URIs for the same real-world scheme, e.g. an NHS number or a
/// US SSN — see `person`'s `matching::adapter::route_identifier`, which
/// keys the same way), falling back to the coarser `identifier_type`
/// token (`"MRN"`, `"SSN"`, …) only when `system` is blank. `None` when the
/// identifier construction rejects a blank scheme/value (never treats
/// blank as "no identifier" that could spuriously match another blank).
fn wire_identifier_to_probe(id: &WireIdentifier) -> Option<ProbeIdentifier> {
    let scheme = if id.system.trim().is_empty() {
        id.identifier_type.as_str()
    } else {
        id.system.as_str()
    };
    ProbeIdentifier::new(scheme, &id.value)
}

/// Map a fetched [`WireRecord`] to an [`IdentityProbe`]. Pure and
/// dependency-free of any live service — unit-tested directly against
/// fixture JSON below, independent of the HTTP fetch that produces the
/// bytes in production.
fn probe_from_wire(record: &WireRecord) -> IdentityProbe {
    IdentityProbe {
        // `given` is space-joined, mirroring how both services' own search
        // indexers flatten multiple given names into one tokenizable
        // string (`person.name.given.join(" ")` in `search/mod.rs`) — the
        // same convention, not a fresh one.
        name: Some(ProbeName {
            family: record.name.family.clone(),
            given: record.name.given.join(" "),
        }),
        birth_date: record.birth_date,
        gender: record.gender.as_deref().and_then(wire_gender_to_matcher),
        identifiers: record
            .identifiers
            .iter()
            .filter_map(wire_identifier_to_probe)
            .collect(),
    }
}

/// The **real** identity source: pages `GET {base_url}?limit=&offset=`,
/// person's and worker's database-backed collection-list endpoint (see the
/// module doc's "Why `GET {base_url}?limit=&offset=`, not `search?q=*`"
/// section), mapping each hit to an `(EntityRef, IdentityProbe)` via
/// [`probe_from_wire`].
pub struct HttpIdentitySource {
    entity_type: EntityType,
    base_url: String,
    token: Option<String>,
}

impl HttpIdentitySource {
    /// Build a source for `entity_type` against `base_url` (a service's
    /// collection base, e.g. `http://host/api/persons`), optionally
    /// bearer-authenticated.
    #[must_use]
    pub fn new(entity_type: EntityType, base_url: String, token: Option<String>) -> Self {
        Self {
            entity_type,
            base_url,
            token,
        }
    }
}

#[async_trait]
impl IdentitySource for HttpIdentitySource {
    fn label(&self) -> &'static str {
        self.entity_type.as_str()
    }

    async fn fetch_all(&self) -> ModelResult<Vec<(EntityRef, IdentityProbe)>> {
        let client = reqwest::Client::new();
        let mut out = Vec::new();
        let mut offset = 0usize;
        loop {
            let mut request = client.get(&self.base_url).query(&[
                ("limit", &PAGE_LIMIT.to_string()),
                ("offset", &offset.to_string()),
            ]);
            if let Some(token) = &self.token {
                request = request.bearer_auth(token);
            }
            let response = request
                .send()
                .await
                .map_err(|e| ModelError::Any(Box::new(e)))?
                .error_for_status()
                .map_err(|e| ModelError::Any(Box::new(e)))?;
            let body: ApiEnvelope<ListData> = response
                .json()
                .await
                .map_err(|e| ModelError::Any(Box::new(e)))?;
            let records = body.data.map(|d| d.records).unwrap_or_default();
            let page_len = records.len();
            for record in &records {
                out.push((
                    EntityRef::new(self.entity_type, record.id),
                    probe_from_wire(record),
                ));
            }
            if page_len < PAGE_LIMIT {
                break;
            }
            offset += PAGE_LIMIT;
            if offset > MAX_FETCH_OFFSET {
                tracing::warn!(
                    entity = self.label(),
                    offset,
                    "suggest fetch: stopped at the defensive offset cap; \
                     some records beyond it were not fetched this run"
                );
                break;
            }
        }
        Ok(out)
    }
}

/// The **real** suggestion sink: `POST {base_url}/{person_id}/links` with a
/// `same_identity`/`matcher_suggested` body, matching person's
/// `LinkRequest` shape exactly
/// (`kind`/`to_ref`/`confidence`/`provenance`/`score_breakdown`). The
/// `score_breakdown` field (T-32,
/// `link-graph-service-with-loco/spec/16-open-questions.md` OQ-9(b)) is
/// this comparator's own [`super::IdentityMatchScore`] mapped verbatim to
/// JSON — person's `create_link` handler stores it straight into the
/// review-queue row's `score_breakdown` column, so an operator reviewing
/// the suggestion sees exactly which components (identifier / name / DOB
/// / gender) drove the score, not just the final number.
pub struct HttpSuggestionSink {
    /// Person's collection base URL (e.g. `http://host/api/persons`).
    base_url: String,
    token: Option<String>,
}

impl HttpSuggestionSink {
    /// Build a sink `POSTing` against `base_url`, optionally
    /// bearer-authenticated.
    #[must_use]
    pub fn new(base_url: String, token: Option<String>) -> Self {
        Self { base_url, token }
    }
}

#[async_trait]
impl SuggestionSink for HttpSuggestionSink {
    async fn post_suggestion(&self, candidate: &IdentityCandidate) -> ModelResult<()> {
        let url = format!("{}/{}/links", self.base_url, candidate.person.id);
        let body = serde_json::json!({
            "kind": "same_identity",
            "to_ref": candidate.worker.to_string(),
            "confidence": candidate.score.confidence,
            "provenance": "matcher_suggested",
            "score_breakdown": {
                "identifier_match": candidate.score.identifier_match,
                "name_score": candidate.score.name_score,
                "dob_score": candidate.score.dob_score,
                "gender_score": candidate.score.gender_score,
            },
        });
        let mut request = reqwest::Client::new().post(&url).json(&body);
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        request
            .send()
            .await
            .map_err(|e| ModelError::Any(Box::new(e)))?
            .error_for_status()
            .map_err(|e| ModelError::Any(Box::new(e)))?;
        Ok(())
    }
}

/// Counts from one suggestion pass. Logged at the end of
/// [`run_suggestion_pass`] and — since T-33 — durably recorded to
/// [`crate::models::suggestion_runs`] by [`run_periodic`] (see the module
/// doc's "Audit — every POST, and every run's counts" section).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SuggestionRunStats {
    /// Persons fetched this pass.
    pub persons_fetched: usize,
    /// Workers fetched this pass.
    pub workers_fetched: usize,
    /// Candidates `generate_candidates_bounded` returned (at/above the
    /// threshold, within the configured `max_candidates` per-anchor cap).
    pub candidates: usize,
    /// Candidates successfully `POSTed`.
    pub posted: usize,
    /// Candidates whose POST failed (logged individually, run continues).
    pub failed: usize,
    /// Candidates that qualified but were never `POSTed` this pass because
    /// [`SuggestionRunStats::candidates`] exceeded the configured
    /// `max_edges_per_run` cap (T-33, OQ-9(d)) — the lowest-confidence
    /// ones by construction (see [`run_suggestion_pass`]'s sort). Not
    /// lost: the next pass re-fetches and re-scores the same data and
    /// will find them again, since the fetch is idempotent.
    pub dropped: usize,
}

/// Run one suggestion pass: fetch both sides, block + score
/// ([`super::generate_candidates_bounded`], bounded by `max_candidates`
/// same-block comparisons per person anchor — T-33, OQ-9(d)), then `POST`
/// the highest-confidence `max_edges_per_run` surviving candidates (see
/// the module doc's "Scale controls" section for why highest-confidence
/// and how ties break). A single candidate's POST failure is logged and
/// does not abort the rest of the run — matching the family's
/// per-row-error-tolerant posture elsewhere
/// (`agents/share/bulk-import-export.md` §7: one bad row never aborts the
/// whole pass).
///
/// # Errors
///
/// Only when a **fetch** fails (either side) — a partial identity feed is
/// not a safe basis for suggesting cross-service identity, unlike an
/// individual POST failure, which is logged and counted instead.
pub async fn run_suggestion_pass<P, W, S>(
    persons: &P,
    workers: &W,
    sink: &S,
    max_candidates: usize,
    max_edges_per_run: usize,
) -> ModelResult<SuggestionRunStats>
where
    P: IdentitySource + ?Sized,
    W: IdentitySource + ?Sized,
    S: SuggestionSink + ?Sized,
{
    let person_probes = persons.fetch_all().await?;
    let worker_probes = workers.fetch_all().await?;
    let mut candidates =
        generate_candidates_bounded(&person_probes, &worker_probes, max_candidates);
    let total_candidates = candidates.len();

    // T-33 (OQ-9(d)): prioritise the strongest evidence when a run must be
    // cut short. Descending confidence, ties broken on the (person,
    // worker) id pair so the ordering — and therefore exactly which
    // candidates get dropped — is fully deterministic, never dependent on
    // `HashMap` iteration order or float-comparison happenstance.
    candidates.sort_by(|a, b| {
        b.score
            .confidence
            .partial_cmp(&a.score.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.person.id.cmp(&b.person.id))
            .then_with(|| a.worker.id.cmp(&b.worker.id))
    });
    let dropped = total_candidates.saturating_sub(max_edges_per_run);
    if dropped > 0 {
        tracing::warn!(
            dropped,
            max_edges_per_run,
            total_candidates,
            "suggest: LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN cap is binding this run; \
             the lowest-confidence candidates were not posted (idempotent fetch means \
             they may be found again next run)"
        );
    }
    candidates.truncate(max_edges_per_run);

    let mut stats = SuggestionRunStats {
        persons_fetched: person_probes.len(),
        workers_fetched: worker_probes.len(),
        candidates: total_candidates,
        posted: 0,
        failed: 0,
        dropped,
    };
    for candidate in &candidates {
        match sink.post_suggestion(candidate).await {
            Ok(()) => stats.posted += 1,
            Err(error) => {
                stats.failed += 1;
                tracing::warn!(
                    %error,
                    person = %candidate.person,
                    worker = %candidate.worker,
                    "suggest: failed to POST a candidate; continuing with the rest of the run"
                );
            }
        }
    }
    Ok(stats)
}

/// Resolve the configured `(person source, worker source, sink)` triple
/// from already-read env values — pure (no env access), so the SEC-B7
/// accept/reject matrix is unit-testable without mutating process env vars
/// (which races under parallel `cargo test`). [`sources_from_env`] is the
/// thin env-reading shim around this.
fn resolve_sources(
    person_url: Option<String>,
    worker_url: Option<String>,
    token: Option<String>,
) -> Option<(HttpIdentitySource, HttpIdentitySource, HttpSuggestionSink)> {
    let person_url = person_url.filter(|s| !s.trim().is_empty())?;
    let Some(worker_url) = worker_url.filter(|s| !s.trim().is_empty()) else {
        tracing::warn!(
            "LINK_GRAPH_SUGGEST_URL_PERSON is set but LINK_GRAPH_SUGGEST_URL_WORKER is not; \
             the suggestion job needs both to produce candidates, so it will not start"
        );
        return None;
    };
    if !crate::reconcile::source_auth_ok(&person_url, token.is_some()) {
        tracing::warn!(
            url = %person_url,
            "refusing to start the suggestion job: LINK_GRAPH_SUGGEST_URL_PERSON is remote but \
             no LINK_GRAPH_SUGGEST_TOKEN is set (only a loopback URL may be token-less)"
        );
        return None;
    }
    if !crate::reconcile::source_auth_ok(&worker_url, token.is_some()) {
        tracing::warn!(
            url = %worker_url,
            "refusing to start the suggestion job: LINK_GRAPH_SUGGEST_URL_WORKER is remote but \
             no LINK_GRAPH_SUGGEST_TOKEN is set (only a loopback URL may be token-less)"
        );
        return None;
    }
    Some((
        HttpIdentitySource::new(EntityType::Person, person_url.clone(), token.clone()),
        HttpIdentitySource::new(EntityType::Worker, worker_url, token.clone()),
        HttpSuggestionSink::new(person_url, token),
    ))
}

/// Build the configured sources/sink from the environment
/// (`LINK_GRAPH_SUGGEST_URL_PERSON` / `_WORKER` / `LINK_GRAPH_SUGGEST_TOKEN`),
/// or `None` when the job is not configured to run (`LINK_GRAPH_SUGGEST_URL_PERSON`
/// unset) or refuses to (SEC-B7, [`resolve_sources`]).
#[must_use]
pub fn sources_from_env() -> Option<(HttpIdentitySource, HttpIdentitySource, HttpSuggestionSink)> {
    let person_url = std::env::var("LINK_GRAPH_SUGGEST_URL_PERSON").ok();
    let worker_url = std::env::var("LINK_GRAPH_SUGGEST_URL_WORKER").ok();
    let token = std::env::var("LINK_GRAPH_SUGGEST_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty());
    resolve_sources(person_url, worker_url, token)
}

/// Default for `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN` (T-33, OQ-9(d)):
/// the bulk subsystem's own per-run row caps
/// (`agents/share/bulk-import-export.md` §12, SEC-B2) are the family
/// precedent for "how many rows can one background pass commit", not a
/// number invented fresh for this job.
pub const DEFAULT_MAX_EDGES_PER_RUN: usize = 200;

/// Pure parse of `LINK_GRAPH_SUGGEST_MAX_CANDIDATES`'s raw value (T-33,
/// OQ-9(d)): the per-anchor same-block comparison cap passed to
/// [`super::generate_candidates_bounded`]. Absent, blank, zero, or
/// unparseable all fall back to [`DEFAULT_MAX_CANDIDATES`] — the same
/// "zero/unparseable falls back to the default" rule
/// `agents/share/restful.md` pins for pagination `limit`, so a
/// misconfigured `0` cannot silently turn the job into a no-op that never
/// compares anything. Pure (no env access) so it is unit-testable without
/// mutating process env vars, mirroring [`resolve_sources`]'s own reason
/// for splitting the pure logic from its env-reading shim.
#[must_use]
fn resolve_max_candidates(raw: Option<&str>) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_MAX_CANDIDATES)
}

/// Pure parse of `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN`'s raw value (T-33,
/// OQ-9(d)): the cap on how many suggestions [`run_suggestion_pass`]
/// `POST`s in one pass. Same zero/unparseable-falls-back-to-default rule
/// as [`resolve_max_candidates`].
#[must_use]
fn resolve_max_edges_per_run(raw: Option<&str>) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_MAX_EDGES_PER_RUN)
}

/// Read `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` from the environment and
/// resolve it via [`resolve_max_candidates`].
#[must_use]
fn max_candidates_from_env() -> usize {
    resolve_max_candidates(
        std::env::var("LINK_GRAPH_SUGGEST_MAX_CANDIDATES")
            .ok()
            .as_deref(),
    )
}

/// Read `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN` from the environment and
/// resolve it via [`resolve_max_edges_per_run`].
#[must_use]
fn max_edges_per_run_from_env() -> usize {
    resolve_max_edges_per_run(
        std::env::var("LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN")
            .ok()
            .as_deref(),
    )
}

/// Run the suggestion pass periodically until the process exits — the
/// "worker" wiring (OQ-9(c)/(d)), mirroring
/// [`crate::reconcile::run_periodic`]'s shape exactly: `LINK_GRAPH_SUGGEST_SECS`
/// (default 3600), first tick skipped so boot is not blocked, a failed
/// pass is logged and retried next tick. Spawned from `App::after_routes`
/// only when [`sources_from_env`] returns `Some`.
///
/// `db` is used only for T-33's durable per-run recording (see the module
/// doc's "Audit" section) — this job's actual work is HTTP client traffic
/// between two peers, not a read-model repair, so `db` is otherwise
/// untouched.
pub async fn run_periodic<P, W, S>(persons: P, workers: W, sink: S, db: DatabaseConnection)
where
    P: IdentitySource,
    W: IdentitySource,
    S: SuggestionSink,
{
    let secs = std::env::var("LINK_GRAPH_SUGGEST_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(3600);
    let max_candidates = max_candidates_from_env();
    let max_edges_per_run = max_edges_per_run_from_env();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(secs));
    interval.tick().await; // consume the immediate first tick
    loop {
        interval.tick().await;
        let started_at = Utc::now().fixed_offset();
        match run_suggestion_pass(&persons, &workers, &sink, max_candidates, max_edges_per_run)
            .await
        {
            Ok(stats) => {
                tracing::info!(?stats, "suggestion pass complete");
                crate::metrics::Metrics::global().set_suggestion_run_stats(&stats);
                let record = SuggestionRunRecord {
                    started_at,
                    persons_fetched: stats.persons_fetched,
                    workers_fetched: stats.workers_fetched,
                    candidates: stats.candidates,
                    posted: stats.posted,
                    failed: stats.failed,
                    dropped: stats.dropped,
                    max_candidates,
                    max_edges_per_run,
                };
                if let Err(error) =
                    crate::models::suggestion_runs::Model::record(&db, &record).await
                {
                    tracing::warn!(
                        %error,
                        "failed to durably record this suggestion run's stats \
                         (the pass itself still completed)"
                    );
                }
            }
            Err(error) => tracing::warn!(%error, "suggestion pass failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::suggest::IDENTIFIER_MATCH_CEILING;

    // ---------- probe_from_wire / gender / identifier mapping ----------

    /// A fixture matching the actual `Person`/`Worker` JSON shape
    /// deserializes into a `WireRecord`, and maps to the expected
    /// `IdentityProbe` — given names space-joined, gender mapped, and the
    /// identifier's `system` URI used as the block/match scheme.
    #[test]
    fn probe_from_wire_maps_the_real_wire_shape() {
        let json = serde_json::json!({
            "id": "0c4f1e2a-0000-4000-8000-000000000001",
            "name": { "family": "Smith", "given": ["Jane", "Marie"] },
            "birth_date": "1980-06-15",
            "gender": "female",
            "identifiers": [
                { "identifier_type": "MRN", "system": "https://fhir.nhs.uk/Id/nhs-number", "value": "943 476 5919" }
            ]
        });
        let record: WireRecord = serde_json::from_value(json).expect("valid wire record");
        let probe = probe_from_wire(&record);

        let name = probe.name.expect("name present");
        assert_eq!(name.family, "Smith");
        assert_eq!(name.given, "Jane Marie");
        assert_eq!(probe.birth_date, NaiveDate::from_ymd_opt(1980, 6, 15));
        assert_eq!(probe.gender, Some(person_matcher::Gender::Female));
        assert_eq!(probe.identifiers.len(), 1);
        // Confirm it round-trips through the SAME normalisation
        // `ProbeIdentifier::new` applies directly (scheme lower-cased,
        // whitespace stripped from value, upper-cased).
        assert_eq!(
            probe.identifiers[0],
            ProbeIdentifier::new("https://fhir.nhs.uk/Id/nhs-number", "9434765919").unwrap()
        );
    }

    /// A record with no `system` on its identifier falls back to
    /// `identifier_type` as the scheme.
    #[test]
    fn probe_from_wire_falls_back_to_identifier_type_when_system_is_blank() {
        let json = serde_json::json!({
            "id": "0c4f1e2a-0000-4000-8000-000000000002",
            "name": { "family": "Doe", "given": ["John"] },
            "identifiers": [
                { "identifier_type": "SSN", "system": "", "value": "078-05-1120" }
            ]
        });
        let record: WireRecord = serde_json::from_value(json).expect("valid wire record");
        let probe = probe_from_wire(&record);
        assert_eq!(
            probe.identifiers[0],
            ProbeIdentifier::new("SSN", "078-05-1120").unwrap()
        );
    }

    /// Every wire gender token round-trips to its matcher variant;
    /// `"unknown"` maps to the real `Unknown` variant (not `None`), and an
    /// unrecognised token maps to `None` (excluded, not guessed).
    #[test]
    fn wire_gender_tokens_map_correctly() {
        assert_eq!(
            wire_gender_to_matcher("male"),
            Some(person_matcher::Gender::Male)
        );
        assert_eq!(
            wire_gender_to_matcher("Female"),
            Some(person_matcher::Gender::Female)
        );
        assert_eq!(
            wire_gender_to_matcher("OTHER"),
            Some(person_matcher::Gender::Other)
        );
        assert_eq!(
            wire_gender_to_matcher("unknown"),
            Some(person_matcher::Gender::Unknown)
        );
        assert_eq!(wire_gender_to_matcher("nonbinary-typo"), None);
    }

    /// A missing/absent birth date, gender, or identifiers list never
    /// panics `probe_from_wire` — it simply excludes those components, the
    /// same "no evidence, no confidence" posture the comparator itself
    /// takes.
    #[test]
    fn probe_from_wire_tolerates_sparse_records() {
        let json = serde_json::json!({
            "id": "0c4f1e2a-0000-4000-8000-000000000003",
            "name": { "family": "Haddad", "given": [] }
        });
        let record: WireRecord = serde_json::from_value(json).expect("valid wire record");
        let probe = probe_from_wire(&record);
        assert_eq!(probe.birth_date, None);
        assert_eq!(probe.gender, None);
        assert!(probe.identifiers.is_empty());
    }

    // ---------- resolve_sources (SEC-B7) ----------

    /// Both URLs configured, loopback, no token: the job starts.
    #[test]
    fn resolve_sources_starts_for_loopback_urls_without_a_token() {
        let resolved = resolve_sources(
            Some("http://127.0.0.1:5150/api/persons".to_string()),
            Some("http://127.0.0.1:5160/api/workers".to_string()),
            None,
        );
        assert!(resolved.is_some());
    }

    /// A remote person URL with no token refuses to start (SEC-B7).
    #[test]
    fn resolve_sources_refuses_a_remote_person_url_without_a_token() {
        let resolved = resolve_sources(
            Some("https://person.example.com/api/persons".to_string()),
            Some("http://127.0.0.1:5160/api/workers".to_string()),
            None,
        );
        assert!(resolved.is_none());
    }

    /// A remote worker URL with no token also refuses to start — the
    /// SEC-B7 rule applies to both fetch sources, not just the write
    /// target.
    #[test]
    fn resolve_sources_refuses_a_remote_worker_url_without_a_token() {
        let resolved = resolve_sources(
            Some("http://127.0.0.1:5150/api/persons".to_string()),
            Some("https://worker.example.com/api/workers".to_string()),
            None,
        );
        assert!(resolved.is_none());
    }

    /// A token makes remote URLs on both sides acceptable.
    #[test]
    fn resolve_sources_starts_for_remote_urls_with_a_token() {
        let resolved = resolve_sources(
            Some("https://person.example.com/api/persons".to_string()),
            Some("https://worker.example.com/api/workers".to_string()),
            Some("secret-token".to_string()),
        );
        assert!(resolved.is_some());
    }

    /// `LINK_GRAPH_SUGGEST_URL_PERSON` unset: the job is simply not
    /// configured to run (not an error, no warning needed — this is the
    /// default-off shape every other periodic worker in this crate uses).
    #[test]
    fn resolve_sources_is_none_when_person_url_is_unset() {
        assert!(
            resolve_sources(
                None,
                Some("http://127.0.0.1:5160/api/workers".to_string()),
                None
            )
            .is_none()
        );
    }

    /// `LINK_GRAPH_SUGGEST_URL_PERSON` set but `_WORKER` unset: refuses to
    /// start rather than running a pass that can never find a worker-side
    /// match.
    #[test]
    fn resolve_sources_is_none_when_worker_url_is_missing() {
        assert!(
            resolve_sources(
                Some("http://127.0.0.1:5150/api/persons".to_string()),
                None,
                None
            )
            .is_none()
        );
    }

    // ---------- run_suggestion_pass (mocked pipeline) ----------

    /// A mock [`IdentitySource`] returning a fixed set of probes.
    struct MockSource(Vec<(EntityRef, IdentityProbe)>);

    #[async_trait]
    impl IdentitySource for MockSource {
        fn label(&self) -> &'static str {
            "mock"
        }

        async fn fetch_all(&self) -> ModelResult<Vec<(EntityRef, IdentityProbe)>> {
            Ok(self.0.clone())
        }
    }

    /// A mock [`SuggestionSink`] capturing every candidate it was asked to
    /// post; optionally always-failing, to exercise the per-candidate
    /// error path.
    #[derive(Default)]
    struct MockSink {
        posted: Mutex<Vec<IdentityCandidate>>,
        always_fail: bool,
    }

    #[async_trait]
    impl SuggestionSink for MockSink {
        async fn post_suggestion(&self, candidate: &IdentityCandidate) -> ModelResult<()> {
            if self.always_fail {
                return Err(ModelError::Any(Box::new(std::io::Error::other(
                    "mock sink failure",
                ))));
            }
            self.posted.lock().unwrap().push(candidate.clone());
            Ok(())
        }
    }

    fn entity_ref(entity_type: EntityType, id: u128) -> EntityRef {
        EntityRef::new(entity_type, Uuid::from_u128(id))
    }

    fn sharing_pair() -> ((EntityRef, IdentityProbe), (EntityRef, IdentityProbe)) {
        let probe = IdentityProbe {
            name: Some(ProbeName {
                family: "Smith".to_string(),
                given: "Jane".to_string(),
            }),
            birth_date: NaiveDate::from_ymd_opt(1980, 6, 15),
            gender: Some(person_matcher::Gender::Female),
            identifiers: vec![ProbeIdentifier::new("nhs", "9434765919").unwrap()],
        };
        (
            (entity_ref(EntityType::Person, 1), probe.clone()),
            (entity_ref(EntityType::Worker, 2), probe),
        )
    }

    /// End to end (mocked): a shared-identifier person/worker pair is
    /// fetched, scored at the identifier ceiling, and `POSTed` exactly once;
    /// the run stats reflect the fetch + candidate + post counts.
    #[tokio::test]
    async fn run_suggestion_pass_posts_the_matching_candidate() {
        let (person, worker) = sharing_pair();
        let persons = MockSource(vec![person.clone()]);
        let workers = MockSource(vec![worker.clone()]);
        let sink = MockSink::default();

        let stats = run_suggestion_pass(
            &persons,
            &workers,
            &sink,
            DEFAULT_MAX_CANDIDATES,
            DEFAULT_MAX_EDGES_PER_RUN,
        )
        .await
        .expect("pass succeeds");

        assert_eq!(stats.persons_fetched, 1);
        assert_eq!(stats.workers_fetched, 1);
        assert_eq!(stats.candidates, 1);
        assert_eq!(stats.posted, 1);
        assert_eq!(stats.failed, 0);

        let posted = sink.posted.lock().unwrap();
        assert_eq!(posted.len(), 1);
        assert_eq!(posted[0].person, person.0);
        assert_eq!(posted[0].worker, worker.0);
        assert!(posted[0].score.identifier_match);
        assert!((posted[0].score.confidence - IDENTIFIER_MATCH_CEILING).abs() < f64::EPSILON);
    }

    /// An unrelated person/worker pair never becomes a candidate, so
    /// nothing is `POSTed`.
    #[tokio::test]
    async fn run_suggestion_pass_posts_nothing_for_unrelated_records() {
        let person = (
            entity_ref(EntityType::Person, 10),
            IdentityProbe {
                name: Some(ProbeName {
                    family: "Smith".to_string(),
                    given: "Jane".to_string(),
                }),
                birth_date: NaiveDate::from_ymd_opt(1980, 6, 15),
                gender: Some(person_matcher::Gender::Female),
                identifiers: vec![],
            },
        );
        let worker = (
            entity_ref(EntityType::Worker, 11),
            IdentityProbe {
                name: Some(ProbeName {
                    family: "Ivanov".to_string(),
                    given: "Dmitri".to_string(),
                }),
                birth_date: NaiveDate::from_ymd_opt(1955, 11, 2),
                gender: Some(person_matcher::Gender::Male),
                identifiers: vec![],
            },
        );
        let persons = MockSource(vec![person]);
        let workers = MockSource(vec![worker]);
        let sink = MockSink::default();

        let stats = run_suggestion_pass(
            &persons,
            &workers,
            &sink,
            DEFAULT_MAX_CANDIDATES,
            DEFAULT_MAX_EDGES_PER_RUN,
        )
        .await
        .expect("pass succeeds");

        assert_eq!(stats.candidates, 0);
        assert_eq!(stats.posted, 0);
        assert!(sink.posted.lock().unwrap().is_empty());
    }

    /// A candidate whose POST fails is counted in `failed`, not `posted`,
    /// and does not stop the run (there is only one candidate here, but
    /// the accounting is what is under test).
    #[tokio::test]
    async fn run_suggestion_pass_counts_a_failed_post_without_erroring_the_run() {
        let (person, worker) = sharing_pair();
        let persons = MockSource(vec![person]);
        let workers = MockSource(vec![worker]);
        let sink = MockSink {
            posted: Mutex::new(Vec::new()),
            always_fail: true,
        };

        let stats = run_suggestion_pass(
            &persons,
            &workers,
            &sink,
            DEFAULT_MAX_CANDIDATES,
            DEFAULT_MAX_EDGES_PER_RUN,
        )
        .await
        .expect("pass itself still succeeds; only the POST failed");

        assert_eq!(stats.candidates, 1);
        assert_eq!(stats.posted, 0);
        assert_eq!(stats.failed, 1);
    }

    /// Empty sources produce an empty, successful pass — no panics.
    #[tokio::test]
    async fn run_suggestion_pass_handles_empty_sources() {
        let persons = MockSource(vec![]);
        let workers = MockSource(vec![]);
        let sink = MockSink::default();

        let stats = run_suggestion_pass(
            &persons,
            &workers,
            &sink,
            DEFAULT_MAX_CANDIDATES,
            DEFAULT_MAX_EDGES_PER_RUN,
        )
        .await
        .expect("pass succeeds");
        assert_eq!(stats, SuggestionRunStats::default());
    }

    // ---------- T-33: max_candidates / max_edges_per_run env parsing ----------

    #[test]
    fn max_candidates_from_env_falls_back_to_default_when_unset_zero_or_unparseable() {
        assert_eq!(super::resolve_max_candidates(None), DEFAULT_MAX_CANDIDATES);
        assert_eq!(
            super::resolve_max_candidates(Some("0")),
            DEFAULT_MAX_CANDIDATES
        );
        assert_eq!(
            super::resolve_max_candidates(Some("not a number")),
            DEFAULT_MAX_CANDIDATES
        );
        assert_eq!(super::resolve_max_candidates(Some("7")), 7);
    }

    #[test]
    fn max_edges_per_run_from_env_falls_back_to_default_when_unset_zero_or_unparseable() {
        assert_eq!(
            super::resolve_max_edges_per_run(None),
            DEFAULT_MAX_EDGES_PER_RUN
        );
        assert_eq!(
            super::resolve_max_edges_per_run(Some("0")),
            DEFAULT_MAX_EDGES_PER_RUN
        );
        assert_eq!(
            super::resolve_max_edges_per_run(Some("nope")),
            DEFAULT_MAX_EDGES_PER_RUN
        );
        assert_eq!(super::resolve_max_edges_per_run(Some("5")), 5);
    }

    // ---------- T-33: run_suggestion_pass scale controls ----------

    /// A block bigger than `max_candidates` is truncated — proven here at
    /// the `run_suggestion_pass` level (not just `generate_candidates_bounded`
    /// directly), so the parameter is genuinely threaded all the way
    /// through, not merely available deeper in the stack.
    #[tokio::test]
    async fn run_suggestion_pass_threads_max_candidates_through_to_blocking() {
        let person_probe = IdentityProbe {
            identifiers: vec![ProbeIdentifier::new("nhs", "1231231234").unwrap()],
            ..IdentityProbe::default()
        };
        let persons = MockSource(vec![(entity_ref(EntityType::Person, 500), person_probe)]);
        let worker_probes: Vec<(EntityRef, IdentityProbe)> = (0..10u128)
            .map(|i| {
                (
                    entity_ref(EntityType::Worker, 600 + i),
                    IdentityProbe {
                        identifiers: vec![ProbeIdentifier::new("nhs", "1231231234").unwrap()],
                        ..IdentityProbe::default()
                    },
                )
            })
            .collect();
        let workers = MockSource(worker_probes);
        let sink = MockSink::default();

        let stats = run_suggestion_pass(&persons, &workers, &sink, 4, DEFAULT_MAX_EDGES_PER_RUN)
            .await
            .expect("pass succeeds");

        assert_eq!(
            stats.candidates, 4,
            "max_candidates=4 must cap the 10-worker block at the run_suggestion_pass level"
        );
        assert_eq!(stats.posted, 4);
        assert_eq!(
            stats.dropped, 0,
            "under max_edges_per_run, nothing is dropped"
        );
    }

    /// When `generate_candidates_bounded` returns more candidates than
    /// `max_edges_per_run`, only the highest-confidence ones are `POSTed`;
    /// the rest are counted in `dropped`, not silently lost from the
    /// stats. Three independent (different-block) pairs are constructed
    /// with three distinct, known confidences via the DOB-proximity table
    /// (`score_dob_pair`), so the expected posting order is unambiguous.
    #[tokio::test]
    async fn run_suggestion_pass_caps_edges_per_run_and_prioritises_highest_confidence() {
        fn pair(
            tag: u128,
            family: &str,
            worker_dob: NaiveDate,
        ) -> ((EntityRef, IdentityProbe), (EntityRef, IdentityProbe)) {
            let family = family.to_string();
            let person_probe = IdentityProbe {
                name: Some(ProbeName {
                    family: family.clone(),
                    given: "Sam".to_string(),
                }),
                birth_date: NaiveDate::from_ymd_opt(1980, 1, 10),
                gender: Some(person_matcher::Gender::Female),
                identifiers: vec![],
            };
            let worker_probe = IdentityProbe {
                name: Some(ProbeName {
                    family,
                    given: "Sam".to_string(),
                }),
                birth_date: Some(worker_dob),
                gender: Some(person_matcher::Gender::Female),
                identifiers: vec![],
            };
            (
                (entity_ref(EntityType::Person, 700 + tag), person_probe),
                (entity_ref(EntityType::Worker, 800 + tag), worker_probe),
            )
        }

        // Three genuinely distinct surnames (different Soundex codes, not
        // merely different trailing digits — Soundex ignores non-letters,
        // so e.g. "Family1"/"Family2"/"Family3" would all collide on one
        // "Family" code and defeat the isolation this test relies on) so
        // each pair blocks independently and only the intended (person,
        // worker) pair within it is ever compared.
        // A: exact DOB match -> dob_score 1.0 -> highest confidence.
        let (person_a, worker_a) =
            pair(1, "Anderson", NaiveDate::from_ymd_opt(1980, 1, 10).unwrap());
        // B: DOB one day off -> dob_score 0.95 -> middle confidence.
        let (person_b, worker_b) = pair(2, "Baxter", NaiveDate::from_ymd_opt(1980, 1, 11).unwrap());
        // C: same year only -> dob_score 0.50 -> lowest (but still >= 0.7 threshold).
        let (person_c, worker_c) =
            pair(3, "Castillo", NaiveDate::from_ymd_opt(1980, 7, 25).unwrap());

        let persons = MockSource(vec![person_a.clone(), person_b.clone(), person_c.clone()]);
        let workers = MockSource(vec![worker_a, worker_b, worker_c]);
        let sink = MockSink::default();

        let stats = run_suggestion_pass(&persons, &workers, &sink, DEFAULT_MAX_CANDIDATES, 2)
            .await
            .expect("pass succeeds");

        assert_eq!(stats.candidates, 3, "all three pairs qualify (>= 0.7)");
        assert_eq!(stats.posted, 2, "max_edges_per_run=2 caps the POSTs");
        assert_eq!(
            stats.dropped, 1,
            "the lowest-confidence pair (C) is dropped"
        );
        assert_eq!(stats.failed, 0);

        let posted_persons: std::collections::HashSet<EntityRef> = sink
            .posted
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.person)
            .collect();
        assert!(
            posted_persons.contains(&person_a.0),
            "the highest-confidence pair (A, exact DOB) must be posted"
        );
        assert!(
            posted_persons.contains(&person_b.0),
            "the middle-confidence pair (B, DOB off by one day) must be posted"
        );
        assert!(
            !posted_persons.contains(&person_c.0),
            "the lowest-confidence pair (C, same-year-only DOB) must be the one dropped"
        );
    }
}
