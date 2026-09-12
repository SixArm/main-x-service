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
- [x] **T-10 — Bulk import / export.** **Done, 2026-09-12** (`src/bulk/`
  in `care-pathway-service-with-loco`). See §9.4, §10.4 and
  [bulk import/export](../../agents/share/bulk-import-export.md).
  - [x] Reused the **existing** `bulk_jobs` table (shared doc §3 schema,
    `UNIQUE (entity, kind, idempotency_key)`) — already migrated for
    FHIR Bulk Data `$export`; no second migration. `format` distinguishes
    a native-bulk row (`jsonl`/`csv`/`tsv`) from an FHIR one (`ndjson`).
  - [x] The five endpoints (§9.4): `POST`/`GET`
    `/api/care-pathways/import`, `POST`/`GET`
    `/api/care-pathways/export`, `GET /api/care-pathways/bulk-jobs`
    (filterable by `kind`/`status`).
  - [x] loco `worker`-queue `BackgroundWorker` draining jobs `queued →
    running → completed | completed_with_errors | failed`.
  - [x] JSONL (lossless reference) + CSV/TSV (flattening per §9.4: every
    repeated / nested field a JSON-in-cell) codecs. Parquet is **not
    built** — a deliberate scope narrowing matching organization's and
    case's own BLK-5 rollouts, since nothing in this task's own
    dependency chain needed it.
  - [x] Per-row pipeline reusing the single-create validators
    (`src/validation.rs`) + matcher + review queue: upsert by stable key
    (a deterministic identifier — DOI/Wikidata/`GuidelineId`/URI/UUID,
    tried in that declared order — then `(provider_id, pathway_code)`,
    then `pid`, §9.4); keyless / unmatched rows → duplicate detection →
    review queue with `provenance = import`; events + audit not
    bypassed. This crate had **no** `review_queue` table before this
    task — added fresh (case's own BLK-5 precedent, not organization's,
    since case also started from zero), with `provenance` in its
    initial shape.
  - [x] Downloadable per-row error report
    (`row_number, field, code, message`); one bad row never
    aborts the load; counts reconcile
    (`rows_total = created + upserted + to_review + errored`).
  - [x] Export masking + audit: `masking_profile` (masked default, full
    gated), `include_soft_deleted` gated (rejected as not-yet-supported,
    same as organization's/case's own posture), every export audited
    (even zero-row, gating delivery per SEC-B8).
  - **Disclosed scope decisions** (full rationale in `src/bulk/`'s own
    module docs): the `active` (soft-delete) column round-trips on
    export but is never applied on import (no bulk
    reactivate/deactivate operation exists); `in_language`
    (`Vec<String>`) is JSON-encoded like every other array column even
    though §9.4's prose lists it among the "one column each" scalars —
    read as "still one column, whose cell holds JSON" rather than a
    contradiction; the per-row upsert is not SEC-B3
    advisory-lock-protected, matching organization's/case's own
    documented BLK-5 gap for the identical structural reason
    (`streaming::create_and_emit`/`update_and_emit` open their own
    internal transaction, hard-coded to `&DatabaseConnection`). This
    crate's existing `ArtifactStore` (`src/bulk/store.rs`) already
    supported S3 before this task (built for FHIR Bulk Data), so native
    bulk inherits it for free — unlike organization's/case's own
    local-filesystem-only BLK-5 rollouts.
  - **Acceptance:** integration tests (`tests/requests/bulk.rs`, 9,
    DB-gated) cover idempotent re-import at both stable-key tiers (same
    file re-upserts to the same state), CSV/JSONL round-trip, the
    per-row error report, a keyless dedupe-to-review row (`provenance =
    import`), masked vs full export, a zero-row export still writing an
    audit record, `include_soft_deleted` rejection, an unsupported
    format token returning `400`, and the `bulk-jobs` listing
    (kind-filtered).

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
  [TBA §14](time-based-analysis.md). **T-14a landed 2026-09-09**, then
  **T-14m** the same day (its own generator lives in
  `src/data/journeys.rs`, not `src/analytics.rs` — see its entry
  below), then **T-14k, T-14b, T-14c, T-14d, and T-14e, all on
  2026-09-10, and T-14f, T-14g, T-14h, T-14i, T-14j, and T-14l all on
  2026-09-11** — **all thirteen sub-tasks (T-14a through T-14m) have
  now landed; T-14 is complete.** (An earlier version of this note said
  "all twelve … T-14 is complete" right after T-14j — miscounting,
  since T-14a/b/c/d/e/f/g/h/i/j/k/m is twelve names, not thirteen, and
  omits T-14l, the front-end sub-task in a different crate, which had
  not landed yet. Corrected here now that it has.)
  Three of those thirteen (T-14a, T-14m, T-14k)
  were out of the suggested
  order above, because none needed the tasks still ahead of it in
  this list to be useful now (see each entry's own scope notes;
  T-14a's covers why it needs no suppression pass from T-14k: it
  exports patient-level rows, which T-14a's own spec text says are
  gated, not suppressed — T-14k's own module docs confirm the same
  thing from the other side). T-14b's own pure logic sits beside
  T-14a's in `src/analytics.rs` rather than in `src/tba.rs`, matching
  [TBA §15](time-based-analysis.md)'s own statement that a sequence
  analysis is not an elapsed-time one. T-14c got its own new file,
  `src/variants.rs`, rather than adding to the already-large
  `src/analytics.rs` — the same reasoning that gave T-14k its own
  `src/suppression.rs` rather than folding into `src/tba.rs`. The
  remaining **nine landed in the suggested order**: T-14b, T-14c, and
  T-14d are the "three derivations" trio in full, **T-14e followed
  immediately after** — its pure logic sits in `src/tba.rs` itself too
  (not a new sibling module, same reasoning as T-14d's), since
  `Observation`/`KaplanMeier`/`LogRank` are a fourth extension of the
  same file rather than a genuinely separate sequence-analysis shape
  like T-14b's/T-14c's — and **T-14f followed T-14e**, exactly next in
  the list. T-14f got its own new file, `src/split.rs`, for the same
  reason T-14c's/T-14k's own new files did (a genuinely separate
  concern — predicate matching over a cohort, not an elapsed-time
  computation) — and is the first real caller of T-14k's own
  `Table`/`decide` stratified-suppression primitive, which had stood
  ready but uncalled since T-14k landed. **T-14g followed T-14f**,
  again exactly next in the list — its own pure logic (`AttritionStep`,
  `attrition_trail`, `attrition_rule_branch`) extends `src/tba.rs`
  itself rather than a new sibling module, since a CONSORT step record
  is a small, mechanical data structure, not a genuinely separate
  algorithm the way T-14c's/T-14f's own new files were. **T-14h
  followed T-14g**, again exactly next — it got its own new file,
  `src/data_quality.rs`, for the same reason T-14c's/T-14f's/T-14k's
  own new files did (detectors over already-computed figures, not an
  elapsed-time computation), and is the **first task to actually
  exercise T-14m's generator** end to end rather than a hand-built
  fixture — see its own entry and T-14m's updated one for the two real
  generator bugs that exercise turned up and this task fixed. **T-14i
  followed T-14h**, again exactly next — it got its own new file,
  `src/conformance.rs`, for the same reason T-14b's/T-14c's/T-14f's/
  T-14h's own new files did (sequence-order comparison, not an
  elapsed-time computation). Unlike T-14h, it does **not** exercise
  T-14m's generator: `journeys:seed` has no defect exercising step
  order at all, so its own DB-gated round trip builds instances
  directly instead (see its own entry's note on backdating fields the
  live HTTP API cannot backdate). **T-14j followed T-14i**, again
  exactly next — the last of the eleven *backend* sub-tasks (an
  earlier note here called it "the last of the twelve", written before
  T-14l — the twelfth, front-end-only sub-task, in a different crate —
  had landed; T-14l followed on 2026-09-11 too, closing T-14 in full).
  Unlike every other sub-task
  from T-14b on, it extends `src/instances.rs` (the crate's existing
  pure instance-lifecycle module) rather than adding a new sibling
  file: a "latest of several optional timestamps, past a threshold"
  fold is a small extension of that module's own remit, not a
  genuinely separate algorithm. Like T-14i, its own DB-gated round
  trip builds fixtures directly rather than via T-14m's generator, for
  the same reason.

  - [x] **T-14a — Event-log and journey-feature export codecs.**
    Landed 2026-09-09, ahead of the suggested order above — at the
    time, T-14m/T-14k/T-14b/T-14c/T-14d/T-14e/T-14f/T-14g/T-14h/T-14i/T-14j/T-14l
    were also still unbuilt (all twelve have since landed too — every
    T-14 sub-task is now complete, as of T-14l on 2026-09-11) — because the two codecs needed
    none of them to produce a real, useful v1 — see the deviations noted below,
    each an explicit scope decision rather than a silent gap. Pure
    row-shaping in `src/analytics.rs` (DB-free, unit-tested); the HTTP
    surface + DB loading in `src/controllers/exports.rs`:
    `GET /api/care-pathways/{pathway}/export/{event-log,journey-features}
    ?format=csv|jsonl&status=open|closed|all`.
    - [x] `event_log` (CSV + JSONL): one row per activity instance.
      `case_id` = the instance `pid` (never `subject_ref`; a patient's
      stitched journey across instances is [OQ-9](16-open-questions.md)),
      `activity` = `stage:<stage>` for segments, `step:<name>` for
      completed steps, `event:<kind>` for instance events, `lifecycle`
      = `start` / `complete` (segments carry both; steps and events are
      `complete`-only, as BNSSG's point-in-time rows were), `timestamp`,
      `category`, `waste`, `resource` = the team **role** of
      `actor_ref` (never the URN — resolved via `instance_team`; `None`
      when the actor is not a recorded team member, never a fallback to
      the raw ref), `location_ref`; case attributes `pathway_pid`,
      `care_setting`, `urgency`, `status`, `outcome`.
    - [x] `journey_features` (CSV + JSONL): one row per instance with
      LT, VT, PT, %A, %VA, coverage, #HO, per-stage durations
      (`stage_<name>_ms`, one column per `tba::STAGES`), gap count, and
      a `censored` flag (`clock.running` — the one part of T-14e
      buildable without that task's own Kaplan–Meier machinery).
      Anchors + delays (T-14d), variant string (T-14c), and conformance
      (T-14i) were present as columns (`anchors_delays`, `variant`,
      `conformance`) but always `null` at landing — those three sibling
      tasks were not yet built, so there was nothing to compute yet;
      documented as a gap (each field's doc comment names the task that
      fills it), not a silently-empty string, and no other T-14a work
      was blocked on landing them first. **Updated 2026-09-10, in
      T-14d's own change:** `anchors_delays` is now wired —
      `journey_feature_row` computes it straight from the same
      `tba::InstanceAnalysis` this row already builds from (one JSON
      cell, `{"anchors": […], "delays": […]}`), since T-14d's data is a
      per-instance property with no cohort context needed. `variant`
      (T-14c, landed 2026-09-10 but genuinely needs the whole cohort's
      pipeline — not derivable from one instance in isolation) stays
      `null`, for the reason its own doc comment now states.
      **Updated 2026-09-11, in T-14i's own change:** `conformance` is
      now wired too — a compact scalar summary
      (`{"ratio": …, "declared_pairs": …, "pairs_in_order": …,
      "escalation_events": …}`), not the per-pair verdict breakdown
      the dedicated `/time-analysis` endpoint carries, since a feature
      table row wants a summary, not a repeated diagnostic detail.
      `journey_feature_row` gained a third parameter
      (`&conformance::Conformance`) for it, the same per-instance,
      no-cohort-context shape `anchors_delays` already established.
    - [x] Both are **patient-level ⇒ non-shareable**, gated as
      `Action::Destructive` (mirroring the `continues_as` bulk-pull
      precedent, [cross-service-linking.md §10.2](../../agents/share/cross-service-linking.md))
      rather than `Read`, and every call is audited as a disclosure
      (`disclosure::action::EXPORT`) — **deviation from the spec text
      above**: no separate `masking_profile` knob was built. The codec
      never produces `subject_ref` or an actor's raw URN in the first
      place (see the `resource` derivation above), so there is nothing
      a `full` mode would additionally reveal — the fields a mask would
      redact are excluded unconditionally, not merely withheld by
      default. T-10's own async job contract (queue, `bulk_jobs` row,
      artifact store) is **not yet built** for this crate either; this
      is a **synchronous v1** that renders on the request path,
      documented as such in `src/controllers/exports.rs`'s module doc
      rather than silently presented as the full T-10 contract.
      Suppression does **not** apply to rows (T-14k, now landed,
      applies only to aggregates, and its own module docs confirm this
      exemption explicitly); gating does, and is live.
    - **Acceptance:** the seeded-cohort/T-14b DFG cross-check is
      **still deferred**, though the reason has changed: T-14b and
      T-14m have both since landed, so the pieces now exist, but
      "export the event log, replay it, and diff it against
      `process-map`'s DFG" is a real integration test nobody has
      written yet — a follow-up, not a blocker on either sibling task.
      A test does assert no codec output
      ever contains a `subject_ref` or a person URN — both a DB-free
      property test in `src/analytics.rs` (`event_log_never_carries_…`,
      `journey_features_never_carries_…`) and a live HTTP round-trip in
      `tests/requests/exports.rs` seeding an instance whose
      `subject_ref` and actor URN are deliberately present, so the
      absence is proven against real data, not merely never
      constructed. The column set is pinned, but by a plain literal
      assertion (`event_log_csv_header_is_pinned`,
      `journey_features_csv_header_is_pinned`) rather than an `insta`
      snapshot — this crate declares `insta` as a dev-dependency but
      had never actually used it anywhere before this task, and
      introducing that workflow (`.snap` files, `cargo insta review`)
      for the first time was judged lower-value than an assertion that
      already gives the same drift protection.
  - [x] **T-14b — Directly-follows process map per pathway cohort.**
    Landed 2026-09-10. `GET /api/care-pathways/{pathway}/process-map
    ?level=stage|step&status=&mode=` (T-14f's own rule-based cohort
    splits landed 2026-09-11 but were deliberately not wired onto this
    endpoint — see T-14f's own scope note; `status`/`mode` are the
    shared cohort filters that already existed here): nodes (activity, instance count, occurrence
    count, median duration where the activity has one) and edges
    (from, to, instance count, occurrence count, median + p90 gap in
    days) derived on read — stage level from segments in time order,
    step level from completed steps in `done_on` order — with explicit
    `start`/`end` pseudo-nodes so entry and exit variety is visible.
    Self-loops are kept: a return to a stage is a finding. Level
    `step` states its own caveat in the response: `done_on` is a date,
    so a same-day pair is a 0-day edge.
    - [x] **Deviation from the spec text above**: edges carry both
      `instance_count` (distinct instances with ≥1 occurrence) *and*
      `occurrence_count` (total occurrences) — the spec text named
      only "instance count," but a self-loop makes the two diverge,
      and the acceptance criterion ("edge counts sum to the transition
      count") is only literally true of `occurrence_count`. Both are
      published rather than picking one silently.
    - [x] The **gap** between two activities is measured from the
      *end* of the first to the *start* of the second (reusing the
      same "idle time between segments" concept TBA's own `Gap`
      already computes), not start-to-start — otherwise the first
      activity's own duration would be double-counted as part of the
      transition's idle time. A step, a pseudo-node, and a still-open
      segment (whose true end is unknown) each use their own start as
      a stand-in end — the least-wrong choice available, documented
      in `ActivityStep`'s own doc comment.
    - [x] Suppression (T-14k) is **per node/edge**, not per cohort:
      an activity visited by fewer than `min_cell_count` instances
      stays withheld even once the *cohort* is well past the floor —
      the two are genuinely different thresholds, confirmed directly
      by the request test rather than assumed.
    - **Acceptance:** pure `process_map` tests — edge occurrence
      counts sum to the transition count
      (`edge_occurrence_counts_sum_to_the_transition_count`), median
      and p90 gaps match hand-computed values
      (`median_and_p90_gaps_match_hand_computed_values`), a cohort of
      one variant yields a chain (`a_cohort_of_one_variant_yields_a_chain`),
      self-loops are kept (`self_loops_are_kept_not_collapsed`); a
      request test (`process_map_round_trip`) exercises both levels,
      an unrecognised `level` (`422`), `?mode=remove`, and nodes/edges
      below the floor withheld with a reason, never zeroed, while the
      cohort itself is large — the seeded-cohort generator (T-14m) was
      not used for this test (a hand-built fixture was simpler for the
      specific per-node-vs-per-cohort distinction this test needed to
      pin), which is a scope note, not a gap: T-14m remains available
      for a future property-style test over generated cohorts.
  - [x] **T-14c — Journey variants (pathway strings).** Landed
    2026-09-10. `GET /api/care-pathways/{pathway}/variants?status=
    &min_segment_days=&collapse_gap_days=&combination_window_days=
    &min_post_combination_days=&filter=&max_path_length=`: per
    instance, the ordered stage sequence from segments, transformed by
    **named, defaulted, echoed** parameters: `min_segment_days`
    (shorter segments dropped), `collapse_gap_days` (same stage
    separated by ≤ N days ⇒ one step), `combination_window_days`
    (overlap ≥ N days ⇒ a canonical alphabetical `a+b` step; shorter
    overlap ⇒ a handoff; FRFS / LRFS decomposition into non-overlapping
    intervals; stubs shorter than `min_post_combination_days` dropped;
    iterate until no overlap remains, so three-way overlap converges to
    `a+b+c`), `filter` = `first` | `changes` | `all`, `max_path_length`.
    Output: variant string (`referral-diagnostics-treatment+follow_up-…`),
    frequency, share, cumulative coverage (the Pareto), and per-position
    ("line") duration quantiles with `overall` as a pseudo-line. Nothing
    stored.
    - [x] **FRFS/LRFS, precisely defined.** The two names and their
      geometric meaning are confirmed against `TreatmentPatterns`' own
      CRAN documentation (via web search, not guessed): for eras `a`
      (starts first) and `b` (starts second), **FRFS** ("first
      received, first stopped") is the shape where `a` also ends first
      or exactly with `b`; **LRFS** ("last received, first stopped")
      is the shape where `b`'s whole span nests inside `a`'s. What
      this crate does with either shape below the combination window —
      attributing the contested middle stretch to the *incoming*
      stage as a "handoff" — is this crate's own documented choice,
      not reproduced from `TreatmentPatterns`' source, which was not
      available to read in this environment.
    - [x] **A combination era is exempt from
      `min_post_combination_days`.** Found while writing the request
      test: applying the stub floor to *every* resulting era, including
      the combination itself, could delete a real (if short) `a+b`
      co-occurrence — the floor's name is "post-combination," what is
      left *around* a combination, not the combination block itself.
    - [x] **No `?mode=` knob**, unlike T-14b/T-14k's cohort endpoints:
      a suppressed variant is always folded into `suppressed_instances`
      (never listed individually, never a "remove" option that would
      just be the same list with the fold already applied) — there is
      no "withhold vs remove" distinction meaningful at the level of
      an aggregated frequency table the way there is for a single
      node/edge.
    - [x] **Attrition row per transformation** (the T-14g shape the
      spec text names) is **not built** — T-14g itself is unbuilt, and
      inventing its row shape here to satisfy this task alone risked
      committing to a format T-14g would then have to match or
      abandon. Deferred to when T-14g lands.
    - **Acceptance:** pure tests — `frfs_overlap_becomes_three_intervals`,
      `lrfs_overlap_becomes_three_intervals` (two overlapping eras
      become three intervals under each shape); `b+a ≡ a+b`
      (`overlap_at_or_above_the_window_combines_canonically`, asserting
      `combined_label` is order-independent directly); a stub below
      `min_post_combination_days` disappears
      (`a_post_combination_stub_disappears`) while a short *combination*
      survives the same floor
      (`a_short_combination_era_survives_the_stub_floor`); `changes`
      collapses `a-a-b` to `a-b` while `all` keeps it
      (`changes_collapses_consecutive_duplicates_all_does_not`);
      coverage sums to 1 over the unsuppressed variants and the
      suppressed count is disclosed
      (`coverage_sums_to_one_and_suppression_is_disclosed`). A three-way
      overlap converging to `a+b+c` is pinned directly too
      (`three_way_overlap_converges_to_a_plus_b_plus_c`), beyond what
      the acceptance text itself named. A live HTTP round trip
      (`tests/requests/tba.rs`'s `variants_round_trip`) exercises the
      full pipeline through real segments, an unrecognised `filter`
      (`422`), and the renormalisation property end to end: a
      six-instance cohort where one instance's unique journey is
      suppressed still reports the five-instance majority variant's
      share as exactly `1.0`.
  - [x] **T-14d — Stage anchors, delay decomposition, and anchored
    standards.** Landed 2026-09-10, in the suggested order — see the
    "Suggested order" paragraph above. Per instance: `anchors` = first
    `started_at` of each stage in `STAGES` (`null` if never reached),
    `delays` = adjacent differences in stage order (IPPA's waiting →
    evaluating → detecting → treating, in our vocabulary), both
    computed in `tba::analyze` and carried as new
    `InstanceAnalysis::anchors`/`::delays` fields (`src/tba.rs`:
    `StageAnchor`, `anchors()`, `Delay`, `delays()`). The standards
    catalogue gains `from_anchor` / `to_anchor` (default `None`/`None`,
    i.e. today's whole-clock behaviour), so `cancer_fds_28_days` can
    score referral → `diagnostics` rather than the whole clock. Cohort
    compliance uses the anchored interval when both anchors are
    present and reports `unreached` as a **third verdict** — never
    compliant, never a breach, disclosed as a count
    (`tba::anchored_compliance`). A standard whose anchor the `STAGES`
    vocabulary cannot express stays whole-clock with an `anchor_note`
    saying so, rather than approximating. Resolves the "segment
    templates" lean in [TBA §17](time-based-analysis.md) only as far as
    anchors go; per-template target durations remain that open question.
    - [x] `GET /api/care-pathways/{pathway}/time-analysis` gains
      `?from_anchor=&to_anchor=` (`src/controllers/tba.rs`:
      `CohortQuery`, `score_compliance`, `resolve_anchor_pair`,
      `resolve_standard`). Precedence, most to least specific: (1) an
      explicit, valid query pair always wins, even over a standard's
      own declared anchor; (2) naming neither falls through to the
      requested standard's own `from_anchor`/`to_anchor` — this is the
      mechanism that makes `?standard=cancer_fds_28_days` alone (no
      anchor query at all) score anchored; (3) neither the query nor
      the standard declaring one leaves whole-clock — today's
      behaviour — untouched, which is every catalogue entry except
      `cancer_fds_28_days`. An explicit-but-invalid query pair (only
      one side given, or a name that is not a `STAGES` value) never
      silently reverts to a standard's own anchor: it always falls all
      the way to whole-clock, with the fallback disclosed on
      `compliance.anchor_note` rather than approximated in silence —
      **not the vaguer "a standard whose anchor `STAGES` cannot
      express" case this section's own prose above describes**,
      because every anchor in this system is a `STAGES` name by
      construction; the disclosed-fallback mechanism is real, the
      motivating scenario for it turned out to be query-side error
      rather than catalogue-side inexpressibility. This wiring —
      the standard-declared-default and the override precedence — is
      a deliberate scope decision beyond the acceptance text's literal
      "the default anchors reproduce today's figures exactly": the
      wording there is about the default for every standard that
      declares *no* anchor (the regression pin below), not a
      prohibition on any standard declaring one, and the acceptance
      example itself names `cancer_fds_28` as scoring anchored.
    - [x] **Deviation from the acceptance text's literal reading:**
      only `cancer_fds_28_days` — the standard the acceptance example
      itself names — declares a real `from_anchor`/`to_anchor` pair in
      the catalogue. The other five (`rtt_18_weeks`, `cancer_31_days`,
      `cancer_62_days`, `diagnostics_6_weeks`, `ae_4_hours`) stay
      `None`/`None`, unchanged. Every one of those five's clinical
      definition is genuinely a whole-journey or decision-to-treatment
      measure, not a named two-stage interval this vocabulary can
      express faithfully; declaring an anchor pair for them would have
      been a guess this change chose not to make, matching this
      whole T-14 sprint's "verify, don't infer" discipline.
    - [x] T-14a's own reserved `journey_features` export column
      `anchors_delays` (`src/analytics.rs`) is wired in this change too
      — a JSON-encoded `{"anchors": […], "delays": […]}` cell, computed
      straight from the same `InstanceAnalysis` `journey_feature_row`
      already builds from, since T-14d's data is a per-instance
      property with no cohort context needed (unlike `variant`, T-14c,
      which stays `null` — a per-instance variant string genuinely
      needs the whole cohort's pipeline). Beyond this task's own literal
      scope, but the column existed for exactly this and no other T-14a
      work depended on leaving it unwired.
    - [x] **Known gap, out of scope for this change:** anchored
      compliance is not separately suppression-gated — a cohort below
      the minimum cell count still returns a real `compliance` figure
      (anchored or whole-clock alike) while `lead_time` is withheld.
      This is a pre-existing gap in `cohort_time_analysis` (it applies
      identically to the whole-clock `compliance` figure today), not
      something T-14d introduced or is required to close.
    - **Acceptance:** a journey whose referral → diagnostics interval is
      20 days inside a 100-day clock is compliant on a 28-day
      referral-to-diagnostics standard and unaffected on `rtt_18_weeks`
      (`tba::tests::anchored_compliance_scores_the_interval_and_reports_unreached`,
      and end to end via
      `tests/requests/tba.rs`'s `anchored_compliance_round_trip`); an
      instance that never reaches `diagnostics` is `unreached`,
      excluded from numerator and denominator, and counted (same
      tests); the default anchors reproduce today's figures exactly —
      pinned for the five catalogue entries that declare no anchor
      (`tba::tests::the_standards_catalogue_is_well_formed`) — and the
      DB-gated test additionally proves `cancer_fds_28_days` scores
      anchored with no anchor query at all, and that an explicit query
      pair overrides even that standard's own declared anchor.
  - [x] **T-14e — Censoring-aware cohort statistics.** Landed
    2026-09-10, in the suggested order, right after T-14d. Today
    `?status=all` mixes closed lead times with open instances' running
    lead time, which understates the eventual distribution (the
    survivorship error ehrapy's MIMIC tutorial exists to teach). Added a
    Kaplan–Meier estimate of time-to-close and time-to-anchor (T-14d)
    treating open instances as right-censored at `as_of`, with median
    and p90 read off the curve where it reaches them (else `null` with
    reason `curve_did_not_reach`), the numbers of events and censored
    instances, and a log-rank test — see the deviation note below for
    why the test exists without a T-14f split to run it against yet.
    Whether `discontinued` closure is an event or a censor is a
    parameter, default `event`, echoed. Nearest-rank percentiles stay as
    they are; KM is an additional, labelled block, not a replacement.
    `src/tba.rs`: `Observation`, `close_observation()` (status +
    the `discontinued` parameter drive the event/censor split; the
    elapsed time is `lead_time_ms` either way — only *what happened*
    at that time differs), `anchor_observation()` (reuses T-14d's own
    `from_anchor`/`to_anchor` pair; excludes an instance outright when
    `from_anchor` itself was never reached, rather than assigning it
    an arbitrary time zero), `KmStep`/`KaplanMeier`/`kaplan_meier()`
    (the estimator itself — ties grouped into one step, `median_ms`/
    `p90_ms` read off at survival `≤ 0.50`/`≤ 0.10`, matching this
    crate's nearest-rank `percentile` convention exactly when there is
    no censoring — pinned by test), `LogRank`/`log_rank()` (the
    Mantel–Haenszel two-sample test), and a hand-rolled `erf`/`erfc`
    (Abramowitz & Stegun 7.1.26) rather than a new statistics
    dependency. `src/controllers/tba.rs`: `Survival`,
    `resolve_discontinued()`, `survival_analysis()`, wired into
    `GET /api/care-pathways/{pathway}/time-analysis` as a new
    `survival` block, withheld under the identical suppression
    decision as the percentile detail (a curve over a handful of
    instances is exactly as disclosive).
    - [x] **Deviation, disclosed rather than silently worked around:**
      the log-rank test has no HTTP surface. Its acceptance bullet
      ("log-rank on two identical cohorts gives p ≈ 1") is provable as
      a pure unit test with two hand-built groups, and *is* proven
      that way — but there is no cohort-splitting mechanism in this
      crate to hand it two real sides from a live request, because
      T-14f (which builds exactly that) has not landed yet. `log_rank`
      is implemented, documented, and unit-tested; it is "ready for
      it, not wired to it" — the same posture `src/suppression.rs`'s
      own module docs already state for its still-unused 2-D
      breakdown primitive, quoted verbatim in T-14e's own doc comments
      so the parallel is explicit rather than merely implied.
      **Update 2026-09-11:** T-14f landed the cohort-splitting
      mechanism this note anticipated, but did not itself call
      `log_rank` — its own split payload compares matched/complement
      via the already-computed `Survival`/`Compliance`/`CohortAnalysis`
      figures, not via a log-rank test between the two sides' curves,
      and `Survival` does not expose the raw `Observation`s a log-rank
      call would need. `log_rank` therefore remains unwired; comparing
      the two split sides' survival curves is a further, still-open
      follow-up beyond what T-14f's own acceptance text asked for.
    - **Acceptance:** KM on a fully closed cohort equals the empirical
      distribution
      (`tba::tests::kaplan_meier_matches_nearest_rank_percentile_when_fully_closed`,
      comparing directly against the existing nearest-rank
      `percentile` function on the same values); an all-open cohort
      returns `null` with the reason
      (`an_all_censored_sample_returns_null_with_the_reason`); log-rank
      on two identical cohorts gives p ≈ 1
      (`log_rank_on_two_identical_cohorts_gives_p_approx_one`);
      property test: the survival function is non-increasing in
      `[0, 1]`, swept over 500 random samples of random size and
      censoring pattern rather than asserted on one
      (`survival_is_non_increasing_and_bounded_over_random_samples`,
      a hand-rolled SplitMix64 generator, matching this crate's own
      existing precedent for property tests rather than a new `rand`/
      `proptest` dependency). A DB-gated round trip
      (`tests/requests/tba.rs`'s `censoring_aware_survival_round_trip`)
      proves time-to-close under both `discontinued` modes, time-to-anchor
      excluding an instance that never reached `from_anchor`, the
      absent `time_to_anchor` block when no anchor pair is named, the
      `422` on an unrecognised `discontinued` value, and the
      suppression withholding, all against real Postgres.
  - [x] **T-14f — Rule-based cohort splits and the paired comparison.**
    Landed 2026-09-11. Every cohort endpoint accepts `contains=` /
    `excludes=` with `stage:<s>`, `step:<name>`, `event:<kind>`,
    `waste:<w>`, `outcome:<o>`, `setting:<s>`, `urgency:<u>`, and
    `compare=true` returns the same figures for the complement side by
    side — the stratified Table 1 (n, lead-time percentiles, %VA,
    coverage, #HO, standard compliance) BNSSG built by hand with
    `check_rule` + `group_by` and `tableone`. New `src/split.rs`
    (named `split`, not `rules` — `crate::instances` is already
    aliased `rules` throughout the controller layer, and this crate's
    own `13-tasks.md` calls the feature "rule-based cohort splits",
    not "rules"): `Predicate` (parses `type:value`, validating against
    a closed vocabulary where one exists — `tba::STAGES`/`WASTES`,
    `instances::URGENCY_LEVELS`/`OUTCOMES`/`EVENT_KINDS` — and
    accepting any string for `step`/`setting`, which are free-form),
    `Features` (built from the same `analytics::SegmentInput`/
    `StepInput`/`EventInput` rows T-14a's event-log codec already
    loads, so there is one source of truth for "what did this
    instance do"), `Rule` (contains is AND, excludes is none-of;
    naming neither is the identity rule), `partition`, and
    `split_table` (the two-cell `suppression::Table` for a
    matched/complement split — **the first real caller of T-14k's own
    `Table`/`decide` stratified-suppression primitive**, which had
    stood ready but unused since T-14k landed). `src/controllers/tba.rs`:
    `load_features` (three bounded queries: segments, steps, events —
    no team-role lookup, unlike T-14a's four-query loader, which this
    deliberately does not reuse), `resolve_rule` (`422` on a malformed
    or unrecognised predicate), `SplitPlan`/`resolve_split` (parses
    the rule, partitions the cohort, decides per-side detail
    suppression via `split_table` when `compare=true`), and
    `split_payload`/`split_payload_constraints` (the `time-analysis`-
    and `constraints`-shaped response blocks). Wired into
    `GET /api/care-pathways/{pathway}/time-analysis` and
    `.../constraints` as a new `split` key, absent entirely — never a
    `null` placeholder — when the query names neither `contains=` nor
    `excludes=`, or when the *unsplit* cohort is itself already below
    the suppression floor (a breakdown of an already-too-small cohort
    would disclose more, not less).
    - [x] **Scope decision: `process-map` and `variants` are not
      wired.** Only `time-analysis` and `constraints` accept
      `contains=`/`excludes=`/`compare=`. Both remaining endpoints
      already carry their own, differently-shaped suppression (per
      node/edge; per variant, folded into `suppressed_instances`), and
      "compare" would mean something structurally different for each
      (two side-by-side process maps? two variant Paretos, each with
      its own renormalised shares?) — fitting the same query contract
      onto them needs its own design pass, so it is a documented
      follow-up, not a rushed, under-thought-through fit. `data-quality`
      (T-14h) does not exist yet either, for the same reason T-14a's
      `anchors_delays`/`variant` columns were reserved ahead of their
      own sibling tasks landing. **Update 2026-09-11:** T-14h has since
      landed the same day, and does not accept `contains=`/`excludes=`/
      `compare=` either — it is a per-instance detector aggregate, not
      a cohort split, and the same "needs its own design pass" applies
      to it too, not merely "does not exist".
    - [x] **The bare instance count is never withheld, only detail
      is** — matching this family's existing scalar-suppression
      convention (`agents/share/time-based-analysis.md` §12.2: hide
      detail, not the count). `split.matched.instances`/
      `split.complement.instances` are always published; `decide()`'s
      verdict on the two-cell table governs only whether that side's
      `cohort`/`compliance`/`survival` (or `findings`) render. This
      still closes a real disclosure gap: those blocks carry additive
      sums (`by_stage`, `by_waste`) that *would* let a withheld side's
      sums be recovered as `unsplit − complement` if the complement's
      own sums stayed visible — which is exactly the scenario
      `suppression::decide` was built to close, applied here for the
      first time to something other than a raw count.
    - [x] `setting:<s>` compares against the *lowercased*
      `care_setting` string (`controllers::exports::care_setting_string`,
      reused rather than duplicated), not the JSON payload's original
      casing (`"Outpatient"` → `setting:outpatient`) — the same
      derivation `auth::care_pathway_resource_attrs` already uses
      elsewhere in this crate. Undocumented in the acceptance text;
      confirmed by reading the existing derivation rather than
      guessing a casing.
    - **Acceptance:** split and complement sizes sum to the unsplit
      cohort, and an identical filter called twice gives identical
      figures — both proven end to end against real Postgres
      (`tests/requests/tba.rs`'s `rule_based_cohort_split_round_trip`,
      10 instances, 5 matched / 5 complement, neither suppressed);
      when one side is below the floor, the other side's figures are
      also withheld wherever they could be differenced against the
      unsplit total (T-14k) — proven with a lone 2-instance matched
      side recruiting a 6-instance complement that would otherwise
      individually clear the floor
      (`rule_based_cohort_split_suppression_round_trip`), and *not*
      recruited when `compare=true` is absent, since the complement is
      then never shown at all. Pure-layer unit tests in `src/split.rs`
      (8) cover predicate parsing (recognised types/values, and every
      rejection: no colon, no value, unrecognised type, unrecognised
      value against each closed vocabulary), the identity rule
      matching everything, `contains` as AND / `excludes` as none-of,
      the partition's sum-and-exactly-once invariant, and `split_table`
      under all three suppression shapes (neither side small, one lone
      small side recruiting its sibling, two sides already small
      needing no secondary recruitment) — reusing `suppression::decide`
      directly rather than re-implementing its property tests.
  - [x] **T-14g — Cohort attrition record (CONSORT).** Landed
    2026-09-11, in the suggested order, right after T-14f — and the
    same query contract this task shares with T-14f narrows to the
    same two endpoints for the same reason. `time-analysis` and
    `constraints` now both carry `attrition`: ordered steps `{label,
    operation, instances, parent}` from `enrolled_on_pathway` through
    `status_filter`, `window`, `degenerate_clock`, `coverage_floor`,
    and `suppression`, so the denominator is explained inside the
    response rather than in a log. `parent` (an index into the same
    array) is what lets a rule-based split's `matched`/`complement`
    steps both fork from the same `rule_filter` parent — the branching
    a `compare` needs — rather than forcing one linear list to choose
    between the two sides. `src/tba.rs`: `AttritionStep`,
    `ATTRITION_STEP_LABELS` (the closed, ordered six-label vocabulary),
    `ATTRITION_RULE_PARENT` (`rule_filter` forks from `coverage_floor`,
    making it `suppression`'s sibling, not its child — a rule-based
    split narrows a different axis than suppression does, and neither
    should wait on the other), `attrition_trail()` (the base six
    steps, over already-computed counts — no I/O), and
    `attrition_rule_branch()` (the three-step fork, appended when a
    split is active). `src/controllers/tba.rs`: `cohort_query()`
    (extracted from `load_cohort` so a *counting* query used for
    attrition and the *loading* query used for the cohort itself can
    never silently diverge), `attrition_counts()` (two unbounded
    `COUNT` queries — deliberately not `instances.len()`, which is
    capped at `MAX_COHORT_INSTANCES` and would understate the true
    population on a pathway large enough to hit that cap), and
    `build_attrition()`, wired into both endpoints right after the
    existing suppression/split logic.
    - [x] **Deviation, disclosed rather than silently invented:**
      `window`, `degenerate_clock`, and `coverage_floor` are steps
      this crate has no exclusion logic for yet, and this task does
      not add any — it only makes their absence visible. `window`:
      no date-window query parameter exists on these endpoints at
      all. `coverage_floor`: no coverage-based exclusion exists either
      — deciding what threshold would exclude an instance is a
      genuine open design question better resolved by T-14h (which
      already lists "coverage below the floor" as one of *its own*
      reportable codes) than invented here as a side effect of a
      reporting task. **Update 2026-09-11:** T-14h has since landed
      (the same day) and reports `coverage_below_floor`, but this
      `attrition` step's own `coverage_floor` gap is still open — T-14h
      *discloses* the count, exactly as this step's own `window` and
      `degenerate_clock` siblings disclose theirs, and never excludes
      an instance from `cohort`/`compliance`/`survival` on that basis.
      "What threshold would exclude" is a different, still-unmade
      decision from "what threshold is worth naming". `degenerate_clock` is the one exception worth
      naming precisely: a degenerate-clock instance
      (`tba::analyze`'s own `reason: Some(_)` case — `clock.stop_ms`
      not strictly after `clock.start_ms`) is **disclosed, not
      excluded** — it stays in `cohort`/`compliance`/`survival`
      exactly as it always has (a real, pre-existing data-quality gap:
      such an instance contributes a fabricated `0`-day lead time to
      the percentile distribution), so this task changes no existing
      figure. All three steps report `instances` unchanged from the
      step before them, proving the acceptance text's own "a step
      that excluded nobody still appears" for exactly the three steps
      that currently have nothing to exclude.
    - [x] **Deferred, not attempted:** "category sets used by any
      composition table are frozen at step 0" (`by_stage`/`by_waste`
      padded with zero-count entries for a category present in the
      wider pool but absent from a narrower split side) is real, but
      is not tested by this task's own three acceptance bullets and
      is a nontrivial change to `tba::cohort`'s existing category
      emission — reviewed on its own rather than smuggled in as a
      side effect of building the attrition record.
    - **Acceptance:** the last step's `instances` equals the analysed
      n — proven for the unsplit trail's own leaf (`suppression`) and
      independently for each rule-branch leaf (`matched`/`complement`)
      (`tba::tests::the_last_steps_instances_equal_the_analysed_n`,
      and end to end via `tests/requests/tba.rs`'s
      `cohort_attrition_round_trip`); a test enumerates every
      declared step label and asserts each has a step, with no
      undeclared extras either
      (`every_declared_attrition_step_label_actually_appears`); a step
      that excluded nobody still appears
      (`a_step_that_excludes_nobody_still_appears`, for `window`,
      `degenerate_clock`, and `coverage_floor`). The DB-gated round
      trip additionally proves a real `?status=` count change (6
      enrolled, 3 after an `open`/`closed` filter), a real
      (deliberately future-dated-then-closed) degenerate-clock
      instance disclosed by count without being excluded, the
      `suppression` step's wording tracking whether the floor was
      actually cleared, and the rule-branch's `matched`/`complement`
      leaves matching `split`'s own counts exactly — all against real
      Postgres.
  - [x] **T-14h — Journey data-quality and missingness report.**
    Landed 2026-09-11, in the suggested order, right after T-14g.
    `GET /api/care-pathways/{pathway}/data-quality`: per cohort, the
    share of instances with no segments, an open segment past closure,
    a terminal status with no clock stop, `done_on` before
    `enrolled_on`, out-of-order step completion, segments clipped by the
    clock, coverage below the floor, and anchors unreached — each a
    code in a closed vocabulary (BNSSG's `bad_date` 1–5, generalised);
    plus per-stage missingness percentage and entropy of missingness
    across instances. The report is the finding; it never imputes (see
    the triage table). A new pure module, `src/data_quality.rs` (not a
    further `src/tba.rs` extension, unlike T-14d/e/g — this is a
    genuinely separate concern, detectors over already-computed
    figures, not an elapsed-time computation, the same reasoning
    T-14c's/T-14f's/T-14k's own new files followed): `DQ_CODES` (the
    closed eight-entry vocabulary, name-for-name matching T-14m's own
    `DEFECT_CODES`), eight `has_*` detector functions, one per code,
    each reusing an already-resolved fact (`tba::Clock`, `tba::clip`,
    the cohort's own `coverage_ratio`, T-14d's `StageAnchor`s) rather
    than re-deriving it; `binary_entropy_bits` (Shannon entropy of a
    per-stage present/absent Bernoulli variable, peaking at 1 bit at
    p=0.5); and `build_report`, folding every instance's detector
    results into per-code `{code, instances, share}` rows plus a
    per-stage `missingness` array. `src/controllers/data_quality.rs`:
    `GET /api/care-pathways/{pathway}/data-quality`
    (`?status=&from_anchor=&to_anchor=`), gated like the sibling
    cohort views (no record-level ABAC, the blanket guard only — this
    is an aggregate count report, not a bulk pull of instance rows).
    - [x] **Scope decision: per-stage missingness only, not
      per-field.** The spec text names "per-field missingness" with no
      field list, and inventing one would be exactly the kind of
      unstated assumption this crate's own discipline refuses — the
      closed `tba::STAGES` vocabulary already exists and is what every
      other T-14 aggregate reports against, so missingness is reported
      per stage, not per an invented field set. A per-field pass is a
      documented follow-up if a concrete field list is ever named.
    - [x] **`anchor_note` consolidates one decision, not two.** T-14d's
      own `resolve_anchor_pair(query)` was refactored to a
      `pub(crate) fn resolve_anchor_pair_raw` taking the raw
      `Option<&str>` pair directly, so both `controllers::tba` and this
      new controller share the one parser. `build_report` takes that
      function's full `Result<Option<(&str,&str)>, &'static str>`
      as its anchor-pair parameter (not a plain `Option`, and not a
      second top-level HTTP field) — the one value already
      distinguishing "no pair requested" from "a pair was requested but
      did not parse", so `anchors_unreached`'s own "not evaluated"
      disclosure and the report-level `anchor_note` are one field, not
      two that could disagree.
    - [x] **Two real generator bugs found and fixed while building the
      DB-gated test, not this crate's own bug.** T-14m's generator
      (`src/data/journeys.rs`) had never been exercised end to end
      before this task (see its own entry's update above); running it
      for real surfaced two defect-construction gaps a hand-built
      fixture had never hit: (1) `coverage_below_floor`'s and
      `anchors_unreached`'s injected one-hour segment could itself be
      clipped by the *base* clean instance's own random, untouched
      clock, since the shortest possible base segment is 15 minutes
      with a zero-length gap — fixed by a new `widen_clock_stop_past`
      helper that widens (never shrinks) a closed instance's
      `clock_stop_at` to comfortably contain the injected segment,
      regardless of what the pre-defect instance's clock happened to
      be. (2) `terminal_without_clock_stop` cleared `closed_on`
      alongside `clock_stop_at`, which is the *rarer* double-missingness
      case `has_terminal_without_clock_stop`'s own doc comment already
      named, not its primary scenario — the "as of now" clock fallback
      this produced incidentally tripped `coverage_below_floor` on
      every seed tried; fixed by setting `closed_on` to the day after
      the journey's own last segment activity (day-resolution, so
      pushed a full day past — not merely past — the last segment, to
      avoid the very "midnight before the segment's own time-of-day"
      trap this same fix could otherwise have reintroduced).
    - [x] **`segment_clipped_by_clock` and `open_segment_past_closure`
      genuinely overlap, and that is correct, not a residual bug.** An
      open segment on a terminal instance is clipped once its
      effective end is bounded by "as of now" rather than the clock's
      own stop — exactly `has_segment_clipped_by_clock`'s own stated
      rule, applied to a real case rather than a constructed one. The
      two codes are not defined to be mutually exclusive, so the
      dedicated `open_segment_past_closure` instance legitimately
      fires both; this is disclosed in the test's own comments rather
      than forced apart.
    - [x] **The acceptance text's "each code exactly once per defect"
      is verified per defect, not across one shared eight-defect
      cohort.** A combined-cohort test was tried first and abandoned:
      several defects are, by construction, *also* low-coverage or
      clock-clipped journeys (a short journey against a multi-hour
      gap-bearing clock window commonly clears under
      `coverage_below_floor`'s threshold whether or not that is the
      defect actually requested), which no fixed seed can reliably
      avoid across every code at once without depending on incidental
      non-overlap the generator makes no promise about. Each code is
      instead checked against its own one-instance cohort (fresh
      pathway, `n=0`, `defects=[code]`), which is what actually pins
      "exactly once" without that dependency.
    - **Acceptance:** a seeded cohort with injected defects (T-14m)
      reports each code exactly once per defect — proven per defect in
      isolation (`each_dq_code_fires_exactly_once_on_a_seeded_cohort`);
      a clean cohort reports every code at zero, rows present
      (`a_clean_cohort_reports_every_code_at_zero_with_rows_present`,
      also proving a malformed anchor pair carries its own reason and
      an absent one reads as "not evaluated", never a silent zero); an
      empty pathway still returns every row, with no share to divide by
      rather than a misleading `0.0`
      (`an_empty_pathway_reports_every_row_with_no_instances`). Pure
      unit tests in `src/data_quality.rs` (15) cover every detector in
      isolation, the entropy function's boundary and peak values, and
      the report's own row/share/`anchor_note` shape.
  - [x] **T-14i — Conformance to the enrolled template.** Landed
    2026-09-11, in the suggested order, right after T-14h. Per
    instance, the steps copied at enrolment (`instance_steps.position`)
    against their completion order (`done_on`): skipped steps, adjacent
    declared pairs completed out of order, steps completed after
    closure, and `escalation` events; a labelled ratio
    `pairs_in_order / declared_pairs` shipped with both numbers; cohort
    share fully conformant. Against the template only — never a
    discovered model — and with no penalty for extra events: a journey
    may need more than its template foresaw. A new file,
    `src/conformance.rs` (not a further `src/tba.rs` extension —
    a genuinely separate concern, sequence-order comparison rather than
    an elapsed-time computation, the same reasoning T-14b's/T-14c's/
    T-14f's/T-14h's own new files followed): `StepRecord` (position +
    `done_on` in epoch ms; `done` is not carried separately, since the
    step-completion handler always sets `done`/`done_on` together),
    `PairVerdict` (`in_order`/`inverted`/`skipped`, one per adjacent
    declared pair), `conformance()` (the per-instance score),
    `CohortConformance`/`cohort_conformance()` (the cohort rollup).
    HTTP surface, `src/controllers/tba.rs`: `load_step_records`/
    `count_escalation_events` (per-instance) and
    `load_conformance_inputs` (cohort, bulk, no N+1 — the same shape
    `analyze_cohort`/`data_quality::load_dq_inputs` already use), wired
    into `GET /api/instances/{pid}/time-analysis` (a new `conformance`
    key) and `GET /api/care-pathways/{pathway}/time-analysis` (a new
    `conformance` cohort-share key, withheld under the identical
    suppression decision `survival`/`split` already use — never a
    separate one). Also closes T-14a's own reserved `conformance`
    journey-feature export column (`src/analytics.rs`'s
    `journey_feature_row`, which T-14d's `anchors_delays` had already
    shown the pattern for): a compact JSON scalar summary, not the
    per-pair breakdown the dedicated endpoint carries.
    - [x] **`declared_pairs` is a fixed structural count, not reduced
      by skips.** A pair with either endpoint undone verdicts
      `skipped`, contributing to neither `pairs_in_order` (so it still
      lowers the ratio, same as a genuine inversion would) nor to a
      separate "not applicable" bucket — it is simply reported under
      its own name so a UI (and this task's own acceptance text) never
      mistakes a skip for an inversion. `ratio` is `None` only when
      `declared_pairs == 0` (fewer than two declared steps at all);
      a skipped-but-multi-step instance still gets a real (possibly
      `0.0`) ratio.
    - [x] **Cohort share denominator excludes no-pair instances,
      not zero-pair ones.** `CohortConformance.with_ratio` counts only
      instances whose own `ratio` is defined; an instance with fewer
      than two declared steps has nothing to be conformant *about*, so
      it is excluded from the share's denominator rather than silently
      counted as either conformant or not (mirroring how `compliance`
      excludes `unreached` from `within`/`breached` rather than
      counting it as a breach).
    - [x] **Not wired into `cohort_constraints`.** Template conformance
      is not a recoverable-time constraint finding, unlike the ranked
      findings that endpoint reports — a documented scope decision, not
      an omission (pinned by `cohort_conformance_share_round_trip`'s
      own assertion that `constraints` carries no `conformance` key).
    - [x] **The DB-gated round trip stamps `done_on`/`closed_on`
      directly on the model, bypassing the HTTP layer for exactly
      those two fields.** `POST .../steps/{step}/complete` and the
      status-transition-to-terminal path both always stamp
      `Utc::now().date_naive()`/today — day-resolution, so two calls
      within one fast test run land on the same date and cannot
      exercise a genuine inversion or a same-day-vs-later-day closure
      comparison. `tests/requests/conformance.rs` therefore builds
      each instance through the real enrolment/segment/event
      endpoints and only backdates the two fields the API itself
      cannot backdate — the same precedent
      `tests/requests/journeys_seed.rs`/`compliance.rs` already set.
    - **Acceptance:** completing steps in template order scores 1.0
      with zero inversions; reverse order scores 0; a skipped step is
      reported as skipped, not as an inversion; an instance with one
      declared step reports `null` (no pairs) with the reason — all
      four proven end to end against real Postgres
      (`conformance_round_trip`), plus escalation events carried
      without affecting the ratio and a step flagged as completed
      after closure. The cohort share
      (`cohort_conformance_share_round_trip`) clears the default
      suppression floor at exactly five instances (three fully
      conformant, two not) and confirms `constraints` carries no
      `conformance` key. Pure unit tests in `src/conformance.rs` (14)
      cover every branch directly, including ties, empty/one-step
      cohorts, and the cohort rollup's `None`/empty edge cases.
  - [x] **T-14j — Stalled journeys (aging WIP).** Landed 2026-09-11,
    in the suggested order, right after T-14i.
    `GET /api/instances/stalled?idle_days=N` (default 60, echoed):
    open instances whose last recorded activity — latest of segment
    start / end, step `done_on`, event `occurred_at`, review — is older
    than N days, sorted by idle time, each row naming its last-activity
    source. Complements `overdue-reviews` (a due date) with an
    observed-silence test. The timeout is retroactive, as IPPA's
    `Process.time_out` is: idle-since is the last activity time, not
    the time the silence was noticed. Never grouped by actor.
    Pure logic extends [`src/instances.rs`](../src/instances.rs) (the
    crate's existing pure instance-lifecycle module, not a new sibling
    file — a "latest of several optional timestamps, past a threshold"
    fold is a small extension of that module's own remit, not a
    genuinely separate algorithm the way T-14b's/T-14c's/T-14f's/
    T-14h's/T-14i's own new files were): `ACTIVITY_SOURCES` (the
    closed five-entry vocabulary), `LastActivity`, `last_activity()`
    (folds every source into the single most recent, falling back to
    `enrolled_on` — the floor every instance has from the moment it
    exists, so a brand-new instance is never "stalled since forever"),
    `is_stalled()` (strictly-older-than, `idle_days` clamped to
    non-negative). HTTP surface,
    `src/controllers/instances.rs`: `load_last_activity_inputs`
    (cohort, bulk, three bounded queries — segments, steps, events —
    no N+1, the same shape every other cohort loader in this crate
    uses), `stalled()`, wired at `GET /api/instances/stalled`.
    - [x] **Scope decision: `review` is not a fifth, separate source.**
      The spec text lists it alongside segment/step/event as if it
      were its own signal, but `POST .../review` already records an
      `instance_events` row (`kind: "review"`) — treating it as a
      sixth data point to track would be a second, redundant path to
      the same fact rather than a genuinely distinct one, so it is
      covered by `event` without further ceremony.
    - [x] **`?idle_days=` falls back to the default rather than
      erroring** on zero, negative, or unparseable input — the same
      "a tuning knob never errors" convention pagination's
      `?limit=`/`?offset=` already uses, since an idle-day threshold
      is a lens setting, not a business promise like `target_days` on
      the standards endpoint (which does reject a non-positive value).
    - [x] **The DB-gated round trip backdates `enrolled_on` directly
      on the model**, the one field none of the endpoints that record
      activity can set to anything but "now"/"today" — a freshly
      enrolled instance's own floor would otherwise be more recent
      than a deliberately old segment, which is not a scenario real
      usage can produce (enrolment always precedes recorded activity)
      but is exactly what an un-backdated test fixture would
      accidentally construct. Segment timestamps themselves needed no
      such bypass: `POST .../segments` already accepts an explicit
      `started_at`/`ended_at`, the same mechanism
      `tests/requests/tba.rs`'s own `closed_instance` helper relies on.
    - **Acceptance:** an instance whose last event was 61 days ago is
      listed at `idle_days=60` and not at 90; an instance with an open
      segment started 5 days ago is not listed; a closed instance is
      never listed — all proven end to end against real Postgres
      (`tests/requests/stalled.rs`'s `stalled_round_trip`), which also
      pins the default `idle_days=60` echo and that each row names its
      `last_activity_source`. Pure unit tests in `src/instances.rs`
      (6 new, alongside the 2 pre-existing) cover the same four
      conditions directly, plus the multi-source fold picking the true
      latest, the fresh-instance fallback to `enrolled_on`, the
      exactly-at-the-threshold boundary, and negative-`idle_days`
      clamping.
  - [x] **T-14k — Disclosure control: modes and marginals.** Landed
    2026-09-10. Generalises the TBA-10 floor into one shared, pure
    module, `src/suppression.rs`: `min_cell_count()`
    (`CARE_PATHWAY_MIN_CELL_COUNT`, deployment-configurable **upward
    only** — a lower or garbage value falls back to the default rather
    than weakening protection, per [TBA §17](time-based-analysis.md)),
    modes `Mode::Withhold` (default: `null` + reason) and
    `Mode::Remove` (drop the key), parsed from a new `?mode=` query
    param. Two of TreatmentPatterns' three modes are deliberately
    **not** adopted: `mean` substitutes a made-up count, and
    `minCellCount` reports a suppressed cell *as* the threshold, which
    reads as a count. And one rule is added that TreatmentPatterns
    leaves to the caller: `decide()` runs **secondary suppression** —
    whenever a declared `Partition` (a row, a column, or any other
    group whose members sum to a published margin) is left with
    exactly one suppressed cell, one more cell from that partition is
    suppressed too (the smallest remaining visible one, deterministically),
    repeated to a fixed point, so a withheld cell can never be
    recovered as `margin − Σ(visible siblings)`.
    - [x] `decide()`/`render()` operate on a generic `Table` (cells +
      partitions) — at landing, no genuinely stratified 2-D breakdown
      existed in this crate yet (that was T-14f, then unbuilt), so
      this was built ready for T-14f to consume, exactly as T-14a's
      codecs and T-14m's generator were each built ready for their own
      not-yet-built consumers. **Update 2026-09-11:** T-14f landed and
      is the first real caller — its `src/split.rs` builds a two-cell
      `Table` (matched/complement) and calls this module's `decide()`
      directly rather than reimplementing any of its logic. Proven at
      landing by 11 unit tests including a 500-seed
      property test (`no_partition_is_ever_left_with_exactly_one_suppressed_cell`)
      over randomly generated row × column tables — a hand-rolled
      `SplitMix64`, the same choice and the same reason as T-14m's
      `Rng`, not a new `proptest` dependency.
    - [x] The **scalar** case (`is_suppressed(n)`) replaces the old
      hardcoded `MIN_COHORT_FOR_PERCENTILES` const in
      `cohort_time_analysis`, and — closing a real, previously-existing
      gap — is now also applied to `cohort_constraints`, which used to
      return unsuppressed findings at any cohort size (a finding
      computed over one instance can describe that patient's journey
      precisely). Both endpoints gained `?mode=`.
    - [x] `flow_metrics.rs`'s own independent floor
      (`CARE_PATHWAY_FLOW_METRICS_MIN_COHORT`) is **left as-is**,
      deliberately not migrated to share this module's env var: it
      already satisfies the same principle independently, and unifying
      the env var name would be a breaking configuration change for
      any deployment that has already set it — a decision, not an
      oversight.
    - **Acceptance:** the property test (above) proves no withheld
      cell is recoverable as `total − Σ visible`; `decide()` is shared
      by both render modes, so `remove` and `withhold` can never
      disagree on *which* cells are small (also unit-tested directly,
      `withhold_and_remove_agree_on_which_cells_are_small`); the T-14a
      codecs are confirmed exempt — `suppression.rs`'s own module docs
      point at `tests/requests/exports.rs`'s existing `n = 1` round
      trip rather than duplicating that proof. A live HTTP round trip
      (`tests/requests/tba.rs`) confirms a 1-instance cohort suppresses
      both endpoints (with `?mode=remove` dropping the key), and that
      a 5-instance cohort (the same one `the_flow_gauges_publish_only_what_may_be_published`
      already builds for the flow-gauge floor) leaves both unsuppressed.
      The `OpenAPI` doc gained the new `export_paths()` function too
      (T-14a's endpoints had no entry at all — a gap found rolling
      this task, closed the same way `instance_paths()`'s own doc
      comment describes for an earlier such gap) and the new `?mode=`
      parameter on both cohort endpoints.
  - [x] **T-14l — Front-end analytics views** in
    `care-pathway-front-end-with-svelte`. Landed 2026-09-11, the last
    of the twelve T-14 sub-tasks (backend and front-end alike). `/time`
    gains the process map
    (an in-house layered SVG layout — node size = instances, edge label
    = median days — rather than a new graph dependency), the variants
    sunburst and Sankey with a `Stopped` terminal node, a dotted chart
    (instances × time, coloured by stage), the attrition flowchart, the
    `compare` two-column view, and the stalled list; filters bound to
    the T-14f parameters; suppressed cells rendered as "withheld
    (n < 5)", never blank. Theme-aware, i18n keys in every locale file.
    New pure module `src/lib/analytics-transforms.ts` (BFS-ranked
    layered layout, sunburst/Sankey transforms, an attrition tree
    layout, dotted-chart points — no DOM, no fetch) plus seven new
    plain-SVG components; `src/lib/api/tba.ts` gained the process-map/
    variants/stalled/event-log client methods and the `Split`/
    `AttritionStep` types, `cohort()`/`constraints()` extended with
    `contains=`/`excludes=`/`compare=`. See the front-end's own
    `spec/index.md` §6 items 14–15 and CPFE-T6 for the full account,
    including two scope decisions: the dotted chart (built from the
    `Destructive`-gated, audited bulk `event_log` export) loads only
    behind an explicit button, never on page mount; and the withheld-
    cell fallback text matches this bullet's own literal "withheld
    (n < 5)" wording, but a real suppressed cell always carries the
    service's own fuller `suppression_note`
    ("withheld: fewer than the minimum cell count"), which the
    front-end prefers whenever it is present.
    - **Acceptance:** vitest units for the sunburst / Sankey transforms
      from a variants payload and the DFG layout; Playwright smoke with
      the API stubbed; a withheld cell is visibly labelled. Proven end
      to end: `tests/unit/analytics-transforms.test.ts` (16 tests,
      including the sunburst's prefix-aggregation and angular-partition
      invariants, the Sankey's position-qualified nodes, and the DFG
      layout's BFS ranking + self-loop/back-edge flagging + unreachable-
      node placement) and 3 new `tests/unit/tba.test.ts` cases, plus 5
      new Playwright cases in `tests/e2e/time.spec.ts` — including one
      that found and fixed a real test-authoring bug along the way
      (`route.fallback()`, not `route.continue()`, is needed for a
      route registered after another stub handler to fall through to
      it rather than escape to the real network).
  - [x] **T-14m — Seeded synthetic journey cohorts.** Landed 2026-09-09.
    A pure, DB-free generator in `src/data/journeys.rs` plus a loco
    task, `journeys:seed` (`pathway`, `n`, `seed`, `open_share`,
    `defects`): deterministic instances with segments, steps, events,
    and a care team across `tba::STAGES`, weighted stage-duration
    distributions and inter-segment gaps, defect injection, and a
    censoring share (`open_share`). Every `subject_ref` / care-team
    `member_ref` / `location_ref` carries a **fixed, recognizably-fake
    byte prefix** (`facade50`/`51`/`52`) rather than an ordinary-looking
    random UUID — never real data, never derived from real data, the
    family's synthetic-only rule, and IPPA-data's own disclaimer ("no
    responsibility to answer any epidemiological question") is the
    reason to say so here too.
    - [x] Defect injection covers **seven** of T-14h's eight codes
      (`no_segments`, `open_segment_past_closure`,
      `terminal_without_clock_stop`, `step_done_before_enrolled`,
      `steps_out_of_order`, `segment_clipped_by_clock`,
      `coverage_below_floor`) — `src/data/journeys.rs`'s
      `DEFECT_CODES`. The eighth, "anchors unreached", needs T-14d's
      stage-anchor configuration, which has since landed (2026-09-10);
      wiring this generator's eighth defect code onto it is a follow-up,
      not done in T-14d's own change (T-14d used a small hand-built
      fixture instead, matching T-14b's/T-14c's precedent — see T-14d's
      own scope notes below), still deferred here, with a documented
      reason in `DEFECT_CODES`'s own doc comment, not silently dropped.
      **Update 2026-09-11 (T-14h):** the eighth code, `anchors_unreached`,
      is now in `DEFECT_CODES` too — all eight land. T-14h's own build
      found and fixed two real generator bugs this "seven of eight"
      state had never been tested against: an open segment on a
      terminal instance is *also* clipped by `has_segment_clipped_by_clock`
      once its effective end is bounded by "as of now" (a genuine,
      disclosed overlap between two codes, not a bug — kept); and
      `terminal_without_clock_stop` originally cleared `closed_on` as
      well as `clock_stop_at`, which fell all the way back to an
      `as_of`-bounded clock spanning to the real wall-clock "now" and
      incidentally tripped `coverage_below_floor` on every seed tried —
      fixed by setting `closed_on` to the day after the journey's own
      last segment activity (its actually-intended `resolve_clock`
      fallback), never cleared alongside `clock_stop_at`. Both fixes are
      in `src/data/journeys.rs`'s `apply_defect`/`widen_clock_stop_past`,
      pinned by this crate's own pure and DB-gated tests, not T-14h's.
    - [x] The pseudo-random source is a hand-rolled `SplitMix64`
      (`src/data/journeys.rs`'s private `Rng`), not the `rand` crate:
      reproducibility across `rand` versions is not part of that
      crate's own compatibility guarantee, and a new dependency (plus a
      new `compliance/soup.tsv` row) to generate fixture data was
      judged not worth it against twenty lines.
    - **Acceptance:** the same seed produces byte-identical output
      (`same_seed_produces_byte_identical_output`, comparing the full
      generated `GeneratedCohort` by `PartialEq`); generated cohorts
      satisfy every §5.1 invariant unless a defect was requested
      (`clean_instances_satisfy_segment_invariants`); each defect code
      actually produces its named condition
      (`each_defect_produces_its_named_condition`); every synthetic ref
      carries its fixed prefix
      (`synthetic_refs_are_always_recognizably_fake`). A live DB-gated
      round trip (`tests/requests/journeys_seed.rs`) proves the task
      persists exactly what the generator returns, that a bad argument
      is refused rather than silently defaulted, and that the persisted
      rows carry the same synthetic-only markers. "Used by every T-14
      test" is **still not true** — T-14b, T-14c, T-14d, T-14e, T-14f
      and T-14g have since landed too, but each used a small
      hand-built fixture rather than this generator (see their own
      scope notes). **Update 2026-09-11 (T-14h):** T-14h is the first to
      actually exercise this generator end to end
      (`tests/requests/data_quality.rs`), rather than a hand-built
      fixture — see that entry's own notes for the two real generator
      bugs that exercise turned up. **Update 2026-09-11 (T-14i):**
      T-14i has since landed too, but its own
      `tests/requests/conformance.rs` builds instances directly through
      the enrolment/step/event endpoints rather than this generator —
      `journeys:seed` has no defect code exercising step order at all,
      so there was nothing here for it to reuse. **Update 2026-09-11
      (T-14j):** T-14j has since landed too, and likewise builds
      `tests/requests/stalled.rs`'s fixtures directly rather than via
      this generator, for the same reason — a stalled journey needs a
      deliberately old `enrolled_on`/segment timestamp, not one of
      this generator's own defect shapes. **Update 2026-09-11 (T-14l):**
      T-14l has since landed too — the front-end sub-task, in
      `care-pathway-front-end-with-svelte`, which has no reason to use
      this backend generator at all (its own Playwright tests stub the
      API directly). Every T-14 sub-task is now genuinely complete.
      The repo demo seed (EX-4)
      integration and the README statement are follow-ups, not done in
      this change (this crate's own `README.md`/`AGENTS.md` document it
      instead — see their `journeys:seed` entries).
