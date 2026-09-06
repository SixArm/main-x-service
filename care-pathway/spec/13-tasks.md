## 13. Tasks

Live entity-level work queue. Tasks that belong to one subproject's
internals should migrate into that crate's spec §13; they are listed
here while the crate specs are thin. Each task has an acceptance
criterion; tick the box when an automated test or clearly described
manual check confirms it. Split tasks too big for one PR
(`T-2a`, `T-2b`).

- [x] **T-1 — Thicken the thin crate docs.**
  *(closed as won't-do, 2026-09-03 — stale against decisions the root
  `AGENTS.md` already made explicit, verified directly rather than
  assumed; see `case/spec/13-tasks.md` T-13 for the identical
  precedent, and `organization/spec/13-tasks.md` T-1 for the same fix
  applied there in the same pass)*
  - [x] ~~Split the service's single-file `spec/index.md` into the
    numbered §-per-file layout~~ — **moot.** Checked directly:
    `care-pathway-service-with-loco/spec/index.md` already carries the
    full §1–§18 numbered structure (`## 1. Purpose and vision` through
    `## 18. Change control`) in one file by choice. Root `AGENTS.md`'s
    "Two spec shapes exist" table fixes the numbering as what matters
    for this shape, not a one-file-per-section split. (The matcher and
    front-end crates, referenced above as carrying "the same task in
    their own §13/§23," carry no matching item today — checked, not
    assumed — so there is nothing further to close alongside this.)
  - [x] ~~Add a service `agents/` reference set (`models.md`,
    `matching.md`, `restful.md`, `testing.md`,
    `spec-driven-development.md`) matching the person-service shape.~~
    — **won't do.** Root `AGENTS.md`'s "Subprojects" section states this
    as a deliberate decision, not a gap: the six original entity crates
    carry a crate-level `agents/` reference set, but the twenty-two
    newer subprojects — care-pathway among them — deliberately do not,
    because "those files restate what the spec already says, and a
    restatement that nobody regenerates is exactly the drift the SDD
    discipline exists to prevent." Confirmed
    `care-pathway-service-with-loco/` has no crate-level `agents/`
    directory today (only its `AGENTS.md` entry point, the newer
    pattern) — that absence is correct per policy; adding the directory
    this task asked for would be the regression, not the fix. (The
    entity-level `care-pathway/agents/` directory this crate sits
    beside already exists and is unaffected — a separate, deliberate
    layer, not what this task asked for.)
  - **Acceptance (superseded):** N/A — task rescoped to won't-do rather
    than closed by a file that was never going to be added.
- [x] **T-2 — Resolve the blank-name status-code discrepancy.**
  - [x] Service crate spec §6 says `422` for a blank `name`; the
    controller returns `400` (`bad_request`). Decide (family
    convention is `422` for validation), align code + spec.
  - **Acceptance:** request-level test posts `{"name": ""}` and gets
    the documented status.
  - **Done (2026-06-13, resolves OQ-1):** `422` is normative. The
    controller's `validate()` returns
    `Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)` on
    blank `name` for both create and update. Pinned un-gated by
    DB-free unit tests in `src/controllers/care_pathways.rs` and by
    the (DB-gated) request tests
    `blank_name_on_{create,update}_returns_422`.
- [x] **T-3 — Audit log + event streaming on CRUD.** (compliance
  driver §12.3)
  - [x] Audit row (action + JSON snapshot + timestamp) per
    create/update/delete. **Done (2026-06-13):** `audit_logs` table
    (migration `m20220101_000002_audit_logs`), `models/audit_logs.rs`
    (`record` / `recent` / `for_entity`); the controller writes a
    best-effort row on each CRUD action (logs on failure, never fails
    the request — the `actor` column is `NULL` until token auth lands,
    T-7). Read endpoints `GET /api/care-pathways/audit/recent` and
    `GET /api/care-pathways/{pid}/audit`.
  - [x] Event publish per CRUD per
    [`agents/share/auditability.md`](../../agents/share/auditability.md).
    **Done:** `streaming.rs` in-memory ring buffer (cap 1 000,
    `OnceLock` global, same MVP shape as the organization service —
    siblings swap a real broker behind `publish`); `created`/`updated`/
    `deleted` published per CRUD; read at
    `GET /api/care-pathways/events/recent`. Durable broker is roadmap
    (§15).
  - **Acceptance:** integration test creates + updates + deletes a
    pathway and reads back three audit rows and three events.
    **Met (DB-gated):** `crud_writes_audit_log_and_events`. Streaming
    is also pinned un-gated by `streaming::publish_and_read_back`.
- [x] **T-4 — Request-level integration tests (PostgreSQL).**
  - [x] loco testing harness over CRUD, `/match`,
    `/check-duplicates` (dev-dependencies already present:
    `serial_test`, `rstest`, `insta`).
  - **Acceptance:** `cargo test` with a Postgres URL covers all
    seven endpoints, including a stored near-duplicate round-trip.
  - **Done (2026-06-13):** `tests/requests/care_pathways.rs` — eight
    loco-style request tests (create, blank-name 422 on
    create/update, get 200/404, list, `/match`,
    `/check-duplicates` near-duplicate round-trip). They are
    `#[ignore]`-gated so the default `cargo test` stays green
    without a database; run with a Postgres URL via
    `cargo test -- --ignored`. (Caveat: authored on a machine with
    no reachable Postgres — first DB-backed run still pending.)
- [x] **T-5 — Front-end tests.**
  - [x] vitest units for `ApiClient` + `CarePathwayRepository`.
    **Done (2026-06-13):** `tests/unit/` (16 tests) — client verb/
    body/headers/bearer/error-classification/empty-body, and every
    repository method's path + verb, including a regression pinning
    `check-duplicates` (not `/duplicates`).
  - [x] Playwright smoke over `/`, `/new`, `/[pid]`, `/[pid]/edit`.
    **Done:** `tests/e2e/smoke.spec.ts` (4 tests) with the API stubbed
    via `page.route`; runs against the production build (`vite
    preview`) to avoid the `vite dev` cold-start module-load race.
    Also fixed two scaffold copy artifacts (`client.ts` "Authentication
    Service" header, `app.html` "Course Service" description).
  - **Acceptance:** both suites run and fail on a broken endpoint
    contract. **Met:** `pnpm test` (vitest, 16) + `pnpm test:e2e`
    (Playwright, 4) both green locally; the `check-duplicates`
    regression test fails if the path drifts. (CI wiring is the
    remaining follow-up.)
- [ ] **T-6 — Search + candidate blocking.** (partly done)
  - [x] Name search endpoint. **Superseded (2026-06-13 → Tantivy
    below):** `GET /api/care-pathways/search?q=` is now Tantivy-backed
    end to end — the `search` handler (`src/controllers/care_pathways.rs`)
    calls `crate::search::engine()`, not the model layer. The original
    `ILIKE`-based `PathwayModel::search`/`search_paged`/`search_count`
    (`src/models/care_pathways.rs`) still exist and still have their
    own DB-gated tests, but are dead code from the controller's point
    of view — confirmed by grep, nothing outside that one file calls
    them. Removing them is a separate, slightly larger cleanup (it
    touches DB-gated tests this pass did not run against a live
    database) and is intentionally left for a follow-up rather than
    bundled here.
  - [x] Tantivy full-text / fuzzy search over the JSONB payload.
    **Done** — `src/search/` (fuzzy + phonetic modes), wired into
    `GET /api/care-pathways/search` (confirmed: `search` handler calls
    `crate::search::engine()...search_page(...)`, no in-memory scan).
  - [ ] Front-end search box. **Still open** — `care-pathway-front-end-with-svelte`
    genuinely has no search route/box today (confirmed: no
    `search`/`q=` references anywhere under its `src/routes/`).
  - [x] Make the `check-duplicates` in-memory scan cap a named,
    documented const with a WARN on hit (interim safety, ahead of
    the redesign). **Done (2026-06-13):** `CHECK_DUPLICATES_SCAN_CAP`
    (= 1000) in `src/controllers/care_pathways.rs`; the handler passes
    it to `Model::list` and emits `tracing::warn!` when the returned
    row count reaches the cap. Pinned by the DB-free unit test
    `check_duplicates_scan_cap_is_the_documented_value`. **Superseded**
    by the next item — the constant is now historical only (its doc
    comment was corrected in the same pass as this task-list update).
  - [x] Replace the 1 000-row in-memory scan in `check-duplicates`
    with search-blocked candidates (NFR-1 / NFR-2; OQ-2). **Done** —
    confirmed live: `check_duplicates` (`src/controllers/care_pathways.rs`)
    calls `crate::search::engine().candidates(&query,
    CHECK_DUPLICATES_CANDIDATE_LIMIT)`, genuinely blocking rather than
    scanning; an unavailable index is `503`, never a silent "no
    duplicates". `CHECK_DUPLICATES_SCAN_CAP` is no longer read by any
    handler — kept only for its historical unit-test pin.
  - **Acceptance:** `check-duplicates` latency test passes at
    100 000 stored pathways. *(Not separately re-verified at this
    scale in this pass — the blocking rollout above is what the
    acceptance criterion was written against; a dedicated
    100 000-row latency test remains a documented gap, not claimed
    met here.)*
- [x] **T-7 — Offline token verification.**
  - [x] Verify offline bearer tokens against the auth-service's published
    key. **Done (2026-06-13, RS256-JWT/JWKS):** `src/auth.rs` embeds the
    [`authentication-verifier`](../../authentication/authentication-verifier-rust-crate)
    crate behind a process-wide `Verifier` built from `CARE_PATHWAY_JWKS`
    / `CARE_PATHWAY_JWT_ISSUER` / `CARE_PATHWAY_JWT_AUDIENCE`. `AuthUser`
    (required) and `MaybeAuthUser` (optional) extractors; `GET
    /api/care-pathways/whoami` is protected. CRUD now stamps the audit
    `actor` from the token when present (previously always `NULL`).
  - [x] *Switch the credential RS256-JWT → **PASETO v4 public** per
    [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)*
    (supersedes the RS256-JWT + JWKS model). **Done:** `Verifier` verifies
    `Authorization: Bearer v4.public.…` tokens against the auth-service's
    published Ed25519 key; the embedded `authentication-verifier` (0.2) is
    PASETO (`from_paseto_keys_value` / `from_paseto_keys_url` replaced
    `from_jwks_*`); same `Claims` shape, verifying `kid`/`iss`/`aud`/`exp`
    with `kid` carried in the footer. Env vars are now
    `CARE_PATHWAY_PASETO_KEYS` / `CARE_PATHWAY_TOKEN_ISSUER` /
    `CARE_PATHWAY_TOKEN_AUDIENCE`.
  - **Acceptance:** no token → `401`; valid signed token → `2xx`.
    **Met:** `whoami_without_token_is_401` (DB-gated) + six un-gated
    crypto unit tests in `auth::tests` (valid→claims, missing/non-bearer/
    expired/tampered→401, empty-verifier rejects) minting a real token +
    matching key in-process.
  - [ ] *Follow-up:* blanket enforcement on every `/api/*` route is
    wired (`auth::enforce`) but **default-off** via
    `CARE_PATHWAY_REQUIRE_AUTH` — activation awaits the coordinated
    family SSO rollout; and paseto-keys-over-HTTP fetch from the auth
    service at boot (currently injected via env).
- [x] **T-8 — Record merge.**
  - [x] Merge confirmed duplicates: union list fields, keep the
    duplicate's title as an `alternate_names` entry, soft-delete the
    duplicate, write a `merge_records` history row (snapshot of the
    transferred payload), and publish a `Merged` event (+ `Deleted`
    for the duplicate). **Done (2026-06-13):** pure `src/merge.rs`
    (`merge_pathways`) + `POST /api/care-pathways/merge` and
    `GET /api/care-pathways/merges/recent`; migration
    `m20220101_000003_merge_records` + `models/merge_records.rs`. Equal
    pids → `422`, unknown pid → `404`. The audit `actor` and merge
    `actor` are stamped from the bearer token (T-7) when present.
  - **Acceptance:** integration test merges two stored pathways and
    verifies survivor contents + soft-deleted duplicate.
    **Met (DB-gated):** `merge_folds_duplicate_into_survivor`,
    `merge_with_equal_pids_is_422`, `merge_unknown_pid_is_404`; the
    merge algorithm is pinned un-gated by five `merge::tests` cases.
  - [ ] *Follow-up:* a front-end merge action from the duplicates list
    (T-5 territory).
- [x] **T-9 — OpenAPI / Swagger + richer validation.**
  - [x] OpenAPI 3 schema + Swagger UI. **Done (2026-06-13):**
    hand-written `src/openapi.rs` (the matcher's `CarePathway` shape is
    the API DTO and is dependency-light, so the schema is authored by
    hand rather than utoipa-derived — same approach as the
    organization service) served by `src/controllers/docs.rs` at
    `GET /api-docs/openapi.json` + `GET /swagger-ui`, registered in
    `app.rs`. Pinned un-gated by `openapi::spec` unit tests
    (`spec_is_wellformed`, `spec_documents_all_seven_endpoints`) and
    (DB-gated) by request tests `openapi_json_is_served` /
    `swagger_ui_is_served`.
  - [x] ICD-10 / ICD-11 / SNOMED CT code-format validation on
    `condition_codes` (`422` on failure). **Done (2026-06-13):**
    `src/validation.rs` format-checks each `condition_codes` entry
    against its `system` — ICD-10 / ICD-11 structural patterns and the
    SNOMED CT SCTID Verhoeff check digit; `Custom` codes need only be
    non-blank. `validate()` reports every problem (incl. blank `name`)
    in one `422`. Pinned un-gated by 9 `validation` unit tests + the
    controller test `malformed_condition_code_returns_422`, and
    (DB-gated) by `malformed_condition_code_on_create_returns_422`.
    Existence-in-a-release validation (terminology server) stays
    deferred.
  - [x] *Extended (2026-06-13):* `identifiers` and `in_language`
    validation. `src/validation.rs` now also structurally checks each
    `identifiers` entry against its `scheme` — a canonical 8-4-4-4-12 hex
    UUID for `Uuid`, the `10.<registrant>/<suffix>` shape for `Doi`, and
    non-blank for every other scheme (the open-valued deterministic ones
    `Wikidata`/`GuidelineId`/`Uri` plus the provider-scoped/custom ones).
    Rejecting a malformed *deterministic* identifier matters because a
    shared value short-circuits the matcher to `1.0` (R-0). `in_language`
    entries are checked for BCP-47 syntax (2–3 or 5–8 letter primary
    subtag, then `-`-separated 1–8 alphanumeric subtags). Pinned un-gated
    by 6 new `validation` unit tests (UUID/DOI accept+reject, open-scheme
    non-blank, indexed-problem reporting, BCP-47 accept+reject,
    malformed-tag problem) and (DB-gated) by
    `malformed_identifier_on_create_returns_422`. IANA-registry and
    terminology-server existence checks stay deferred.
  - **Acceptance:** Swagger UI serves the seven endpoints; malformed
    code test returns `422`. *(Validation leg met; Swagger leg open.)*
- [ ] **T-10 — Bulk import / export.**
  See §9.4, §10.4 and
  [bulk import/export](../../agents/share/bulk-import-export.md).
  - [ ] Migration creating the `bulk_jobs` table (shared doc §3 schema,
    with the `UNIQUE (entity, kind, idempotency_key)` key).
  - [ ] The five endpoints (§9.4): `POST`/`GET`
    `/api/care-pathways/import`, `POST`/`GET`
    `/api/care-pathways/export`, `GET /api/care-pathways/bulk-jobs`.
  - [ ] `bg_pg` worker draining jobs `queued → running →
    completed | completed_with_errors | failed`, with progress updates.
  - [ ] JSONL (lossless reference) + CSV (flattening per §9.4: every
    repeated / nested field a JSON-in-cell) codecs; Parquet
    **export-only**, feature-gated.
  - [ ] Per-row pipeline reusing the single-create validators
    (`src/validation.rs`: ICD/SNOMED code formats, identifier shapes,
    BCP-47) + matcher + review queue: upsert by stable key (deterministic
    scheme-scoped identifier, `(provider_id, pathway_code)`, or `pid`,
    §9.4); keyless / unmatched rows → duplicate detection → review queue
    with `provenance = import`; events + audit not bypassed.
  - [ ] Downloadable per-row error report
    (`row_number, source_line, field, code, message`); one bad row never
    aborts the load; counts reconcile
    (`rows_total = created + upserted + to_review + errored`).
  - [ ] Export masking + audit: `masking_profile` (masked default, full
    gated), `include_soft_deleted` gated, every export audited (even
    zero-row).
  - **Acceptance:** integration tests cover idempotent re-import (same
    file re-upserts to the same state), the per-row error report, a
    keyless dedupe-to-review row (`provenance = import`), masked vs full
    export, and that a zero-row export still writes an audit record.

- [x] **T-11 — Extended regulatory frameworks (§12.4).** HIPAA
  read/disclosure auditing + tamper-evident history; GDPR/EHDS erasure
  against the immutable chain, residency, lawful basis, purpose-of-use;
  ONC/HTI profile + terminology validation, `$validate`, SMART
  discovery, Bulk Data `$export`; IEC 62304 SOUP register + SBOM,
  machine-checked requirement→test traceability, reproducible builds,
  and a runtime posture surface.
  - **Done (2026-07-25):** implemented in the service crate as the
    family's reference implementation — see
    [service spec §12](../care-pathway-service-with-loco/spec/index.md)
    and its §13 T-11–T-14 for the per-framework breakdown, and
    [`spec/compliance` §8](../../spec/compliance/index.md) for the
    repository-wide status and the rollout to the other services.
  - **Acceptance (met):** the audit chain verifies after a Postgres
    JSONB round-trip and reports a `content` break when a row is
    rewritten with raw SQL; erasure destroys content while the chain
    still verifies; adding an un-annotated dependency or orphaning a
    requirement fails the build. Full `--ignored` suite 35/35 vs
    Postgres 18; 177 unit tests; clippy pedantic clean.
  - **Deliberately not claimed:** ONC certification, US Core
    conformance, SMART App Launch, medical-device qualification — see
    [§12.5](12-compliance.md).
- [ ] **T-12 — Compliance follow-ups.** Row-level integrity hashing over
  the entity table; Bulk Data `$export` on the `bg_pg` worker + an
  artifact store; the fail-open decision for audit writes; CI wiring for
  `cargo deny` / SBOM / traceability; an Inferno-style conformance run.
  Tracked in detail as the service spec's §13 T-15.
  - **Acceptance:** each sub-item closed with a test, or explicitly
    re-declared as an accepted limitation in §12.5.

- [x] **T-13 — Time-based analysis (TBA-1 … TBA-7).** The time dimension
  of the pathway: a recorded journey **segment** primitive, an explicit
  pathway **clock**, and the derived per-instance / cohort / constraint /
  flow views. Unifies Barker's time-based analysis (the value-adding
  ratio), value stream mapping (the VA / NNVA / UNVA classification and
  the VT / PT / LT / %A / #HO metric names) and queueing theory (λ / μ /
  ρ / κ / τ, Little's Law). Full contract, including the parts
  deliberately refused, in the cross-cutting
  [`time-based-analysis.md`](time-based-analysis.md).
  - **Done (2026-08-23):** implemented in the service crate —
    `migration/src/m20260823_000014_time_based_analysis.rs`,
    `src/models/_entities/instance_segments.rs`, the pure `src/tba.rs`,
    `src/controllers/tba.rs`, routes, OpenAPI, and
    `tests/requests/tba.rs`.
  - **Acceptance (met):** a 100-day journey with 14 value-adding days
    reports 0.14 — **and the same journey with only its value-adding
    segments recorded still reports 0.14**, which is the regression test
    that stops the calendar-time denominator being "simplified" into a
    sum-of-recorded-activity denominator; the four category buckets sum
    to the lead time over a generated sweep; ratios stay in `[0, 1]`
    under overlapping and out-of-window segments; degenerate clocks
    return a stated null rather than a panic; every §5.1 invariant is a
    `422`; the new paths are `401` under `CARE_PATHWAY_REQUIRE_AUTH`.
    48/48 `--ignored` request tests green vs Postgres 18; 279 unit
    tests; clippy pedantic clean.
  - **Open (TBA-8 … TBA-11):** the front-end timeline wall and cohort
    view, cross-service journey stitching via the link-graph aggregator,
    and Prometheus gauges for cohort %VA / p90 lead time.

- [ ] **T-14 — Pathway analytics: what to borrow from process mining,
  treatment-pattern analysis, and exploratory EHR analysis.**
  *(Triaged 2026-09-03 against four open-source projects. Their code was
  read, not just their READMEs — the defects and the undocumented
  behaviour noted below came from the source. Citations in
  [§17.3](17-references.md).)*

  **The four sources.**

  | Source | What it is | State |
  |---|---|---|
  | [IPPA-py](https://github.com/PatientPathwayAnalysis/IPPA-py) | *Individualised Patient Pathway Analysis*: timed state machines (evaluation / treatment / related-illness) run over per-visit claims rows, cut into episodes where every machine is idle, then reduced to named **anchors** and a **delay decomposition** (waiting → evaluating → detecting → treating). TB in Taiwan's NHI; BMJ Glob Health 2020. | Python, Apache-2.0; dormant since 2019-03; no tests; a `'2st'` typo and a `zip(ser_t[:1], …)` slice bug are live in the anchor code. |
  | [process-mining-clinical-pathways](https://github.com/nhs-bnssg-analytics/process-mining-clinical-pathways) | NHS BNSSG single-study code: SUS spells, e-RS referrals, SWD contacts → one bupaR **event log** (`case = pseudonymised NHS number`, `activity = <setting>_<service>`) → variants, directly-follows process maps annotated with median days, heuristics + inductive miners, PM4Py alignments. Elective hip replacement. | R + T-SQL; **no licence**; dormant since 2021-06; not runnable as committed (blank connection strings, private tables, a syntax error at `2_…extracting_data.R:413`). |
  | [TreatmentPatterns](https://darwin-eu-dev.github.io/TreatmentPatterns/) | DARWIN EU / OHDSI R package: OMOP target + event cohorts → treatment **eras** → gap-collapsed, overlap-combined, truncated **pathway strings** with strata, an **attrition table**, **cell suppression**, sunburst / Sankey. | R, Apache-2.0; CRAN 3.1.2 (2026-02), one maintainer, active. Read from source: `minEraDuration` also filters *target* rows, and the unstratified export path skips `censorData()`. |
  | [ehrapy](https://github.com/theislab/ehrapy) | Theis lab scanpy-style EHR toolkit on `EHRData(AnnData)`: QC + missingness, imputation, **bias detection** (SMD, value-count ratios), Kaplan–Meier / Cox, clustering / pseudotime, and a CONSORT-style **`CohortTracker`**. Nature Medicine 2024. | Python, Apache-2.0; 0.15.0 (2026-07), active, heavy API churn (`ep.io` moved out in 0.14, AnnData compatibility dropped in 0.15, MedCAT removed in 0.12.1). |

  **Triage.** Every concept was placed in exactly one column. The
  refusals are recorded here so they are not re-litigated; the
  baseline they were judged against is the instance layer
  (`pathway_instances` / `instance_steps` / `instance_events` /
  `instance_team` / `instance_measures`), the TBA segment + clock
  model ([time-based-analysis.md](time-based-analysis.md)), the
  `continues_as` journey edge, and the five template insight lenses.

  | Concept | From | Decision |
  |---|---|---|
  | Event log (case / activity / timestamp / lifecycle / resource); source-prefixed activity labels; per-case lookup table | BNSSG | **Adopt** → T-14a (export codec) |
  | Directly-follows process map, nodes + edges annotated with case counts and median inter-activity days; state-transition Sankey (`sankey_diagram_time`) | BNSSG, ehrapy | **Adopt** → T-14b |
  | Trace variants + coverage Pareto; pathway strings built with named knobs (`minEraDuration`, `eraCollapseSize`, `combinationWindow`, `minPostCombinationDuration`, `filterTreatments`, `maxPathLength`); FRFS / LRFS overlap decomposition; canonical `a+b`; duration stats per line | BNSSG, TreatmentPatterns | **Adopt** → T-14c |
  | Named anchors → delay decomposition; anchored windows (`startAnchor` / `windowStart` / `endAnchor` / `windowEnd`) | IPPA, TreatmentPatterns | **Adopt** → T-14d |
  | Right-censoring: open journeys as censored; Kaplan–Meier + log-rank; explicit `CENSORED` / `LOST` outcomes | ehrapy, IPPA | **Adopt** → T-14e |
  | Rule-based cohort split (`check_rule(contains(activity))` → paired throughput / trace length / map); stratified Table 1 | BNSSG, ehrapy | **Adopt** → T-14f |
  | CONSORT-style cohort tracker (`label`, `operation`, `n`, `parent`; category sets frozen at step 0); attrition table with a row per transformation | ehrapy, TreatmentPatterns | **Adopt** → T-14g |
  | Missingness metrics (`missing_values_pct`, entropy of missingness), date-sanity codes (`bad_date` 1–5), the MCAR caveat | ehrapy, BNSSG | **Adopt** → T-14h |
  | Conformance checking (alignments against a model) | BNSSG | **Adapt** → T-14i: against the *template the instance was enrolled on*, never against a discovered model |
  | Retroactive timeout → idle state, stamped at the moment the timeout expired rather than when it was noticed | IPPA | **Adopt** → T-14j (stalled journeys) |
  | Cell suppression modes (`minCellCount` / `remove` / `mean`), `"<5"` rendering, the shareable-aggregate vs non-shareable-patient-level split | TreatmentPatterns | **Adopt with two changes** → T-14k: no `mean` mode, no censor-up-to-threshold, and secondary suppression of marginals |
  | Sunburst + Sankey of variants (with a `Stopped` terminal node), dotted chart, zoomable process-map SVG, attrition flowchart | TreatmentPatterns, BNSSG, ehrapy | **Adopt** → T-14l (front-end) |
  | Seeded synthetic pseudo-data release (IPPA-data); bundled reference datasets (`ed.dt.*`) | IPPA, ehrapy | **Adopt** → T-14m |
  | Sensitivity sweep over timeout parameters (`run_sens.py`) | IPPA | **Adopt as a rule, not an endpoint:** every parameterised derivation echoes its parameters in the response (T-14c, T-14j), so a caller can sweep. |
  | Heuristics / inductive / alpha miners; alignments against a discovered Petri net; model fitness / precision | BNSSG | **Refuse.** A discovered model is a notebook artefact, and BNSSG itself only ever checked conformance against the model it had just discovered. The service ships the event log (T-14a) and the DFG (T-14b) that those miners consume. |
  | Automatic episode segmentation (all state machines idle ⇒ cut) | IPPA | **Refuse.** Instances are explicit enrolments. Inferring episodes from events is the [TBA §3](time-based-analysis.md) refusal restated; it would also make coverage a lie. |
  | Facility capability inferred from observed behaviour (`'Anti-TB'` count > 0 ⇒ capable) | IPPA | **Refuse.** Not a registry question; what a provider can do is [organization](../../organization/)'s to assert, and IPPA's own paper says the inference under-counts. |
  | Trajectory clustering (`leiden`, `dpt`, NCP tensor decomposition), pseudotime, causal estimators (IPTW, g-computation) | ehrapy | **Refuse in the service.** Enabled by the per-journey feature export (T-14a) for a notebook; a service that clusters patients is making a claim it cannot audit. |
  | Cox proportional hazards with case-mix adjusted provider curves; bias / fairness slice by sensitive attributes (`detect_bias`) | ehrapy | **Open** → [OQ-7](16-open-questions.md): needs demographics the instance layer deliberately does not hold. |
  | Cost annotations on the process map (`custom(attribute="cost2", median)`) | BNSSG | **Open** → [OQ-8](16-open-questions.md): no cost field exists; adding one is a domain expansion, not an analytics feature. |
  | Imputation (`knn_impute`, `miss_forest_impute`, `locf_impute`) | ehrapy | **Refuse.** A missing clock stop or segment boundary is a finding ([TBA §6.6](time-based-analysis.md)), never a value to fill. ehrapy's own paper lists informative missingness as unaddressed; on a pathway clock, missingness is *always* informative. |
  | Results data model + federated upload (`ResultModelManager`, Strategus module) | TreatmentPatterns | **Refuse.** T-10's export contract covers it; a study-package uploader is not a registry's job. |
  | Per-resource throughput from the event log's `resource` column | BNSSG (where it is `NA` throughout) | **Refuse**, per [family TBA §7](../../agents/share/time-based-analysis.md): never a person metric. |

  **Suggested order.** T-14m (fixtures every other test needs) → T-14k
  (the suppression rule every aggregate inherits) → T-14b, T-14c, T-14d
  (the three derivations) → T-14e, T-14f, T-14g → T-14h, T-14i, T-14j →
  T-14a → T-14l. Each sub-task is one three-part PR (spec + code +
  tests); the pure parts go in `src/tba.rs` or a sibling
  `src/analytics.rs`, DB-free and property-tested, per
  [TBA §14](time-based-analysis.md).

  - [ ] **T-14a — Event-log and journey-feature export codecs.**
    Extends T-10 with two named export codecs, so a bupaR / PM4Py /
    ehrapy user can consume the instance layer without a bespoke query.
    - [ ] `event_log` (CSV + JSONL): one row per activity instance.
      `case_id` = the instance `pid` (never `subject_ref`; a patient's
      stitched journey across instances is [OQ-9](16-open-questions.md)),
      `activity` = `stage:<stage>` for segments, `step:<name>` for
      completed steps, `event:<kind>` for instance events, `lifecycle`
      = `start` / `complete` (segments carry both; steps and events are
      `complete`-only, as BNSSG's point-in-time rows were), `timestamp`,
      `category`, `waste`, `resource` = the team **role** of
      `actor_ref` (never the URN), `location_ref`; case attributes
      `pathway_pid`, `care_setting`, `urgency`, `status`, `outcome`.
    - [ ] `journey_features` (CSV + JSONL): one row per instance with
      LT, VT, PT, %A, %VA, coverage, #HO, per-stage durations, gap
      count, anchors + delays (T-14d), variant string (T-14c),
      conformance (T-14i), outcome, and a `censored` flag. This is the
      per-journey feature vector ehrapy's longitudinal tutorial builds
      by hand — the input to a notebook's clustering, not the service's.
    - [ ] Both are **patient-level ⇒ non-shareable**
      (TreatmentPatterns' `exportPatientLevel` split): `masking_profile`
      masked by default, `full` gated, every export audited, per T-10.
      Suppression does **not** apply to rows (T-14k applies to
      aggregates); gating does.
    - **Acceptance:** exporting a seeded cohort (T-14m) and re-deriving
      the DFG from the file equals the T-14b endpoint's DFG; a test
      asserts no codec output ever contains a `subject_ref` or a person
      URN; the column set is pinned by a snapshot so a bupaR
      `eventlog(case_id, activity_id, lifecycle_id, timestamp,
      resource_id)` mapping does not drift.
  - [ ] **T-14b — Directly-follows process map per pathway cohort.**
    `GET /api/care-pathways/{pathway}/process-map?level=stage|step`
    (+ the T-14f cohort filters): nodes (activity, instance count,
    occurrence count, median duration where the activity has one) and
    edges (from, to, instance count, median + p90 gap in days) derived
    on read — stage level from segments in time order, step level from
    completed steps in `done_on` order — with explicit `start` / `end`
    pseudo-nodes so entry and exit variety is visible. Self-loops are
    kept: a return to a stage is a finding. Level `step` states its own
    caveat in the response: `done_on` is a date, so a same-day pair is
    a 0-day edge.
    - **Acceptance:** pure `process_map` tests — edge counts sum to the
      transition count, median gaps match hand-computed values, a
      cohort of one variant yields a chain; a request test on the
      seeded cohort; nodes and edges below the floor are withheld with
      a reason, never zeroed (T-14k).
  - [ ] **T-14c — Journey variants (pathway strings).**
    `GET /api/care-pathways/{pathway}/variants`: per instance, the
    ordered stage sequence from segments, transformed by **named,
    defaulted, echoed** parameters: `min_segment_days` (shorter
    segments dropped), `collapse_gap_days` (same stage separated by ≤ N
    days ⇒ one step), `combination_window_days` (overlap ≥ N days ⇒ a
    canonical alphabetical `a+b` step; shorter overlap ⇒ a handoff;
    FRFS / LRFS decomposition into non-overlapping intervals; stubs
    shorter than `min_post_combination_days` dropped; iterate until no
    overlap remains, so three-way overlap converges to `a+b+c`),
    `filter` = `first` | `changes` | `all`, `max_path_length`. Output:
    variant string (`referral-diagnostics-treatment+follow_up-…`),
    frequency, share, cumulative coverage (the Pareto), and per-position
    ("line") duration quantiles with `overall` as a pseudo-line. Nothing
    stored; an attrition row per transformation (T-14g shape).
    - **Acceptance:** pure tests reproduce TreatmentPatterns' documented
      cases — two overlapping eras become three intervals under FRFS and
      under LRFS; `b+a` ≡ `a+b`; a stub below
      `min_post_combination_days` disappears; `changes` collapses
      `a-a-b` to `a-b` while `all` keeps it; coverage sums to 1 over the
      unsuppressed variants and the suppressed count is disclosed.
  - [ ] **T-14d — Stage anchors, delay decomposition, and anchored
    standards.** Per instance: `anchors` = first `started_at` of each
    stage in `STAGES` (`null` if never reached), `delays` = adjacent
    differences in stage order (IPPA's waiting → evaluating → detecting
    → treating, in our vocabulary). The standards catalogue gains
    `from_anchor` / `to_anchor` (default clock start → clock stop, i.e.
    today's behaviour), so `cancer_fds_28` can score referral →
    `diagnostics` rather than the whole clock. Cohort compliance uses
    the anchored interval when both anchors are present and reports
    `unreached` as a **third verdict** — never compliant, never a
    breach, disclosed as a count. A standard whose anchor the `STAGES`
    vocabulary cannot express stays whole-clock with an `anchor_note`
    saying so, rather than approximating. Resolves the "segment
    templates" lean in [TBA §17](time-based-analysis.md) only as far as
    anchors go; per-template target durations remain that open question.
    - **Acceptance:** a journey whose referral → diagnostics interval is
      20 days inside a 100-day clock is compliant on a 28-day
      referral-to-diagnostics standard and unaffected on `rtt_18_weeks`;
      an instance that never reaches `diagnostics` is `unreached`,
      excluded from numerator and denominator, and counted; the default
      anchors reproduce today's figures exactly (regression pin).
  - [ ] **T-14e — Censoring-aware cohort statistics.** Today
    `?status=all` mixes closed lead times with open instances' running
    lead time, which understates the eventual distribution (the
    survivorship error ehrapy's MIMIC tutorial exists to teach). Add a
    Kaplan–Meier estimate of time-to-close and time-to-anchor (T-14d)
    treating open instances as right-censored at `as_of`, with median
    and p90 read off the curve where it reaches them (else `null` with
    reason `curve_did_not_reach`), the numbers of events and censored
    instances, and a log-rank test between the two sides of a T-14f
    split. Whether `discontinued` closure is an event or a censor is a
    parameter, default `event`, echoed. Nearest-rank percentiles stay as
    they are; KM is an additional, labelled block, not a replacement.
    - **Acceptance:** KM on a fully closed cohort equals the empirical
      distribution; an all-open cohort returns `null` with the reason;
      log-rank on two identical cohorts gives p ≈ 1; property test: the
      survival function is non-increasing in `[0, 1]`.
  - [ ] **T-14f — Rule-based cohort splits and the paired comparison.**
    Every cohort endpoint (`time-analysis`, `constraints`, `variants`,
    `process-map`, `data-quality`) accepts `contains=` / `excludes=`
    with `stage:<s>`, `step:<name>`, `event:<kind>`, `waste:<w>`,
    `outcome:<o>`, `setting:<s>`, `urgency:<u>`, and `compare=true`
    returns the same figures for the complement side by side — the
    stratified Table 1 (n, lead-time percentiles, %VA, coverage, #HO,
    standard compliance) BNSSG built by hand with `check_rule` +
    `group_by` and `tableone`.
    - **Acceptance:** split and complement sizes sum to the unsplit
      cohort; identical filters on both sides give identical figures;
      when one side is below the floor, the other side's figures are
      also withheld wherever they could be differenced against the
      unsplit total (T-14k).
  - [ ] **T-14g — Cohort attrition record (CONSORT).** Every cohort
    response carries `attrition`: ordered steps `{label, operation,
    instances, parent}` from "enrolled on pathway" through the status
    filter, window, rule filters, degenerate-clock exclusion, coverage
    floor, and suppression, so the denominator is explained inside the
    response rather than in a log. `parent` gives the branching a
    `compare` needs. Category sets used by any composition table are
    frozen at step 0 (ehrapy's rule), so a later step cannot invent a
    stratum — or silently lose one.
    - **Acceptance:** the last step's `instances` equals the analysed
      n; a test enumerates every exclusion reason in the code and
      asserts each has a step; a step that excluded nobody still
      appears (an empty cell is a finding).
  - [ ] **T-14h — Journey data-quality and missingness report.**
    `GET /api/care-pathways/{pathway}/data-quality`: per cohort, the
    share of instances with no segments, an open segment past closure,
    a terminal status with no clock stop, `done_on` before
    `enrolled_on`, out-of-order step completion, segments clipped by the
    clock, coverage below the floor, and anchors unreached — each a
    code in a closed vocabulary (BNSSG's `bad_date` 1–5, generalised);
    plus per-stage and per-field missingness percentage and entropy of
    missingness across instances. The report is the finding; it never
    imputes (see the triage table).
    - **Acceptance:** a seeded cohort with injected defects (T-14m)
      reports each code exactly once per defect; a clean cohort reports
      every code at zero, rows present.
  - [ ] **T-14i — Conformance to the enrolled template.** Per
    instance, the steps copied at enrolment (`instance_steps.position`)
    against their completion order (`done_on`): skipped steps, adjacent
    declared pairs completed out of order, steps completed after
    closure, and `escalation` events; a labelled ratio
    `pairs_in_order / declared_pairs` shipped with both numbers; cohort
    share fully conformant. Against the template only — never a
    discovered model — and with no penalty for extra events: a journey
    may need more than its template foresaw.
    - **Acceptance:** completing steps in template order scores 1.0
      with zero inversions; reverse order scores 0; a skipped step is
      reported as skipped, not as an inversion; an instance with one
      declared step reports `null` (no pairs) with the reason.
  - [ ] **T-14j — Stalled journeys (aging WIP).**
    `GET /api/instances/stalled?idle_days=N` (default 60, echoed):
    open instances whose last recorded activity — latest of segment
    start / end, step `done_on`, event `occurred_at`, review — is older
    than N days, sorted by idle time, each row naming its last-activity
    source. Complements `overdue-reviews` (a due date) with an
    observed-silence test. The timeout is retroactive, as IPPA's
    `Process.time_out` is: idle-since is the last activity time, not
    the time the silence was noticed. Never grouped by actor.
    - **Acceptance:** an instance whose last event was 61 days ago is
      listed at `idle_days=60` and not at 90; an instance with an open
      segment started 5 days ago is not listed; a closed instance is
      never listed.
  - [ ] **T-14k — Disclosure control: modes and marginals.** Generalise
    the TBA-10 floor to every aggregate above: `min_cell_count`
    (deployment-configurable upward only, per [TBA §17](time-based-analysis.md)),
    modes `withhold` (default: `null` + reason) and `remove`. Two of
    TreatmentPatterns' three modes are deliberately **not** adopted:
    `mean` substitutes a made-up count, and `minCellCount` reports a
    suppressed cell *as* the threshold, which reads as a count. And one
    rule is added that TreatmentPatterns leaves to the caller: when a
    cell in a stratified table is withheld, enough sibling cells are
    also withheld that the value cannot be recovered by differencing
    against a visible total (secondary suppression).
    - **Acceptance:** property test over generated stratified outputs —
      no withheld cell is recoverable as `total − Σ visible`; `remove`
      and `withhold` never disagree on *which* cells are small; the
      T-14a codecs are exempt from suppression and gated instead.
  - [ ] **T-14l — Front-end analytics views** in
    `care-pathway-front-end-with-svelte`. `/time` gains the process map
    (an in-house layered SVG layout — node size = instances, edge label
    = median days — rather than a new graph dependency), the variants
    sunburst and Sankey with a `Stopped` terminal node, a dotted chart
    (instances × time, coloured by stage), the attrition flowchart, the
    `compare` two-column view, and the stalled list; filters bound to
    the T-14f parameters; suppressed cells rendered as "withheld
    (n < 5)", never blank. Theme-aware, i18n keys in every locale file.
    - **Acceptance:** vitest units for the sunburst / Sankey transforms
      from a variants payload and the DFG layout; Playwright smoke with
      the API stubbed; a withheld cell is visibly labelled.
  - [ ] **T-14m — Seeded synthetic journey cohorts.** A generator in
    `src/data` plus a loco task (`journeys:seed` with `pathway`, `n`,
    `seed`, `open_share`, `defects`): deterministic instances with
    segments, steps, and events across `STAGES`, configurable
    stage-duration distributions, gap and overlap rates, defect
    injection for every T-14h code, and a censoring share. `subject_ref`
    values come from a fixed, obviously fictional UUID namespace. Never
    real data, never derived from real data — the family's synthetic-
    only rule, and IPPA-data's own disclaimer ("no responsibility to
    answer any epidemiological question") is the reason to say so in
    the docs. Used by every T-14 test and by the repo demo seed (EX-4).
    - **Acceptance:** the same seed produces byte-identical output;
      generated cohorts satisfy every §5.1 invariant unless a defect
      was requested; the README states the data is synthetic.
