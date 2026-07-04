# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Fixed

- **`cargo fmt` drift.** Reformatted `src/auth.rs` and
  `src/validation.rs` so `cargo fmt --check` passes again (no
  behavioural change).

### Changed

- **Auth pivot — sessions + PASETO (spec-level; code follow-up pending).**
  The family is moving off RS256 JWT + JWKS access tokens to server-side
  cookie sessions plus short-lived **PASETO v4.public** tokens verified
  offline against the authentication-service's published **Ed25519** key;
  the `authentication-verifier` becomes a PASETO verifier and RS256/JWKS
  is decommissioned. Front-ends adopt a BFF + httpOnly cookie + CSRF (the
  browser holds no token). The `CARE_PATHWAY_REQUIRE_AUTH` flag and
  blanket-enforcement semantics are unchanged — only the verified
  credential changes. Human-facing docs (README/AGENTS/index) updated to
  describe the new model; runtime code follow-up is tracked in spec §13.
  Source of truth:
  [agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md).

### Documentation

- **Merge request-body field-name harmonization + worked examples.**
  Fixed the `README.md` Quick-start merge `curl` (was the unrecognized
  `survivor_pid`; now `main_pid`/`duplicate_pid`, matching the controller
  `MergeRequest` and the OpenAPI schema) and the `index.md` worked-flow
  merge row (was `{survivor_pid, dup_pid}`; now `{main_pid,
  duplicate_pid}`). Added a `README.md` multi-problem `422` example and
  an `Authorization: Bearer` / `whoami` example, and an `index.md`
  auth + `CARE_PATHWAY_REQUIRE_AUTH` note plus a cross-reference to the
  un-gated multi-dimension aggregation test. Reworded spec §15 so the
  roadmap reflects that all of the v0.1–v0.3 scope shipped together in
  the still-unreleased `0.1.0` line (the milestone split was never
  tagged).

### Tested

- **Self-merge `422` guard pinned DB-free.** Extracted the merge
  handler's equal-pid check into a pure `is_self_merge(main, dup)`
  predicate and added an un-gated unit test, so the §6.8 self-merge
  rejection holds on the default `cargo test` (previously covered only by
  the `#[ignore]`-gated `merge_with_equal_pids_is_422` request test).
- **Unknown-pid `404` on update + delete.** Added `#[ignore]`-gated
  request tests `update_unknown_pid_returns_404` and
  `delete_unknown_pid_returns_404`, closing the gap where only GET (and
  merge) had a `404` request test.
- **CI now runs the DB-backed request suite.** The `test` job gained a
  dedicated `cargo test --all-features --all -- --ignored` step against
  the already-provisioned Postgres service (the prior single step never
  passed `--ignored`, so every request-level test was silently skipped).
  Also removed a duplicate `- main` push branch in the workflow.

- **Doc harmonization pass (spec is the source of truth).** Refreshed
  the stale `README.md` Status section (now lists CRUD + `ILIKE` search +
  matching + merge + audit + in-memory streaming + OpenAPI/Swagger +
  Prometheus + offline JWT verification + blanket `/api/*` enforcement
  off-by-default as implemented, with only Tantivy full-text, durable
  event bus Phases 2–3, privacy, front-end merge action, and
  JWKS-over-HTTP fetch deferred) and the validation note (now covers
  ICD/SNOMED/UUID/DOI/BCP-47, all problems reported together). Corrected
  the `AGENTS.md` deferred list so blanket `/api/*` JWT enforcement is
  shown as implemented (off by default via `CARE_PATHWAY_REQUIRE_AUTH`)
  and only JWKS-over-HTTP fetch at boot remains deferred. Added a
  §6.12/§9 cross-reference for the `/metrics.prom` public path in the
  spec. Expanded `index.md`'s worked flow with merge / merges / audit /
  events / whoami / docs / metrics examples and a validation note.

### Tested

- **`validation::problems` multi-dimension aggregation pin.** Added a
  DB-free test asserting that a blank `name`, a malformed
  `condition_codes` entry, a malformed `identifiers` entry, and a
  malformed `in_language` tag each surface as a distinct problem in one
  call — pinning the §6.1 "all problems reported together" guarantee
  across every validated dimension at once.

### Added

- **Prometheus `/metrics.prom` endpoint.** A root-level
  `GET /metrics.prom` (Content-Type `text/plain; version=0.0.4`) for
  parity with the older Axum services. `src/metrics.rs` owns a
  process-wide `OnceLock<Metrics>` Prometheus `Registry` with four
  care-pathway counters (`care_pathway_created_total`,
  `_updated_total`, `_deleted_total`, `_merged_total`) plus an
  `http_requests_total` `IntCounterVec` (`method`, `path`, `status`);
  `Metrics::global()` and `Metrics::render()` (TextEncoder →
  text-exposition). The handler lives in `src/controllers/metrics.rs`
  and is mounted at the root via `App::routes` (mirroring
  `controllers/docs.rs`). The path is added to `auth::is_public_path`,
  so it stays open under blanket JWT enforcement (a scraper needs no
  token). The CRUD/merge controllers increment one counter per success
  path (create→created, update→updated, delete→deleted, merge→merged).
  New dependency `prometheus = "0.13"`. Un-gated tests: a DB-free
  `metrics` render test (every metric name + `# HELP`/`# TYPE` preamble +
  content type), an `auth::enforce` public-path test for `/metrics.prom`,
  and an `openapi` test for the documented `/metrics.prom` path.

- **Durable event bus — Phase 1 (in-memory envelope + `EventPublisher`
  seam).** `src/streaming.rs` now carries the canonical, versioned
  `Envelope` (`event_id` UUID dedup key, `schema_version` 1, `entity`
  `"care_pathway"`, `kind`, `pid`, `seq`, `actor`, `name`) and the
  `EventPublisher` trait, with an `InMemoryPublisher` ring buffer wired as
  the process-wide global — a pure refactor of the previous free
  functions. `occurred_at` / `data` are deferred to the outbox stage
  (Phase 2) per `agents/share/event-bus.md`; no new dependency added.
  `GET /api/care-pathways/events/recent` returns the frozen `EventView`
  projection (`{kind, pid, name, seq}`), **byte-identical** to the
  previous wire shape (the front-end recent-activity view depends on it).
  Added `publish_with_actor(kind, pid, name, actor)`; the CRUD/merge
  controller call sites now stamp the `actor` from the bearer token (the
  bare `publish` back-compat surface stays, actor `None`). Phases 2–3
  (transactional outbox → Fluvio) remain infra-gated roadmap. Un-gated
  tests: envelope Serde round-trip + `schema_version == 1`, `EventView`
  projects exactly `{kind, pid, name, seq}`, `InMemoryPublisher`
  publish→recent, `actor` populated/`None`, `seq` monotonic.

- **Blanket `/api/*` JWT enforcement (off by default).** A pure
  `auth::enforce(require_auth, path, headers, verifier)` decision plus an
  `axum::middleware::from_fn` layer wired unconditionally in `app.rs`
  `after_routes`. Gated per-request by `CARE_PATHWAY_REQUIRE_AUTH`
  (`auth::require_auth`, `OnceLock<bool>`; `1`/`true`/`yes`/`on` ⇒ on,
  anything else incl. unset ⇒ off). When on, every `/api/*` route needs a
  valid bearer token (`401` otherwise); the public paths `/_health`,
  `/_ping`, `/api-docs/openapi.json`, `/swagger-ui*` stay open. Default-off
  keeps existing behaviour and the DB-gated request suite green until an
  operator opts in. Un-gated `auth::tests` cover `parse_bool` and
  `enforce` (off/public/protected × no/valid/expired/tampered token); a
  `#[serial]` `#[ignore]` request test asserts `401` on `GET
  /api/care-pathways` and `200` on `GET /api-docs/openapi.json` with the
  flag set. Family contract: `agents/share/jwt-enforcement.md`.

### Changed

- **Validation failures now return `422 Unprocessable Entity`**
  (was `400`) for a blank `name`, on both create and update — the
  family convention (entity spec OQ-1 / T-2). Implemented as a shared
  controller `validate()` returning
  `Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`; pinned
  by DB-free unit tests.

### Added

- **`identifiers` and `in_language` payload validation** in
  `src/validation.rs`: each `identifiers` entry is structurally checked
  against its `scheme` — a canonical 8-4-4-4-12 hex UUID for `Uuid`, the
  `10.<registrant>/<suffix>` shape for `Doi`, and non-blank for every
  other scheme — and each `in_language` entry is checked for BCP-47
  syntax. A malformed entry joins the existing single `422` (all
  problems reported together). Rejecting a malformed *deterministic*
  identifier (UUID / DOI) matters because a shared value short-circuits
  the matcher to `1.0`. Pinned by 6 new DB-free `validation` unit tests
  and the DB-gated request test
  `malformed_identifier_on_create_returns_422`.

- Request-level integration tests
  (`tests/requests/care_pathways.rs`, loco testing harness) covering
  all seven endpoints: create, blank-name `422` on create/update,
  get-by-pid `200`/`404`, list, `/match`, and a stored near-duplicate
  `/check-duplicates` round-trip. `#[ignore]`-gated — they need a
  PostgreSQL `DATABASE_URL`; run with `cargo test -- --ignored`.

- **Inaugural scaffold (v0.1.0).** loco.rs clinical care-pathway
  registry.
  - Generated via `loco new` (loco-rs 0.16) and stripped of the auth
    starter (auth is centralized in the authentication-service).
  - `care_pathways` table (`pid`, denormalised `name`, full
    `CarePathway` payload as JSONB `data`, `active`, soft-delete) +
    `sea-orm-migration` migrator.
  - CRUD controller: create / list / get / update / soft-delete, plus
    `POST /match` and `POST /check-duplicates`.
  - **Embeds `care-pathway-matcher` directly**: the API DTO *is*
    `care_pathway_matcher::CarePathway`, stored verbatim and matched
    with the canonical engine — no separate model or adapter.
  - DB-free tests (`tests/matching.rs`): matcher embedding + JSON
    storage round-trip. Green `cargo build`, clippy clean.

### Notes

- MVP scope is CRUD + matching. Search, streaming, audit, privacy,
  OpenAPI, and richer validation are tracked in spec §13.
