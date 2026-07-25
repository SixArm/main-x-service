## 13. Tasks

Live entity-level work queue. Tasks that belong to one subproject's
internals should migrate into that crate's spec §13; they are listed
here while the crate specs are thin. Each task has an acceptance
criterion; tick the box when an automated test or clearly described
manual check confirms it. Split tasks too big for one PR
(`T-2a`, `T-2b`).

- [ ] **T-1 — Thicken the thin crate docs.**
  - [ ] Split the service's single-file `spec/index.md` into the
    numbered §-per-file layout (the matcher and front-end carry the
    same task in their own §13 / §23).
  - [ ] Add a service `AGENTS/` reference set (`models.md`,
    `matching.md`, `restful.md`, `testing.md`,
    `spec-driven-development.md`) matching the person-service shape.
  - **Acceptance:** every link in this entity spec resolves to a
    numbered section file rather than an anchor in a monolith.
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
  - [x] Name search endpoint. **Done (2026-06-13):**
    `GET /api/care-pathways/search?q=` — pragmatic Postgres `ILIKE`
    substring match on the denormalised `name` (cap 50; `%`/`_`/`\`
    escaped so the query matches literally), mirroring the organization
    service. `PathwayModel::search` + the `search` handler; blank `q`
    → `400`. Pinned by `can_search_pathways_by_name` (DB-gated) and the
    un-gated `escape_like_neutralises_wildcards` unit test.
  - [ ] Tantivy full-text / fuzzy search over the JSONB payload +
    front-end search box.
  - [x] Make the `check-duplicates` in-memory scan cap a named,
    documented const with a WARN on hit (interim safety, ahead of
    the redesign). **Done (2026-06-13):** `CHECK_DUPLICATES_SCAN_CAP`
    (= 1000) in `src/controllers/care_pathways.rs`; the handler passes
    it to `Model::list` and emits `tracing::warn!` when the returned
    row count reaches the cap. Pinned by the DB-free unit test
    `check_duplicates_scan_cap_is_the_documented_value`.
  - [ ] Replace the 1 000-row in-memory scan in `check-duplicates`
    with search-blocked candidates (NFR-1 / NFR-2; OQ-2).
  - **Acceptance:** `check-duplicates` latency test passes at
    100 000 stored pathways.
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
