# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added — registry insight views (2026-07-20)

- Five read-only derived views (`controllers/insights.rs`): setting +
  `specialty:<x>` directory, condition-coverage gaps, cross-provider
  variants (with the `jurisdiction:<x>` facet), provider directory,
  and language coverage. No migration, no matcher change — facets from
  existing DTO fields + two disclosed keyword conventions.

### Fixed

- 2026-07-18 — **Order-dependent enforcement test** (QA-CP-FLAKE):
  `require_auth_gates_api_but_not_openapi` set
  `CARE_PATHWAY_REQUIRE_AUTH` inside the shared requests binary, but
  the flag's `OnceLock` was cached by whichever sibling test booted
  first — it only passed when it happened to run first. Moved to its
  own `tests/enforcement.rs` binary (the case / patient-flow
  pattern). Full DB-gated suite green vs Postgres 18.


### Fixed

- 2026-07-18 — **Unknown-pid reads returned 500, not 404.** loco 0.16's
  `IntoResponse` catch-all maps an unmapped `ModelError::EntityNotFound`
  to a 500, so `GET /…/{pid}` with an unknown pid crashed instead of
  404ing (the organization service was immune — its `http_err` helper
  already mapped it; the copy-adaptors dropped it). Controller lookups
  now route through a `model_not_found` mapping. Family-wide fix with
  per-crate request-test pins.


### Fixed

- 2026-07-18 — **Fresh-database `db migrate` failure.** The
  `…_000004_event_outbox` migration used the loco `create_table`
  helper, which pluralizes table names (`event_outbox` →
  `event_outboxes`); its own index DDL then failed and rolled back
  the entire fresh migrate (zero tables). Rewritten as explicit SQL
  creating exactly `event_outbox`; verified against a fresh
  Postgres 18 (all migrations apply, correct table names). Family-wide
  fix (case, care-pathway, organization, portfolio; patient-flow
  shipped with the explicit-SQL form).


### Security

- **SEC-G6: trailing slash can no longer downgrade a destructive POST.**
  `derive_action` classified `/merge` / `/deduplicate` / `/import` via
  `path.ends_with`, so a trailing slash (`POST …/merge/`) fell through to
  `Write` — a non-admin `access=write` caller could reach a destructive op.
  The path is now `trim_end_matches('/')`-normalised first. Test extended.

- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance every relay would **double-ship** the
  same rows. `drain_once` now runs in a transaction and `unpublished` claims
  rows with `FOR UPDATE SKIP LOCKED` (a second relay skips locked rows; the
  lock releases on commit). Delivery stays at-least-once (consumers dedupe on
  `event_id`).

### Security — input-size caps on payload validation (SEC-M1) (2026-07-13)

- `src/validation.rs` now rejects oversized `CarePathway` payloads before
  they are stored or matched. The matcher runs O(n·m) Jaro-Winkler and
  Jaccard over the payload's text fields and arrays, so an unbounded
  string or array is a CPU/memory DoS — amplified by the
  `check-duplicates` scan over every stored record. New named caps
  enforced in the `problems` entrypoint (collecting *all* over-cap
  problems, surfaced as one `422`): `MAX_TEXT_LEN = 1024` per single
  free-text field (`name`, `pathway_code`, `provider_id`,
  `provider_name`; counted in Unicode scalar values), `MAX_ARRAY_LEN =
  256` per array (`alternate_names`, `condition_codes`, `interventions`,
  `keywords`, `identifiers`, `same_as`, `in_language`), and
  `MAX_ITEM_LEN = 512` per string entry inside an array (including
  `condition_codes[i].code` and `identifiers[i].value`). Messages such as
  `"name: exceeds 1024 characters"`, `"keywords: exceeds 256 entries"`,
  `"keywords[3]: exceeds 512 characters"`. All existing format checks are
  unchanged. New DB-free unit tests cover an oversized field, array,
  and array entry (one problem each) plus a large-but-within-caps record
  (zero problems).

### Changed — event bus: audit now joins the outbox transaction (2026-07-08)

- Under the `outbox` transport, the `audit_logs` write now rides the
  **same transaction** as the entity mutation and its `event_outbox` row
  (`agents/share/event-bus.md` §3 — the three "can never disagree"). It
  was previously a best-effort side channel written *after* the
  transaction committed, so a crash or audit failure could leave a
  committed change + event with no audit row. `AuditModel::record` is now
  generic over `ConnectionTrait`; the `create/update/delete/merge_and_emit`
  functions own the audit write (strict/in-txn under `outbox`, best-effort
  logged under `memory`), and the controllers no longer audit separately.
  New DB-gated `tests/outbox_audit.rs` drives `create_and_emit` under
  `outbox` and asserts entity + event + audit all commit together.
  (The `merge_records` history row stays a best-effort side channel — it
  is merge metadata, not the §3 audit trail.)

### Added — event bus: transactional-outbox storage (Phase 2 start) (2026-07-06)

- New `event_outbox` table (migration `…_000004_event_outbox`) + SeaORM
  entity + `models::event_outbox` — the durable hand-off buffer for the
  event bus (`agents/share/event-bus.md` §3). This crate is the family
  reference for the Phase-2 storage layer. Pieces:
  - `OutboxInsert::from_envelope` — the **pure** envelope→row mapping
    (pid parse, kind token, full-envelope JSONB payload, `occurred_at`
    stamp), DB-free unit-tested.
  - `Model::enqueue` — generic over `ConnectionTrait`, so a request
    handler can pass its own `&DatabaseTransaction` and give the entity
    write and the event one commit boundary.
  - `Model::unpublished` / `Model::mark_published` — the relay worker's
    poll (oldest-unpublished, id order) + ack (`published_at`).
  - Dedup unique index on `event_id`; partial index over unpublished rows.
  - Remaining (roadmap): the tx-aware `OutboxPublisher` behind the
    `EventPublisher` seam + handlers on an explicit transaction, then the
    Fluvio relay worker (Phase 3).

### Added — authz: ABAC policy authorization inside the blanket guard (2026-07-05)

- ABAC authorization landed (supersedes the earlier per-crate
  roles/RBAC sketch; family contract:
  `agents/share/authorization-attributes.md`). When
  `CARE_PATHWAY_REQUIRE_AUTH` is on, a verified PASETO token is
  further checked by the shared policy engine in
  `authentication-verifier` 0.3: the request's action is derived from
  the HTTP method plus the crate's destructive named POSTs
  (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`, `/deduplicate`,
  `/import`), and the policy is evaluated over the token's new `attrs`
  claim, first-match-wins, defaulting to allow-read / deny-mutation.
- New env vars `CARE_PATHWAY_ABAC_POLICY` (inline JSON) and
  `CARE_PATHWAY_ABAC_POLICY_FILE` (path); unset or unparsable ⇒
  `tracing::warn!` + the built-in default policy (`svc=true` ⇒
  everything; `access=admin` ⇒ destructive+write; `access=write` ⇒
  write) — the service always boots.
- `auth::enforce` now takes the HTTP method and the policy and returns
  `403` (deciding-rule reason) for a valid token the policy denies;
  `401` remains missing/bad credential. DB-free unit tests pin the
  family §7 matrix. Flag off ⇒ behaviour-neutral.

### Added

- **Boot-time PASETO key-set fetch over HTTP.** New env var
  `CARE_PATHWAY_PASETO_KEYS_URL`: when set, the service fetches the
  auth-service's published Ed25519 key set once at boot
  (`Verifier::from_paseto_keys_url`, `authentication-verifier` `fetch`
  feature) from `App::after_routes` via the new `auth::init_from_env`,
  seeding the process-wide verifier before serving. The fetched key set
  wins over `CARE_PATHWAY_PASETO_KEYS` (`tracing::info!`); any fetch
  failure logs a warning and falls back to the env key set, so the
  service always boots. Unset/blank URL keeps the prior env-injection
  behaviour exactly. Fetch-once only — a periodic refresh loop on key
  rotation is tracked as a future spec item (spec §16). Tests: a local
  ephemeral-port HTTP listener serving the test key set (the fetch-built
  verifier accepts a token signed by that key), a fast-failing-URL
  fallback pin (no panic), and a no-URL env-path pin. (Spec §9 auth
  section + §13 fetch follow-up.)

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
