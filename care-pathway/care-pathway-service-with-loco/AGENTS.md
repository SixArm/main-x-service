# AGENTS.md — Care Pathway Service

Entry point for AI coding agents working in the `care-pathway-service`
crate: a registry of **clinical care-pathway** records.

> Read [`spec/index.md`](./spec/index.md) first — the living spec.

## What this is

A **loco.rs** service for care-pathway records: CRUD + matching,
embedding the canonical [`care-pathway-matcher`](../care-pathway-matcher-rust-crate).
The API DTO **is** `care_pathway_matcher::CarePathway` — stored verbatim
(JSONB) and matched with the same type, so there is no separate model or
adapter to drift.

| Question | Answer |
|---|---|
| Framework | loco.rs 1.0.1 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration` 2.0). |
| Build / test | `cargo build` · `cargo test` (DB-free: `tests/matching.rs` + controller 422 pin) · `cargo test -- --ignored` (request tests, need Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `care_pathways` table: `pid`, `name`, `data` (JSONB CarePathway), `active`, soft-delete. |

## API surface

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/care-pathways` | Create (body: `CarePathway`; blank `name` → `422`) → `{pid, name}` |
| GET | `/api/care-pathways` | List active (capped 100) |
| GET | `/api/care-pathways/search?q=` | Tantivy full-text search (`?fuzzy=true`, `?phonetic=true`) |
| GET | `/api/care-pathways/{pid}` | Fetch the stored `CarePathway` (record-level ABAC; a `mask`-obligation allow returns the redacted view) |
| GET | `/api/care-pathways/{pid}/masked` | The masked view: provider name / provider id redacted |
| GET | `/api/care-pathways/{pid}/export` | GDPR right-of-access export (audited as a disclosure; masked when the policy says so) |
| PUT | `/api/care-pathways/{pid}` | Replace payload |
| DELETE | `/api/care-pathways/{pid}` | Soft-delete |
| POST | `/api/care-pathways/match` | Rank a `{query, candidates}` set |
| POST | `/api/care-pathways/check-duplicates` | Match a query against stored pathways |
| POST | `/api/care-pathways/merge` | Merge a duplicate into a survivor (`422` equal pids, `404` unknown) |
| GET | `/api/care-pathways/merges/recent` | Merge-history records |
| GET | `/api/care-pathways/whoami` | Verified bearer-token claims (`401` without one) |
| POST | `/api/care-pathways/import` | **Native bulk import** (§13 T-10): multipart JSONL/CSV/TSV upload → `202 {job_id}`; destructive under ABAC |
| GET | `/api/care-pathways/import/{id}` | Import job status + counts + `errors_url` |
| POST | `/api/care-pathways/export` | **Native bulk export**: `{format, q, limit, offset, masking_profile, include_soft_deleted}` → `202 {job_id}`; the unmasked `full` profile is destructive under ABAC |
| GET | `/api/care-pathways/export/{id}` | Export job status + `download_url` |
| GET | `/api/care-pathways/bulk-jobs` | Recent bulk jobs (native + FHIR Bulk Data — one shared table), newest first; `?kind=&status=` |
| GET | `/api/care-pathways/review-queue` | Stored duplicate-review queue: pairs a keyless bulk-import row queues (`provenance=import`) |
| POST | `/api/care-pathways/review-queue/{id}/decision` | Decide a pending review item (`confirmed`/`rejected`; first-writer-wins) |
| GET | `/api/care-pathways/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/care-pathways/events/recent` | In-memory event stream |
| GET | `/api/care-pathways/insights/{directory,coverage,variants,providers,languages}` | Registry lenses: setting/specialty facets, condition-coverage gaps, cross-provider variants, provider directory, language coverage |
| POST/GET | `/api/care-pathways/{pid}/instances` · `/{pid}/cohort` | Enrol a `person:` URN on a pathway; the chronic cohort view |
| — | `/api/instances/{pid}` (+ `/status` `/review` `/urgency` `/team` `/events` `/steps/{s}/complete`) | Instance lifecycle, review cadence, urgency, care team, steps |
| GET | `/api/instances/{caseload,overdue-reviews,stalled,care-team-load}` | Derived operational views; `stalled` (T-14j): `?idle_days=` (default 60), aging-WIP journeys by last recorded activity |
| POST/GET | `/api/instances/{pid}/segments` (+ `/segments/{seg}/close`, `/clock`) | **Time-based analysis**: record a journey segment (VA / NNVA / UNVA + stage + waste), close a running one, set the pathway clock (no pause, by design) |
| GET | `/api/instances/{pid}/{time-analysis,timeline}` | Per-journey TBA: value-adding ratio, coverage, gaps, handoffs, per-stage anchors + adjacent delays (T-14d), conformance to the enrolled template (T-14i); and the segment/gap wall |
| GET | `/api/care-pathways/{pid}/{time-analysis,constraints}` | Cohort TBA: nearest-rank lead-time percentiles vs an NHS access standard, optionally anchored between two named stages (T-14d: `?from_anchor=&to_anchor=`, or automatic from a standard's own declared anchor, e.g. `cancer_fds_28_days`); censoring-aware Kaplan–Meier survival for time-to-close and time-to-anchor (T-14e: `?discontinued=event\|censor`); ranked constraints; both endpoints splittable by a rule (T-14f: `?contains=&excludes=&compare=` over `stage`/`step`/`event`/`waste`/`outcome`/`setting`/`urgency`); both also carry a CONSORT-style `attrition` trail (T-14g) explaining the denominator; `time-analysis` also carries a template-conformance fully-conformant cohort share (T-14i, not `constraints` — a documented scope decision). All suppress below `min_cell_count` (T-14k; `?mode=withhold\|remove`), and a rule-split's own cross-side protection reuses the same primitive |
| GET | `/api/care-pathways/{pid}/export/{event-log,journey-features}` | **Bulk export codecs** (T-14a): `?format=csv\|jsonl&status=`; `event_log` (bupaR/PM4Py shape) and `journey_features` (one row per instance); gated `Destructive`, audited as a disclosure; never a `subject_ref` or a person/actor URN |
| GET | `/api/care-pathways/{pid}/process-map` | **Directly-follows process map** (T-14b): `?level=stage\|step&status=&mode=`; nodes/edges with instance/occurrence counts + median(+p90) gaps; self-loops kept, `start`/`end` pseudo-nodes; suppressed per node/edge (T-14k), not per cohort |
| GET | `/api/care-pathways/{pid}/variants` | **Journey variants** (T-14c): pathway strings via a named/defaulted/echoed parameter chain (era filter/collapse/combine/filter-mode/truncate); frequency/coverage Pareto + per-position duration lines; suppressed variants folded into `suppressed_instances`, shares renormalised |
| GET | `/api/care-pathways/{pid}/data-quality` | **Journey data-quality and missingness report** (T-14h): `?status=&from_anchor=&to_anchor=`; eight-code closed vocabulary (no segments / open segment past closure / terminal without clock stop / step done before enrolled / steps out of order / segment clipped by clock / coverage below floor / anchors unreached) plus per-stage missingness + entropy; the report is the finding, it never imputes |
| GET | `/api/instances/{flow,time-standards}` | Little's Law flow (λ/μ/ρ/κ/τ) and the access-standard catalogue |
| GET | `/api/instances/{pid}/journey` | **Stitched journey**: follows `continues_as` across services, each leg fetched under the *caller's* credential; combined figures withheld unless every leg resolved |
| POST/GET/DELETE | `/api/instances/{pid}/links` (+ `/{id}`) · `GET /api/instances/links` | **Cross-service journey links**: the `continues_as` edge from a pathway instance into the next episode (another instance, a `patient_flow_stay`, or a `case`); high-sensitivity governance, audited; a denial is reported as `404` so it cannot disclose the journey's existence; the bulk pull is the aggregator's reconciliation source and is a privileged read |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (text-exposition; root path, public under auth enforcement) |

Plus loco's default `/_health`, `/_ping`. Every CRUD action writes an
`audit_logs` row and publishes a `created`/`updated`/`deleted` event.
`GET /api/care-pathways` and `GET /api/care-pathways/search` take
`?limit=`/`?offset=` and report `X-Total-Count`/`X-Limit`/`X-Offset`.

This crate is also the family's **reference implementation** of the
compliance controls in
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2 (spec §12) — a full `/api/compliance/*` surface (posture, SBOM, audit
hash-chain verify, row-integrity verify, MAC'd external-witness
checkpoints) plus a mounted FHIR R5 `PlanDefinition` surface (spec
§12.3). See [README.md](./README.md) for both endpoint tables in full;
the tables above cover only the native `/api/care-pathways` surface.

## MVP scope

CRUD + Tantivy full-text/fuzzy/phonetic search + matching, with payload validation
(`condition_codes` ICD-10 / ICD-11 / SNOMED CT SCTID Verhoeff;
`identifiers` UUID / DOI shapes; `in_language` BCP-47 syntax;
`src/validation.rs`), OpenAPI 3 +
Swagger UI (`src/openapi.rs`, `controllers/docs.rs`), an audit log +
in-memory event stream on every CRUD/merge (`models/audit_logs.rs`,
`src/streaming.rs`), record merge (`src/merge.rs` + `models/merge_records.rs`,
`POST /merge`), offline PASETO v4.public verification (`src/auth.rs`,
embeds `authentication-verifier`; `/whoami` + audit `actor`), and blanket
`/api/*` auth enforcement (`auth::enforce` + an `after_routes` middleware
in `app.rs`) wired but **off by default** — gated by
`CARE_PATHWAY_REQUIRE_AUTH`. The durable event bus's
Phase-2 outbox/relay landed (`models/event_outbox.rs`, `src/relay.rs`),
default-off via `CARE_PATHWAY_EVENT_TRANSPORT` (`memory` unless set to
`outbox`). **Tantivy full-text/fuzzy/phonetic search** (`src/search/`)
replaces the `ILIKE` name search and backs search-blocked
`check-duplicates` candidates. **Privacy** (`src/privacy.rs`) provides
field masking (`provider_name` / `provider_id`), the always-masked
`/masked` view, and the audited GDPR `/export`, wired to the ABAC `mask`
obligation via `auth::authorize_record` + `auth::care_pathway_resource_attrs`
(`care_setting`, `sensitive_setting` for the mental-health/palliative
special-category settings). A pathway *template* names no patient, so
the masked field set is thin — see `src/privacy.rs`'s module docs for
why, and for the explicit note that the patient-identifying linkage
(`pathway_instances.subject_ref`) is a separate, not-yet-addressed
surface. The durable bus's real broker sink (BUS-3, `FluvioSink` in
`src/relay.rs`, behind this crate's own `fluvio` Cargo feature, off by
default) landed 2026-08-03, ported from case-service's BUS-1 reference —
see `agents/share/event-bus.md`. **Time-based analysis** (`src/tba.rs` +
`src/controllers/tba.rs`, spec §6.18, T-13) and **cross-service journey
links** (`continues_as`; `src/journey.rs` + `src/controllers/links.rs`,
spec §6.19) landed 2026-08-23 through 2026-08-27 — see the API surface
table above and `agents/share/time-based-analysis.md` /
`../../spec/time-based-analysis.md` for the full contract. The pathway
analytics suite T-14 builds on TBA. All twelve of this crate's own
backend sub-tasks have landed. T-14 also has a thirteenth,
front-end-only sub-task, T-14l, in the sibling
`care-pathway-front-end-with-svelte` crate — out of this crate's own
scope, but also landed (2026-09-11), so T-14 is complete across both
crates. See
`../spec/13-tasks.md` T-14a/T-14m/T-14k/T-14b/T-14c/T-14d/T-14e/T-14f/T-14g/T-14h/T-14i/T-14j
for each one's documented scope deviations from its original spec
text: three landed out of T-14's own suggested build order, since each
needed none of the sibling T-14 sub-tasks ahead of it to be useful
now: **T-14a** (event-log/journey-feature bulk export codecs,
`src/analytics.rs` + `src/controllers/exports.rs`), **T-14m** (the
seeded synthetic journey-cohort generator + `journeys:seed` task,
`src/data/journeys.rs` + `src/tasks/journeys_seed.rs`) — both
2026-09-09 — and, all 2026-09-10, **T-14k** (disclosure control: the
shared `min_cell_count`/`Mode` primitive plus secondary suppression of
stratified marginals, `src/suppression.rs`, wired into
`cohort_time_analysis` and, closing a real gap, `cohort_constraints`),
**T-14b** (the directly-follows process map, pure graph-building
alongside T-14a's codecs in `src/analytics.rs`, suppressed per
node/edge via T-14k's primitive rather than per cohort), and **T-14c**
(journey variants — pathway strings via a `TreatmentPatterns`-derived
transform chain, its own new `src/variants.rs`, frequency/coverage
Pareto folding suppressed variants into a disclosed count rather than
listing them). **T-14d** (also 2026-09-10) landed *in* the suggested
order — the third of the "three derivations" trio right after T-14b
and T-14c: stage anchors + adjacent delays extend `tba::analyze`
directly (`StageAnchor`/`Delay`/`anchored_compliance` in `src/tba.rs`
itself, not a new sibling module); cohort compliance can score a named
two-stage interval instead of the whole clock
(`src/controllers/tba.rs`'s `?from_anchor=&to_anchor=`, or
automatically from a standard's own catalogue-declared anchor —
`cancer_fds_28_days` is the one standard that declares one, every
other standard stays whole-clock); an unreached anchor pair counts as
`Compliance::unreached`, a third verdict never folded into
`within`/`breached`; and T-14a's own reserved `anchors_delays` export
column is wired in the same change, since it is a per-instance
property T-14c's still-unwired `variant` column is not. **T-14e**
(2026-09-10, also in the suggested order — right after T-14d)
extends `src/tba.rs` again: `Observation`/`close_observation`/
`anchor_observation`/`KaplanMeier`/`kaplan_meier` (a censoring-aware
survival estimate over time-to-close, and — reusing T-14d's own
`from_anchor`/`to_anchor` pair — time-to-anchor) plus `LogRank`/
`log_rank` (a two-sample Mantel–Haenszel test, unit-tested but still
with no HTTP surface — see T-14f below for why landing the split
mechanism didn't change that). `?discontinued=event|censor`
(default `event`) selects whether a discontinued closure counts as
the Kaplan–Meier event or a censoring; the whole `survival` block is
withheld under the identical suppression decision as the percentile
detail, never a separate one. **T-14f** (2026-09-11, also in the
suggested order — right after T-14e) is a new file,
[`src/split.rs`](./src/split.rs) (named `split`, not `rules`, since
`crate::instances` is already aliased `rules` throughout the
controller layer): `Predicate`/`Features`/`Rule`/`partition` for
`?contains=&excludes=` (AND / none-of, over
`stage`/`step`/`event`/`waste`/`outcome`/`setting`/`urgency`, reusing
`analytics::SegmentInput`/`StepInput`/`EventInput` — the same rows
T-14a's event-log codec already loads) and `split_table` — **the first
real caller of T-14k's own `Table`/`decide` primitive**, which had
stood ready but unused since T-14k landed. `?compare=true` reports the
complement side alongside the matched one; a lone small side recruits
its sibling's suppression even when the sibling alone clears the
floor, closing a real gap where a suppressed side's per-stage/per-waste
sums would otherwise be recoverable as `unsplit − complement`. Wired
into `time-analysis` and `constraints` only — `process-map`/`variants`
each have their own differently-shaped suppression and their own
notion of "compare", so wiring the same contract onto them is a
documented follow-up, not attempted here. `log_rank` itself is still
not called by this change: T-14f's split compares sides via the
already-computed cohort/compliance/survival figures, not a curve-vs-curve
log-rank test, which would need raw `Observation`s `Survival` does not
expose — a further, still-open follow-up beyond this change's scope.
**T-14g** (also 2026-09-11, again next in the suggested order) extends
`src/tba.rs` once more, not a new sibling module (a CONSORT step
record is mechanical bookkeeping, not a genuinely separate algorithm):
`AttritionStep`/`ATTRITION_STEP_LABELS`/`ATTRITION_RULE_PARENT`/
`attrition_trail()`/`attrition_rule_branch()`. Both `time-analysis`
and `constraints` gain an `attrition` key — the ordered pipeline steps
(`enrolled_on_pathway` → `status_filter` → `window` →
`degenerate_clock` → `coverage_floor` → `suppression`, plus
`rule_filter`/`matched`/`complement` when T-14f's split is active)
explaining the denominator inside the response. `window` and
`coverage_floor` are honestly disclosed, not invented: no such filter
exists in this crate yet, so both report zero exclusions.
`degenerate_clock` is the one exception worth naming: it discloses the
count of instances whose clock is not strictly forward
**without excluding them** — they stay in
`cohort`/`compliance`/`survival` exactly as they always have, so this
task changes no existing figure, only what is now visible about it.
**T-14h** (also 2026-09-11, again next in the suggested order) is a
new file, `src/data_quality.rs` (not a further `src/tba.rs`
extension — detectors over already-computed figures, not an
elapsed-time computation, the same reasoning T-14c's/T-14f's/T-14k's
own new files followed): `DQ_CODES` (an eight-entry closed vocabulary
matching T-14m's `DEFECT_CODES` name-for-name), eight `has_*`
detectors, `binary_entropy_bits` (Shannon entropy of a per-stage
present/absent Bernoulli variable), and `build_report`. HTTP surface:
`GET /api/care-pathways/{pid}/data-quality`
(`src/controllers/data_quality.rs`, `?status=&from_anchor=&to_anchor=`),
gated like the sibling cohort views — no record-level ABAC, the
blanket guard only. Missingness is reported **per stage** only, not
per field: the spec text names no field list, and inventing one would
be exactly the kind of unstated assumption this crate's discipline
refuses. This is the **first task to actually exercise T-14m's
generator** end to end rather than a hand-built fixture, and doing so
found and fixed two real generator bugs in `src/data/journeys.rs`: a
defect's own injected segment could itself be clipped by the base
instance's untouched random clock (`widen_clock_stop_past`), and
`terminal_without_clock_stop` cleared `closed_on` alongside
`clock_stop_at`, landing on the rarer double-missingness "as of now"
clock fallback rather than its primary "closed_on" one — which
incidentally tripped `coverage_below_floor` on every seed tried,
fixed by setting `closed_on` to the day after the journey's own last
segment activity. `open_segment_past_closure`'s dedicated instance
genuinely also fires `segment_clipped_by_clock` by design, not by
residual bug: an open segment on a terminal instance is clipped once
its effective end is bounded by "as of now" — exactly
`has_segment_clipped_by_clock`'s own stated rule. Accordingly, the
acceptance text's "each code exactly once per defect" is verified
per defect in an isolated one-instance cohort, not across one shared
eight-defect cohort, since several defects are, by construction, also
low-coverage or clock-clipped journeys that no fixed seed can reliably
keep apart at once.
**T-14i** (also 2026-09-11, again next in the suggested order) is a
new file, `src/conformance.rs` (a genuinely separate concern from
`src/tba.rs`, the same reasoning T-14b's/T-14c's/T-14f's/T-14h's own
new files followed — sequence-order comparison, not an elapsed-time
computation): `StepRecord`, `PairVerdict`
(`in_order`/`inverted`/`skipped` — a pair with either endpoint undone
verdicts `skipped`, but still counts against the fixed structural
`declared_pairs` denominator, same as a genuine inversion would),
`conformance()`, `CohortConformance`/`cohort_conformance()` (excludes
instances with fewer than two declared steps from its own denominator
rather than counting them either way). HTTP surface,
`src/controllers/tba.rs`: `load_step_records`/`count_escalation_events`
(per instance) and `load_conformance_inputs` (cohort, bulk, no N+1),
wired into `GET /api/instances/{pid}/time-analysis` (a new
`conformance` key) and `GET /api/care-pathways/{pid}/time-analysis` (a
new cohort `conformance` share, withheld under the identical
suppression decision `survival`/`split` already use) — **not**
`cohort_constraints`, since template conformance is not a
recoverable-time constraint finding. Unlike T-14h, this task does
**not** exercise T-14m's generator: `journeys:seed` has no defect
exercising step order at all, so `tests/requests/conformance.rs`
builds each instance through the real enrolment/step/event endpoints
and only backdates `done_on`/`closed_on` directly on the model — the
one pair of fields those endpoints always stamp at "now"/"today",
which would otherwise make every within-test-run completion order
indistinguishable. Also wires T-14a's own reserved `conformance`
journey-feature export column (`src/analytics.rs`'s
`journey_feature_row`, a third parameter alongside the analysis,
matching T-14d's `anchors_delays` precedent) — a compact scalar
summary, not the per-pair breakdown the dedicated endpoint carries.
**T-14j** (also 2026-09-11, again next in the suggested order — the
last of the twelve) extends `src/instances.rs` itself, not a new
sibling file (a "latest of several optional timestamps, past a
threshold" fold is a small extension of that module's own remit, not
a genuinely separate algorithm the way T-14b's/T-14c's/T-14f's/
T-14h's/T-14i's own new files were): `ACTIVITY_SOURCES`
(five-entry closed vocabulary), `LastActivity`, `last_activity()`
(folds segment start/end, step `done_on`, and event `occurred_at`
into the single most recent, falling back to `enrolled_on` — the
floor every instance has from the moment it exists), `is_stalled()`.
HTTP surface, `src/controllers/instances.rs`: `load_last_activity_inputs`
(bulk, three bounded queries, no N+1) and `stalled()`, wired at
`GET /api/instances/stalled?idle_days=N` (default 60, echoed;
`?idle_days=` falls back to the default on zero/negative/unparseable
input, matching pagination's own convention rather than erroring like
`target_days` does). **Scope decision:** a recorded review is not a
sixth, separate source — `POST .../review` already emits an
`instance_events` row (`kind: "review"`), covered by the generic
`event` source. Like T-14i, its own DB-gated round trip
(`tests/requests/stalled.rs`) builds fixtures directly rather than via
T-14m's generator, and backdates `enrolled_on` directly on the model —
the one field none of the activity-recording endpoints can set to
anything but "now"/"today", which would otherwise make a freshly
enrolled instance's own floor mask a deliberately old segment's
staleness. Every T-14 sub-task is now complete.

**Native (non-FHIR) bulk import/export** (spec §13 T-10, landed
2026-09-12) reuses the existing `bulk_jobs` table and `ArtifactStore`
(`src/bulk/`) FHIR Bulk Data `$export` already carried — one table
serving both, distinguished by `format` (`ndjson` for FHIR;
`jsonl`/`csv`/`tsv` for native), rather than a second migration.
Stable key (`src/bulk/stable_key.rs`): a **deterministic identifier**
(DOI / Wikidata / `GuidelineId` / URI / UUID, tried in that declared
order) → the **provider-scoped `(provider_id, pathway_code)`** pair →
the explicit `pid`. A keyless row runs the same search-blocked
`MatchingEngine::match_care_pathways` duplicate detection
`check-duplicates` uses and — above `Confidence::Medium`'s 0.7 lower
bound — is still **created** (never silently dropped) *and* queued in
a newly added `review_queue` table (`provenance = "import"`; this
crate had none — case's own BLK-5 precedent, not organization's, since
case also started from zero). Every written row goes through
`streaming::create_and_emit`/`update_and_emit`, so a bulk-imported
pathway gets the same event/audit/search-index side effects as one
created interactively. Export defaults to `crate::privacy::mask_pathway`
(masked); the unmasked `full` profile is `Destructive` under ABAC.
Every export is audited (SEC-B8: the audit write gates delivery — a
failed audit write fails the job rather than silently handing back an
unaudited export), even a zero-row one. Three disclosed scope
decisions, each documented in `src/bulk/`'s own module docs: **(1)**
the row's `active` (soft-delete) column round-trips on export but is
never applied on import — there is no bulk reactivate/deactivate
operation; **(2)** `in_language` (`Vec<String>`) is JSON-encoded like
every other array column even though crate spec §9.4's prose groups it
with the "one column each" scalars — read as "one column, whose cell
happens to hold JSON" rather than a contradiction; **(3)** the per-row
upsert is not SEC-B3 advisory-lock-protected, matching
organization's/case's own documented BLK-5 gap for the identical
reason (`streaming::create_and_emit`/`update_and_emit` open their own
internal transaction, hard-coded to `&DatabaseConnection`). Scope is
**JSONL + CSV + TSV** (no Parquet) — this crate's existing
`ArtifactStore` already supports S3 (unlike organization's/case's
local-only BLK-5 rollout), so native bulk inherits that for free. DB-gated
round trip: `tests/requests/bulk.rs` (idempotent re-import by each
stable-key tier, CSV/JSONL round-trip, keyless dedupe-to-review, masked
vs. full export, zero-row export still audited, `include_soft_deleted`
rejection, unsupported-format `400`, `bulk-jobs` listing).

Still deferred (spec §13): instance-layer masking/authz for
`subject_ref`, and terminology-server code-existence checks.

The published key set
is fetched over HTTP once at boot when `CARE_PATHWAY_PASETO_KEYS_URL` is
set (fetched set wins; warn + env fallback via
`CARE_PATHWAY_PASETO_KEYS` otherwise — the service always boots), **and
then a background loop keeps re-fetching it**
(`CARE_PATHWAY_PASETO_KEYS_REFRESH_SECS`, default 3600), so a rotated
key reaches a running process with no restart; the ABAC policy
(`CARE_PATHWAY_ABAC_POLICY_FILE`) hot-reloads the same way (AU-2,
2026-08-01).

The full **compliance surface** (`src/compliance/`, spec §12) — the
family's reference implementation of
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2 — landed 2026-07-25 through 2026-08-04: a tamper-evident audit hash
chain, keyed HMAC integrity MACs with per-domain HKDF subkeys
(`src/compliance/mac.rs`, embedding the shared `integrity-mac` crate),
MAC'd external-witness chain checkpoints
(`src/compliance/checkpoint.rs` — closes the tail-truncation blind spot
the hash chain alone cannot see), row-level record content hashing,
read/disclosure auditing, GDPR Art. 17 erasure, and a SOUP register +
CycloneDX SBOM + machine-checked requirement→test traceability.
`cargo loco task integrity_key`/`integrity_resign` generate, check, and
re-sign the MAC key without ever logging it. The mounted **FHIR R5**
surface (`src/fhir/`, `controllers/fhir.rs`, spec §12.3) maps the
stored DTO to `PlanDefinition`, with `$validate`, conditional SMART
discovery, and a durable (worker + artifact-store) Bulk Data `$export`.
Family-wide activation order for the compliance controls (all default
off/inert):
[`agents/share/runbooks/integrity-activation.md`](../../agents/share/runbooks/integrity-activation.md).

Auth pivot done in this crate: the family moved from RS256 JWT + JWKS to
cookie sessions + short-lived PASETO v4.public verified offline against a
published Ed25519 key (RS256/JWKS decommissioned); the
`CARE_PATHWAY_REQUIRE_AUTH` flag and enforcement semantics are unchanged,
only the credential changed. See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); `src/auth.rs` verifies PASETO via the
`authentication-verifier` crate (0.9.0, `from_paseto_keys_*`).

## Golden rules

1. **Spec-first.** Update `spec/index.md` with behavioural changes.
2. **Loco-idiomatic.** Endpoints are loco controllers in `app.rs`; new
   tables are `sea-orm-migration` migrations.
3. **Reuse the matcher type.** Do not fork a `CarePathway` DTO.
4. **Auth** comes from the central
   [authentication-service](../../authentication/authentication-service-with-loco):
   cookie sessions + offline PASETO v4.public verification.

## Layout

```
src/
├── app.rs                 loco Hooks (routes, workers, truncate, key/policy-refresh spawn)
├── bin/main.rs             loco CLI entrypoint
├── bin/sbom.rs             `cargo run --bin sbom` — SBOM to stdout, no server boot
├── controllers/
│   ├── care_pathways.rs   CRUD + match + check-duplicates + merge + masked/export + audit/events/disclosures + erase + whoami
│   ├── compliance.rs      posture, SBOM, audit-chain verify, record verify, checkpoint take/verify
│   ├── fhir.rs             mounted FHIR R5 PlanDefinition CRUD/search + $validate + SMART + $export
│   ├── insights.rs         directory/coverage/variants/providers/languages registry lenses
│   ├── instances.rs        instance lifecycle/review/urgency/team/steps/outcomes + caseload/overdue/care-team-load
│   ├── tba.rs              time-based analysis: segment + clock recording, per-instance and cohort views (optionally anchored, T-14d; rule-splittable, T-14f), constraints (also rule-splittable), flow, T-14b process-map, T-14c variants, T-14i conformance loaders (load_step_records/count_escalation_events/load_conformance_inputs, pub(crate), reused by controllers/exports.rs)
│   ├── exports.rs           T-14a: event_log / journey_features bulk export HTTP surface (loads + renders; pure shaping is in src/analytics.rs; its care_setting_string helper is pub(crate), reused by tba.rs's T-14f split for setting: predicates; wires T-14i's conformance column via tba.rs's load_conformance_inputs)
│   ├── data_quality.rs      T-14h: journey data-quality and missingness report HTTP surface (loads segments/steps, reuses tba.rs's resolve_anchor_pair_raw; pure detectors are in src/data_quality.rs)
│   ├── docs.rs             OpenAPI JSON + Swagger UI
│   ├── review_queue.rs      T-10: GET review-queue + POST review-queue/{id}/decision (list/decide crate::models::review_queue rows)
│   └── metrics.rs          root /metrics.prom Prometheus endpoint
├── compliance/
│   ├── mod.rs              posture assembly, data-protection declarations, safety class
│   ├── audit_chain.rs      SHA-256 tamper-evident hash chain over audit_logs
│   ├── mac.rs               keyed HMAC integrity MAC (embeds shared integrity-mac crate, per-domain HKDF subkeys)
│   ├── checkpoint.rs        MAC'd external-witness chain checkpoints (tail-truncation blind spot)
│   ├── record_integrity.rs  row-level content_hash over care_pathways
│   ├── disclosure.rs        purpose-of-use vocabulary + read/disclosure auditing (HIPAA §164.528)
│   ├── erasure.rs           GDPR Art. 17 erasure against the immutable chain
│   ├── soup.rs               SOUP register + CycloneDX SBOM (embeds compliance/soup.tsv + Cargo.lock at compile time)
│   └── bulk.rs               FHIR Bulk Data $export in-process shape (job orchestration lives in bulk/ + workers/)
├── fhir/
│   ├── mod.rs               to/from PlanDefinition conversions
│   ├── profile.rs           family-local StructureDefinition profile + terminology validation
│   ├── resources.rs         resource structs, OperationOutcome, Bundle
│   └── search.rs             FHIR search-param parsing → searchset Bundle
├── bulk/                    T-10: native bulk import/export, sharing the bulk_jobs table + ArtifactStore FHIR $export already used
│   ├── mod.rs               BulkKind/BulkFormat/MaskingProfile + shared constants (MAX_IMPORT_BYTES/ROWS, IMPORT_REVIEW_THRESHOLD)
│   ├── columns.rs            the wire "bulk row" shape (pid + active + every CarePathway field) + CSV column set
│   ├── csv.rs                 CSV/TSV codec (§9.4 flattening)
│   ├── jsonl.rs               JSONL codec (lossless reference)
│   ├── stable_key.rs          identifier → provider+pathway_code → pid priority chain
│   ├── error_report.rs        the per-row import error report
│   ├── pipeline.rs            process_import_job/process_export_job — the testable core
│   ├── worker.rs               the loco BackgroundWorker draining native bulk_jobs rows
│   ├── handlers.rs             POST/GET import, POST/GET export, GET bulk-jobs
│   └── store.rs              ArtifactStore trait + local-filesystem/S3 backends (shared with FHIR $export)
├── workers/
│   └── bulk_export.rs        bg_pg worker materialising FHIR $export NDJSON off the request path
├── tasks/
│   ├── search.rs             `cargo loco task` Tantivy reindex + boot-time reindex-if-empty
│   ├── integrity_key.rs      generate/check/report the MAC root key (never logs the key)
│   ├── integrity_resign.rs   re-sign existing rows after a MAC key rotation
│   └── journeys_seed.rs      T-14m: `journeys:seed` — persists what src/data/journeys.rs generates
├── metrics.rs             process-wide Prometheus registry (CRUD/merge counters + http_requests_total)
├── auth.rs                offline PASETO v4.public verification (AuthUser/MaybeAuthUser) + ABAC, both reloadable (ReloadableVerifier/ReloadablePolicy — AU-2 key/policy hot-reload)
├── version.rs             `Accepts-version` header negotiation middleware (agents/share/api-versioning.md)
├── instances.rs            pure instance lifecycle state machine (active↔on_hold→terminal) + T-14j stalled-journey detection (ACTIVITY_SOURCES/LastActivity/last_activity()/is_stalled())
├── analytics.rs            T-14a event_log/journey_features codecs (anchors_delays column wired by T-14d, conformance column wired by T-14i) + T-14b directly-follows process map, pure, DB-free
├── data/
│   └── journeys.rs          T-14m: pure, DB-free synthetic journey-cohort generator (SplitMix64, deterministic)
├── data_quality.rs         T-14h: journey data-quality and missingness detectors (DQ_CODES, has_*, binary_entropy_bits, build_report), pure, DB-free
├── conformance.rs          T-14i: conformance to the enrolled template — StepRecord/PairVerdict/conformance()/CohortConformance/cohort_conformance(), pure, DB-free
├── suppression.rs          T-14k: disclosure control — min_cell_count/Mode + secondary suppression of stratified marginals, DB-free (first real caller: T-14f's split_table)
├── split.rs                T-14f: rule-based cohort splits — Predicate/Features/Rule/partition/split_table, pure, DB-free
├── tba.rs                 pure time-based analysis: interval union/subtract, the four-bucket
│                          clock partition, gaps, handoffs, nearest-rank percentiles, the NHS
│                          access-standard catalogue (each entry's own optional anchor pair,
│                          T-14d), cohort rollup, constraint ranking, Little's Law, stage
│                          anchors + adjacent delays + anchored compliance (T-14d), and a
│                          censoring-aware Kaplan-Meier survival estimator + a two-sample
│                          log-rank test (T-14e), and a CONSORT-style cohort attrition
│                          record (T-14g). No I/O; `as_of` is a parameter, so it is
│                          deterministic. `src/controllers/tba.rs` (not this file) adds
│                          conformance-to-template loaders (T-14i, `src/conformance.rs`)
├── merge.rs               pure record-merge logic (merge_pathways)
├── openapi.rs             hand-written OpenAPI 3 document
├── privacy.rs             field masking (provider name/id) + GDPR export envelope
├── relay.rs               durable-bus Phase 3 outbox relay (poll/ack loop) + FluvioSink (BUS-3, `fluvio` feature)
├── search/                Tantivy full-text/fuzzy/phonetic index (index.rs schema + mod.rs engine)
├── streaming.rs           CRUD/merge event stream — Phase 1 durable-bus
│                          envelope (Envelope) + EventPublisher seam +
│                          InMemoryPublisher; frozen EventView projection
├── validation.rs          name + condition-code (ICD/SNOMED) + input-size-cap checks → 422
├── variants.rs             T-14c: pure journey-variant (pathway string) transform pipeline + cohort Pareto, DB-free
├── models/
│   ├── care_pathways.rs   CRUD helpers over the stored payload (sets content_hash on every write)
│   ├── audit_logs.rs      audit-trail record/query helpers (chains under pg_advisory_xact_lock)
│   ├── merge_records.rs   merge-history record/query helpers
│   ├── event_outbox.rs    durable-bus Phase 2: OutboxInsert::from_envelope mapping + enqueue (tx-generic) + relay poll/ack
│   ├── bulk_jobs.rs        queued→running→terminal job lifecycle, shared by FHIR $export and native bulk import/export (submit/submit_with_ttl, set_input_url, finish_import, complete, fail)
│   ├── review_queue.rs      T-10: the duplicate-review queue (raw SQL, no SeaORM entity) — upsert/list/decide
│   └── _entities/{care_pathways,audit_logs,merge_records,event_outbox,bulk_jobs,pathway_instances,instance_steps,instance_team,instance_events,instance_measures,instance_segments}.rs  SeaORM entities
├── observability.rs       structured logging + real OpenTelemetry OTLP export (PRO-H12 slice 5 — see below)
migration/src/            …care_pathways, …audit_logs, …merge_records, …event_outbox, …instances (m20260720_…), …outcomes, …compliance (m20260725_000007), …record_integrity, …bulk_jobs, …review_queue (m20260912_000016)
config/                   development/production/test yaml
compose.fluvio.yaml        opt-in local Fluvio broker (`fluvio` feature, BUS-3; not part of CI)
Dockerfile.fluvio-cli       support image for compose.fluvio.yaml's sc-setup step
tests/fluvio_relay.rs       `fluvio`-feature-gated, #[ignore]d live-broker round-trip test
compliance/                crate-root IEC 62304 evidence: lifecycle.md, soup.tsv, traceability.tsv
tests/otlp_export.rs       real OTLP/gRPC export proof, in-process collector, no database
tests/otlp_middleware.rs   the mounted `trace_mw` layer proved end to end over a real HTTP request
tests/otlp_collector/      the shared in-process OTLP/gRPC collector both otlp_* binaries use
```

## OpenTelemetry OTLP export

`src/observability.rs` (repo `tasks.md` PRO-H12 slice 5 of 7, landed
2026-09-01) is a close port of organization-service's
`src/observability.rs` — itself a port of course's, itself person's,
itself link-graph-service's, the family's first working exporter. This
crate carried **no** `src/observability` module at all before this
change, and is the **second of the four loco-idiomatic registries**
(organization, care-pathway, case, portfolio — `src/controllers/`, not
`src/api/rest/`) to carry it. `App::init_logger` installs it (loco's
own `EnvFilter` + formatted layer, plus the `tracing-opentelemetry`
bridge over an OTLP/gRPC exporter); `App::on_shutdown` flushes it.
Export is **on by default** — set `OTLP_ENDPOINT=""` to disable it —
at `OTLP_ENDPOINT` (default `http://localhost:4317`) with
`service.name` from `OTLP_SERVICE_NAME` (default
`care-pathway-service`); both variables are **deliberately
unprefixed**, matching every other crate that carries this pipeline,
not the per-service `CARE_PATHWAY_*` convention
`CARE_PATHWAY_REQUIRE_AUTH` and its siblings use.

**Where this crate's shape forced real adaptation**, confirmed rather
than assumed:

- **Exactly one router-construction surface**, unlike the person-style
  crates' two. This crate is genuinely loco-idiomatic: `App::routes` +
  `App::after_routes` in `src/app.rs` is the only place a router gets
  built — confirmed by grepping `src/` and `tests/` for a second
  `Router::new()`/`create_router`: the one hit (`src/auth.rs`) is a
  unit test for the auth middleware itself, not an app-level router,
  and the request-level test suites (`tests/enforcement.rs`,
  `tests/masking.rs`, …) boot the real `App` via loco's own testing
  harness rather than a second hand-rolled router.
  `observability::trace_mw` is therefore layered **once**, as the
  outermost middleware in `after_routes` — the same precedent
  `require_auth_mw` and `require_version_mw` already set by being
  layered there.
- **No `tonic` rename needed** — this crate declares no `tonic`
  dependency of its own (no gRPC stub — `agents/share/overview.md`'s
  capability matrix), so the in-process OTLP collector tests' `tonic
  0.14` dev-dependency is a plain, un-renamed dependency, exactly as
  course's and organization's are.
- **A new SOUP-register bookkeeping step the other four ports had no
  equivalent of.** This crate is the family's IEC 62304 SOUP-register
  reference implementation (`compliance/soup.tsv`, verified live by
  `every_direct_dependency_is_annotated`), so the eight new
  dependencies (five main, three test-only) each needed a `name<TAB>
  purpose<TAB>safety relevance` row before `cargo test --lib` was
  green — found by running the test, not anticipated in advance.

`tests/otlp_export.rs` and `tests/otlp_middleware.rs` (ported from
organization-service, with `tests/otlp_collector/` — an in-process
OTLP/gRPC collector, unchanged) prove real export against a real gRPC
listener in a normal `cargo test` run: a `tracing` span and a metric
both reach the collector's decoded protobuf, and a served HTTP request
returns a `traceparent` whose trace id matches the exported span's.
None of this needs a database. Landing this raised `cargo test --lib`
from 308 to 316 (8 new `src/observability.rs` unit tests), plus 4 new
tests across the two `tests/otlp_*.rs` binaries. Verified
independently: `cargo fmt --check`, `cargo clippy --all-targets -- -D
warnings`, `cargo deny check`, `cargo bench --no-run`, and the MSRV
check (`cargo +1.96 check --all-targets`) all clean.

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`, `care-pathway-matcher`)
live outside `care-pathway/care-pathway-service-with-loco/`:

```sh
podman build -f care-pathway/care-pathway-service-with-loco/Dockerfile \
  -t care-pathway-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /_health` returns `200`. This exercise found and
fixed two real bugs: (1) `src/compliance/soup.rs` embeds the IEC 62304
SOUP register via a relative `include_str!` from `compliance/soup.tsv`
at the crate root — the Dockerfile now copies that directory
explicitly. (2) `config/production.yaml`'s `mailer.smtp.auth.user`/
`password` used an unquoted Tera `{{ get_env(name="…", default="") }}`
call, which renders as YAML `null` (not `""`) when the env var is
unset — loco's `SmtpAuth` fields are `String`, not `Option<String>`, so
this failed config parsing at boot with "invalid type: unit value,
expected a string". This crate's `.gitignore` also excluded
`config/production.yaml` entirely (a loco scaffold default nobody had
removed), which is why the SMTP bug had never been caught — the file
never left this machine, so no other checkout could exercise it. Both
are fixed (the file is now tracked; see the `.gitignore` for the
reasoning). See `.containerignore` at the repository root (excludes
every crate's `target/`, or the build context would try to copy
hundreds of GB of build artifacts). The wired multi-service
`examples/compose/` stacks (DEP-1) that build on this are not yet
written.

**Re-verified 2026-09-07** for
[`agents/share/runbooks/first-deployment.md`](../../agents/share/runbooks/first-deployment.md)'s
T-28o exercise (run against project-portfolio-management-service, then
rolled here), which found two more real defects the 2026-08-03 pass had
not: (3) the same dead loco JWT `auth:` block as portfolio's — a
`JWT_SECRET` env-var lookup with no `default` crashed a fresh container
with the var unset, for a value this crate reads nowhere (`loco-rs` is
built without the `auth` feature; PASETO, never JWT). loco's
`Config.auth` is `Option<Auth>`, so the fix is deleting the block, not
defaulting it. (4) The Dockerfile never `COPY`'d the `entity-ref`
sibling path dependency at all — added 2026-08-24 for the
`continues_as` journey-link write side, **after** this Dockerfile's
2026-08-03 verification, so the two silently diverged: `cargo build
--release --bin care-pathway-service` failed with "failed to read
/app/link/entity-ref-rust-crate/Cargo.toml". Both fixed and
re-verified against a real container + real Postgres.
