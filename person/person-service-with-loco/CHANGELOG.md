# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]

### Added — bulk export: rollout step 3 — masking + gating + audit (2026-07-10)

- Bulk **export** now honours the §8 privacy contract
  ([bulk-import-export.md](../../agents/share/bulk-import-export.md) §8):
  a `masking_profile` (`masked` default / `full`) and an
  `include_soft_deleted` flag (default `false`), plus a per-export audit
  row on every run. Import, CSV, and Parquet are untouched (other steps).
- New `bulk::MaskingProfile` (`masked` | `full`, default `masked`, wire
  tokens `masked`/`full`). `ExportParams` gains `masking_profile` and
  `include_soft_deleted`.
- `process_export_job` now maps every record through
  `privacy::mask_person` under the default `Masked` profile before
  encoding, so a default export never reveals more than the masked read
  view; `Full` leaves records unmasked. It returns the row count for the
  audit and **rejects** `include_soft_deleted=true` as
  `Error::Validation` ("not yet supported") — the repository cannot
  express a soft-deleted listing without a larger change, so the flag is
  refused rather than leaked or silently ignored.
- `POST /api/persons/export` accepts `masking_profile` (default `masked`)
  and `include_soft_deleted` (default `false`); an unknown profile token
  is a `400`. The **privileged** paths (`full` OR `include_soft_deleted`)
  are gated behind elevated authorisation via person's existing
  record-level guard (`auth::authorize_record` with a `destructive`
  action): a no-op when `PERSON_REQUIRE_AUTH` is off, else `403` unless
  the ABAC policy allows it (`access=admin` / `svc=true` by default). The
  default masked, active-only export stays open to any authorised caller.
- The export audit row (worker `audit_export`) now records actor, the
  filter (`q`/`limit`/`offset`), format, masking profile,
  `include_soft_deleted`, and the row count — written even for a zero-row
  export via the new `AuditLogRepository::log_export` (`EXPORT` action).
- Tests: DB-free unit — masking applied for `Masked` / skipped for `Full`
  (`apply_masking`), the privileged-path gate decision
  (`export_requires_elevation`), and `MaskingProfile` round-trip;
  DB-gated `#[ignore]` — a default export returns masked JSONL and writes
  an `EXPORT` audit row, a `Full` export returns unmasked, and
  `include_soft_deleted=true` is rejected.

### Added — bulk import/export: rollout step 1 (2026-07-10)

- Person is the **reference entity** for the family-wide bulk
  import/export capability
  ([bulk-import-export.md](../../agents/share/bulk-import-export.md) §3–§7,
  §10; rollout step 1). Async, job-based, driven by a Postgres-backed
  background worker (`bg_pg`); **JSONL** is the lossless reference format.
- New `bulk_jobs` table (migration `m20260710_000002_create_bulk_jobs`;
  `UNIQUE(entity, kind, idempotency_key)`), SeaORM entity
  (`db::models::bulk_jobs`), and persistence helpers (`db::bulk_jobs`:
  `create`/`set_input_url`/`set_status`/`finish_import`/`finish_export`/
  `find_by_id`/`list_recent`).
- New `bulk` module: `store` (the `ArtifactStore` trait +
  `LocalFsArtifactStore` for dev/test, `PERSON_BULK_ARTIFACT_DIR`; S3 is
  the deployment backend, deferred), `jsonl` (streaming codec — one
  person wire record per line), `stable_key` (person's upsert key —
  §10.1: a strong scheme-scoped identifier (SSN/TAX/NPI/PPN) → `tax_id` →
  record `pid`), `error_report` (§7 per-row `row_number/field/code/message`
  → CSV), `pipeline` (the testable `process_import_job` /
  `process_export_job` core), and `worker` (the `BulkJobWorker` adapter,
  registered in `connect_workers`).
- Import (§6): per row parse → validate (the single-create validators, so
  the same `422` reasons) → **upsert in place** when the stable key
  matches an existing record (idempotent re-import), else create; invalid
  rows are skipped into the downloadable error report, never aborting the
  load; each written row emits its normal event + audit via the repository.
- Export (§8): honours the person list/search filter, streams matching
  records to a JSONL artifact, and writes an export audit row.
- Endpoints (`bulk::handlers`, mounted on `persons_routes`, in OpenAPI):
  `POST /api/persons/import` (multipart, `202 {job_id}`, `dry_run`
  supported; a declared destructive POST),
  `POST /api/persons/export` (JSON filter, `202 {job_id}`),
  `GET /api/persons/import/{id}` + `GET /api/persons/export/{id}` (status +
  counts + `errors_url`/`download_url`), `GET /api/persons/bulk-jobs`.
- Tests: DB-free unit (JSONL round-trip, stable-key precedence,
  error-report shape, store round-trip, enum round-trips — 16 tests) plus
  DB-gated `#[ignore]` pipeline tests (create-then-idempotent-upsert with
  error report, dry-run commits nothing, export JSONL round-trip).
- **Deferred** (rollout steps 2–5, noted not built): CSV + Parquet
  formats, export masking profiles + `include_soft_deleted` gating,
  keyless-row → duplicate-review routing, S3 artifact store, other
  entities.

### Added — cross-service links: `same_identity` write side (2026-07-10)

- Person is the **reference originator** of the cross-service
  `same_identity` (person ↔ worker) backbone edge
  ([cross-service-linking.md](../../agents/share/cross-service-linking.md)
  §4.1/§4.2/§9, rollout step 2). New `entity_links` table (migration
  `m20260710_000001_create_entity_links`; `UNIQUE(from_pid, kind, to_ref,
  valid_from) NULLS NOT DISTINCT` for idempotent upsert), SeaORM entity
  (`db::models::entity_links`), and persistence (`db::entity_links`:
  `upsert` — idempotent, revives a soft-deleted row; `list_active`;
  `find_active`; `list_all_active(since)`; `soft_delete`).
- Endpoints (`api::rest::links`, mounted on both router surfaces):
  `POST /api/persons/{id}/links` (validate → upsert → best-effort audit),
  `GET /api/persons/{id}/links`, `DELETE /api/persons/{id}/links/{link_id}`,
  and the aggregator's reconciliation pull
  **`GET /api/persons/links[?since=<rfc3339>]`** returning
  `{ "edges": [EdgeDetail…] }` in the canonical §4.2 shape (`edge_id` /
  `edge_kind` / `from_ref = person:<id>`). Depends on the shared
  `entity-ref` crate.
- Validation (`validate_edge`, DB-free, unit-tested): accepts **only**
  `same_identity` person → worker; `422` for a non-`same_identity` kind
  (`subject_of` / `works_at`), a `same_identity` to a non-worker, an
  unknown kind, or a malformed `to_ref`. Writes are authorised at the
  person record-level (`authorize_record`) and audited (`person_link`
  create/delete).
- **Deferred:** cross-service `linked`/`unlinked` **event** emission —
  neither the durable `Envelope` (no link kind / no `data`) nor the
  in-memory `PersonEvent::Linked` (person `Uuid`s only) can carry the
  §4.2 edge `data` without a cross-cutting refactor; the bulk endpoint is
  the aggregator's sync path (§8). Worker's symmetric side is the
  follow-up.

### Added — authz: record-level resource attributes + obligations (2026-07-05)

- Record-level ABAC (verifier 0.3 → 0.6). Beyond the coarse blanket
  guard, `GET`/`PUT`/`DELETE /api/persons/{id}` run a second, finer
  decision after loading the record: `auth::person_resource_attrs`
  derives `resource.active` / `resource.deceased` / `resource.managing_org`
  and `auth::authorize_record` calls `Policy::evaluate_with_context`
  (gated on `PERSON_REQUIRE_AUTH`, a no-op when off). `PUT`/`DELETE`
  evaluate the **stored** record. A deployment can thus write e.g.
  "deny write on a deceased person's record unless `access=admin`".
- Also supplies **environment attributes** (`env.hour` / `env.after_hours`,
  UTC, via `auth::request_env_attrs`) and honours the **`mask`
  obligation** on `GET` (returns `mask_person`). New `auth::MaybeAuthUser`
  extractor + module-level `auth::policy()` / `require_auth()` accessors.
  DB-free tests for the resource-attribute mapping and the working-hours
  derivation.

### Added — authz: ABAC policy authorization inside the blanket guard

- ABAC authorization landed (spec §13 T-1c, the authorization sub-item
  — supersedes the earlier roles/RBAC-on-`roles`/`scope` sketch;
  family contract: `agents/share/authorization-attributes.md`). When
  `PERSON_REQUIRE_AUTH` is on, a verified PASETO token is further
  checked by the shared policy engine in `authentication-verifier`
  0.3: the request's action is derived from the HTTP method plus the
  crate's destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES`
  — `/merge`, `/deduplicate`, `/import`), and the policy is evaluated
  over the token's new `attrs` claim, first-match-wins, defaulting to
  allow-read / deny-mutation.
- New env vars `PERSON_ABAC_POLICY` (inline JSON) and
  `PERSON_ABAC_POLICY_FILE` (path), read once at router construction
  by the new `auth::policy_from_env` (restart to change); unset or
  unparsable ⇒ `tracing::warn!` + the built-in default policy
  (`svc=true` ⇒ everything; `access=admin` ⇒ destructive+write;
  `access=write` ⇒ write) — the service always boots.
- `auth::enforce` now takes the HTTP method and the policy and
  returns `403` (with the deciding-rule reason) for a valid token the
  policy denies; `401` remains missing/bad credential. `Enforcement`
  carries the policy alongside the verifier.
- DB-free unit tests pin the family §7 matrix: action derivation,
  empty-`attrs` read-only default, `access=write` / `access=admin` /
  `svc=true` tiers, deny-beats-later-allow, 401-vs-403, bad-policy
  fallback.
- Flag off ⇒ behaviour-neutral: no authn and no authz, exactly as
  before.

### Added — boot-time PASETO key-set fetch (`PERSON_PASETO_KEYS_URL`)

- New `PERSON_PASETO_KEYS_URL` env var (spec §13 T-1c fetch item): when
  set, the auth-service published Ed25519 key set
  (`/.well-known/paseto-keys`) is fetched **once at boot** via
  `Verifier::from_paseto_keys_url` (the `authentication-verifier`
  `fetch` feature, now enabled in Cargo.toml). On success the fetched
  key set **wins** over `PERSON_PASETO_KEYS` (logged at `info`); on any
  fetch failure (network / HTTP / parse) a `warn` is logged and the
  verifier falls back to the `PERSON_PASETO_KEYS` env path — the
  service **always boots**; auth-service downtime never prevents
  startup. Unset/blank URL ⇒ prior behaviour exactly. One-shot fetch —
  no refresh loop (periodic refresh is a spec §15 roadmap note).
- Wired in the loco `after_routes` hook: the verifier is resolved
  (`state::verifier_from_env_or_fetch`) and swapped into `AppState` via
  `with_verifier` **before** the enforcement middleware and the
  shared-store state are built, so both router surfaces (the
  enforcement layer and the `AuthUser` extractor) verify against the
  fetched key set. Issuer/audience still come from
  `PERSON_TOKEN_ISSUER` / `PERSON_TOKEN_AUDIENCE` (same defaults).
- New DB-free tokio tests in `src/api/rest/auth.rs` (reusing the
  in-process PASETO minting helpers): a local ephemeral-port HTTP
  listener serves the key set and a token signed by that key verifies;
  a dead port falls back to the env path without panicking; URL-unset
  uses the env path (precedence).
- Authorization has since landed as ABAC (see the top entry), not
  RBAC — the spec §13 T-1c authorization item is complete.

### Added — blanket auth enforcement (default-off)

- New blanket `/api/*` auth enforcement middleware (spec §13 T-1b; family
  contract: `agents/share/jwt-enforcement.md`). When `PERSON_REQUIRE_AUTH`
  is truthy (`1`/`true`/`yes`/`on`, case-insensitive; unset/blank/junk ⇒
  **off**, the default), every route requires a valid PASETO `v4.public`
  bearer token except the public allow-list: `/api/health`, loco's
  `/_health` / `/_ping`, `/api-docs/openapi.json`, `/swagger-ui*`, and
  `/metrics.prom`. Unauthorised requests get `401`.
- Implemented as a pure, DB-free `auth::enforce(flag, path, headers,
  verifier)` decision plus an `Enforcement` middleware state in
  `src/api/rest/auth.rs`, layered unconditionally on **both** router
  surfaces (`create_router` and the loco `after_routes` hook). The flag
  is snapshotted at router construction — changing the env var requires
  a restart; the flag is the only switch.
- New DB-free unit tests pin the full enforcement matrix (off + no
  token ⇒ Ok; on + each public path ⇒ Ok; on + protected + no token ⇒
  `401`; on + valid ⇒ Ok; on + expired/tampered ⇒ `401`) and the lenient
  flag-parser semantics, reusing the in-process PASETO minting helpers.
- Boot-time HTTP key fetch has since landed (see the entry above);
  authorization has since landed as ABAC (top entry), completing
  spec §13 T-1c.

### Changed — auth pivot: RS256 JWT/JWKS → PASETO v4.public

- Bearer-token verification migrated off RS256 JWT + JWKS to **PASETO
  `v4.public`** (Ed25519), per the family-wide design in
  `agents/share/authentication-sessions.md` (§5, §9 step 4; spec §13
  T-1a). The `AuthUser` extractor and `GET /api/whoami` are unchanged
  in shape; only the credential changes.
- `authentication-verifier` bumped from the crates.io `0.1` (RS256)
  release to the monorepo path dependency `0.2` (PASETO-only:
  `Verifier::from_paseto_keys_value`); the direct `jsonwebtoken`
  dependency is dropped.
- The verifier is now built from the environment at boot:
  `PERSON_PASETO_KEYS` (the Ed25519 key set the auth service publishes
  at `/.well-known/paseto-keys`), `PERSON_TOKEN_ISSUER` (default
  `authentication-service`), `PERSON_TOKEN_AUDIENCE` (default
  `main-x-service`). Absent/blank/unparseable key set ⇒ empty key set:
  every token is rejected but the service still boots.
- New DB-free unit tests in `src/api/rest/auth.rs` mint `v4.public`
  tokens in-process (throwaway Ed25519 key via `rusty_paseto` +
  `ed25519-dalek` dev-deps) and pin valid / missing / non-bearer /
  expired / tampered / no-key outcomes.

### Fixed — privacy masking UTF-8 safety

- `privacy::mask_value` is now char-based instead of byte-based. The
  previous implementation sliced the string at byte offset
  `len - visible_chars`; when that offset fell inside a multibyte UTF-8
  character it **panicked** (`end byte index … is not a char boundary`),
  so the masked-view endpoint (`GET /api/persons/{id}/masked`) would
  500 on any person whose tax ID, identifier, document number, or phone
  carried a non-ASCII character near the tail (accented names, non-Latin
  identifiers). Masking now counts Unicode scalar values and keeps
  exactly the last four *characters* visible. Pinned by
  `privacy::tests::test_mask_value_multibyte_does_not_panic`; the
  contract is recorded in spec §6.6.

### Added — matcher bridge

- New `src/matching/adapter.rs` exposing
  `to_matcher_person(&service::Person) -> person_matcher::Person`.
  Projects the FHIR/schema.org-shaped service record into the matcher
  crate's builder shape: name flattening (`HumanName` → flat
  `given_name`/`family_name`/`middle_name`), telecom sampling
  (first phone / sms / email of each system),
  identifier routing by FHIR-style `system` URI (UK NHS via `https://fhir.nhs.uk/Id/nhs-number` → `uk_nhs_number`, US SSN, 40+ country slots
  with type-based fallbacks), and address field
  renaming (`state` → `county`, `postal_code` → `postcode`).
- `src/matching/mod.rs` now re-exports the sibling `person-matcher` crate
  as `matcher_lib`, so callers can reach `MatchingEngine`,
  `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`, and
  every public matcher type without taking a separate dependency.
- Field-routing rules are inline-documented in `adapter.rs` and
  pinned by `tests/duplicate_detection.rs`.

### Added — tests

- New `tests/duplicate_detection.rs`. Black-box bridge tests that
  drive service records through `to_matcher_person` and assert on
  the canonical `MatchingEngine::match_persons` output. Covers
  identical clones, name typos (Jaro-Winkler), deterministic
  short-circuits (national / strong identifiers), negative cases
  (unrelated records, divergent demographics), per-adapter field
  routing, and config-preset invariants (strict ⊆ lenient).

### Added — bridge benchmarks

- New `benches/bridge_bench.rs` (Criterion). Three groups:
  `bridge_adapter_only` (projection cost on minimal vs. rich
  records), `bridge_end_to_end` (adapter + engine call), and
  `bridge_one_to_many` (single query vs. 10 / 50 / 100 candidates).
  Regression guard for the duplicate-check hot path.

### Added — observability

- New `src/metrics.rs` exposing a process-wide `LazyLock<Metrics>`
  Prometheus registry. Standard counters
  (`person_created_total` / `_updated_total` / `_deleted_total` /
  `_matched_total`, labeled `http_requests_total`) and histograms
  (`http_request_duration_seconds`, `person_match_score`,
  `person_search_duration_seconds`).
- New `GET /metrics.prom` route on the web router serving Prometheus
  text-exposition format (`text/plain; version=0.0.4`). The
  canonical `/metrics` continues to render the HTML dashboard;
  configure scrapers with `metrics_path: /metrics.prom`.

### Added — UI

- `assets/static/css/themes/` ships 39 standalone Lily Design System
  themes (light, dark, dracula, nord, cyberpunk, … + four
  United Kingdom NHS variants). The layout's theme picker now lists
  all 39; default is `light`. Selection swaps the `<link href>` of
  `<link id="lily-theme-css">` at runtime; persisted in
  `localStorage["lily-theme"]`. The command palette also lists all
  39 themes.

### Changed — Loco background jobs

- Dropped the `bg_redis` and `bg_sqlt` features from the `loco-rs`
  dependency. Background jobs are now backed exclusively by
  PostgreSQL (`bg_pg`), using the same database as application data
  — no external Redis broker. `config/development.yaml` and
  `config/production.yaml` updated to `queue.kind: Postgres` with
  `uri: DATABASE_URL`. Removes the `rusty-sidekiq` →
  `redis 0.22.3` future-incompat warning chain.

### Changed — documentation

- Reduced healthcare / clinical / patient / hospital / clinician /
  practitioner framing across spec.md, AGENTS.md, AGENTS/*, README,
  CLAUDE.md, and index.md. Preserved: FHIR R5 resource and field
  names (e.g. `Patient.birthPlace`, `Practitioner` resource),
  national-identifier proper nouns (United Kingdom National Health
  Service Number, Australia IHI), paper citations, the
  `compliance-for-healthcare.md` doc, and `HIPAA` / `NHS` / `PHI`
  as compliance regimes.
- `spec.md §11 Testing Strategy` now lists the bridge integration
  tests; `AGENTS/testing.md` gained a `## Bridge Integration Tests`
  section; `AGENTS/restful.md` gained adapter + Prometheus blocks;
  `index.md` gained a worked example showing the canonical bridge
  in action.

### Fixed

- The person-matcher crates.io 0.3.0 API drift (Sweden personnummer
  renamed from `se_personnummer` to `se_workernummer`,
  `united_kingdom_national_health_service_number` shortened to
  `uk_nhs_number`) is now caught at the matcher level by each
  matcher's `tests/adapter_contract.rs` — see the matcher
  CHANGELOG.
