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
//! ## Scale controls deferred to T-33
//!
//! `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` / `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN`
//! (OQ-9(d)) are **not** implemented here — T-33's job, alongside the
//! audit-every-POST trail and the non-auto-promotion governance test. This
//! job does apply one defensive, non-configurable cap of its own
//! ([`MAX_FETCH_OFFSET`]) on the fetch pagination loop regardless — both
//! person's and worker's list endpoints now bound `offset` themselves
//! (`MAX_SEARCH_OFFSET` / `MAX_LIST_OFFSET`, both `10_000`), but this job
//! does not rely on a peer enforcing that; it stops itself either way.

use async_trait::async_trait;
use chrono::NaiveDate;
use entity_ref::{EntityRef, EntityType};
use loco_rs::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::{IdentityCandidate, IdentityProbe, ProbeIdentifier, ProbeName, generate_candidates};

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

/// Counts from one suggestion pass, logged (not audited — T-33's job) at
/// the end of [`run_suggestion_pass`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SuggestionRunStats {
    /// Persons fetched this pass.
    pub persons_fetched: usize,
    /// Workers fetched this pass.
    pub workers_fetched: usize,
    /// Candidates `generate_candidates` returned (at/above the threshold).
    pub candidates: usize,
    /// Candidates successfully `POSTed`.
    pub posted: usize,
    /// Candidates whose POST failed (logged individually, run continues).
    pub failed: usize,
}

/// Run one suggestion pass: fetch both sides, block + score
/// ([`generate_candidates`]), and POST every surviving candidate. A single
/// candidate's POST failure is logged and does not abort the rest of the
/// run — matching the family's per-row-error-tolerant posture elsewhere
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
) -> ModelResult<SuggestionRunStats>
where
    P: IdentitySource + ?Sized,
    W: IdentitySource + ?Sized,
    S: SuggestionSink + ?Sized,
{
    let person_probes = persons.fetch_all().await?;
    let worker_probes = workers.fetch_all().await?;
    let candidates = generate_candidates(&person_probes, &worker_probes);

    let mut stats = SuggestionRunStats {
        persons_fetched: person_probes.len(),
        workers_fetched: worker_probes.len(),
        candidates: candidates.len(),
        posted: 0,
        failed: 0,
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

/// Run the suggestion pass periodically until the process exits — the
/// "worker" wiring (OQ-9(c)/(d)), mirroring
/// [`crate::reconcile::run_periodic`]'s shape exactly: `LINK_GRAPH_SUGGEST_SECS`
/// (default 3600), first tick skipped so boot is not blocked, a failed
/// pass is logged and retried next tick. Spawned from `App::after_routes`
/// only when [`sources_from_env`] returns `Some`.
pub async fn run_periodic<P, W, S>(persons: P, workers: W, sink: S)
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
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(secs));
    interval.tick().await; // consume the immediate first tick
    loop {
        interval.tick().await;
        match run_suggestion_pass(&persons, &workers, &sink).await {
            Ok(stats) => tracing::info!(?stats, "suggestion pass complete"),
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

        let stats = run_suggestion_pass(&persons, &workers, &sink)
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

        let stats = run_suggestion_pass(&persons, &workers, &sink)
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

        let stats = run_suggestion_pass(&persons, &workers, &sink)
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

        let stats = run_suggestion_pass(&persons, &workers, &sink)
            .await
            .expect("pass succeeds");
        assert_eq!(stats, SuggestionRunStats::default());
    }
}
