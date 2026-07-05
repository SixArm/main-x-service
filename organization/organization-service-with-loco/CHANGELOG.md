# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

- **Boot-time PASETO key-set fetch over HTTP.** New env var
  `ORGANIZATION_PASETO_KEYS_URL`: when set, the service fetches the
  auth-service's published Ed25519 key set once at boot
  (`Verifier::from_paseto_keys_url`, `authentication-verifier` `fetch`
  feature) from `App::after_routes` via the new `auth::init_from_env`,
  seeding the process-wide verifier before serving. The fetched key set
  wins over `ORGANIZATION_PASETO_KEYS` (`tracing::info!`); any fetch
  failure logs a warning and falls back to the env key set, so the
  service always boots. Unset/blank URL keeps the prior env-injection
  behaviour exactly. Fetch-once only — a periodic refresh loop on key
  rotation is tracked as a future spec item (spec §16). Tests: a local
  ephemeral-port HTTP listener serving the test key set (the fetch-built
  verifier accepts a token signed by that key), a fast-failing-URL
  fallback pin (no panic), and a no-URL env-path pin. (Spec §7 env
  table + §13 fetch follow-up.)

### Fixed

- **`cargo fmt` drift.** Reformatted `src/auth.rs` and
  `tests/requests/organizations.rs` so `cargo fmt --check` passes again
  (no behavioural change).

### Changed

- **Auth pivot — sessions + PASETO (spec-level; code follow-up pending).**
  The family is moving off RS256 JWT + JWKS access tokens to server-side
  cookie sessions plus short-lived **PASETO v4.public** tokens verified
  offline against the authentication-service's published **Ed25519** key;
  the `authentication-verifier` becomes a PASETO verifier and RS256/JWKS
  is decommissioned. Front-ends adopt a BFF + httpOnly cookie + CSRF (the
  browser holds no token). The `ORGANIZATION_REQUIRE_AUTH` flag and
  blanket-enforcement semantics are unchanged — only the verified
  credential changes. Human-facing docs (README/AGENTS/index) updated to
  describe the new model; runtime code follow-up is tracked in spec §13.
  Source of truth:
  [agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md).

### Added

- **Doc/test harmonization pass.** Request-level tests added for the
  audit endpoints (`/audit/recent` + `/{pid}/audit` record CRUD actions;
  invalid pid ⇒ `400`) and for the plain-CRUD `created`/`updated`/
  `deleted` events on `/events/recent` (frozen `EventView` projection).
  `index.md` worked-flow extended to cover search / merge / audit /
  events / whoami / metrics, with a worked merge example; `README.md`
  gained worked merge, `whoami`, and `/metrics.prom` examples and a
  corrected Status section. `AGENTS.md` deferred list corrected (blanket
  `/api/*` enforcement is implemented; only JWKS-over-HTTP fetch
  remains). Crate `spec/index.md` and `AGENTS.md` now cross-link the
  entity umbrella spec (`../spec/index.md`) where the `R-DUP`/`T-7`/
  `T-9`/`T-12` task IDs the source comments cite are defined. Umbrella
  spec §13 T-9 follow-up marked blanket-enforcement done; cap-boundary
  truncation test and request-level whoami-200 test recorded as open
  tasks.

- **Prometheus metrics — `GET /metrics.prom`.** A new process-wide
  `prometheus::Registry` (`src/metrics.rs`, behind a `OnceLock` reached
  via `Metrics::global()`, mirroring `auth::verifier`) is served at the
  application **root** path `/metrics.prom` in text-exposition format
  (`text/plain; version=0.0.4`) by a loco controller
  (`src/controllers/metrics.rs`, mounted at root like the docs). The
  metric set: `organization_created_total`, `organization_updated_total`,
  `organization_deleted_total`, `organization_merged_total` (plain
  counters, incremented one per success path in the CRUD/merge
  controller handlers) plus a labelled `http_requests_total`
  (`path`/`status`) declared for a future request middleware. The path
  is added to `auth::is_public_path`, so it stays public under blanket
  JWT enforcement (no bearer token needed to scrape). New DB-free tests:
  registry render + counter increment (`metrics::tests`), the
  `/metrics.prom` OpenAPI path (`openapi::tests`), and `/metrics.prom`
  in the `enforce` public-path matrix (`auth::tests`). Brings parity
  with the older Axum services, which already expose Prometheus metrics.
- **Durable event bus — Phase 1 (in-memory envelope + `EventPublisher`
  seam).** `src/streaming.rs` now carries the canonical versioned
  `Envelope` (`event_id` UUID dedup key, `schema_version` = 1,
  `entity` = `"organization"`, `kind`, `pid`, `seq`, `actor`, `name`),
  an `EventPublisher` trait, and an `InMemoryPublisher` ring buffer
  (process-wide `OnceLock`) implementing it — replacing the flat
  `OrgEvent` free-function buffer. `occurred_at`/`data` are deferred to
  the outbox stage (Phase 2; the in-memory envelope is kept minimal).
  CRUD/merge call
  sites now stamp the bearer `actor` via `publish_with_actor`
  (`publish` kept as a `None`-actor shim). Pure refactor: behaviour is
  identical and the `GET /api/organizations/events/recent` wire shape is
  **frozen** — it returns the flat `EventView { kind, pid, name, seq }`
  projection of the envelope, byte-identical to before (front-end safe).
  Phases 2–3 (transactional outbox + Fluvio relay) remain infra-gated
  roadmap per [`agents/share/event-bus.md`](../../agents/share/event-bus.md).
- **Blanket `/api/*` JWT enforcement (default-off).** A new
  `ORGANIZATION_REQUIRE_AUTH` env flag (lenient bool — `1`/`true`/`yes`/
  `on`) gates an `axum::middleware::from_fn` layer wired in
  `App::after_routes`. When on, every route except the public health/ping
  + OpenAPI/Swagger paths requires a valid bearer token (`401`
  otherwise); when off (the default) behaviour is unchanged. The decision
  is the pure, unit-tested `auth::enforce` (plus `auth::require_auth`,
  `auth::parse_bool`, `is_public_path`). New `auth::tests` cover the
  matrix; a `#[serial]`/`#[ignore]` request test pins un-authed `GET
  /api/organizations` ⇒ `401` with the public OpenAPI doc still `200`.
  Implements the family contract in `agents/share/jwt-enforcement.md`.
- **Request-level integration tests.** `tests/requests/organizations.rs`
  (loco testing harness + `serial_test`): create round-trip
  (snake_case wire), blank-name `422` on create + update, unknown-pid
  `404`, search (+ blank-`q` `400`), check-duplicates ranking.
  `#[ignore]`-gated so the default `cargo test` stays green without
  Postgres; run with `cargo test -- --ignored`.

### Changed

- **Validation failures now return `422 Unprocessable Entity`** (was
  `400`): blank `name` on create and on replace (`PUT`), per the
  family convention. A DB-free unit test pins the mapping; OpenAPI
  updated.
- **Unknown `pid` now returns `404`** on get/replace/delete (loco's
  default `ModelError::EntityNotFound` mapping produced a `500`,
  breaking the documented contract).
- Docs (`README.md`, `index.md`, `AGENTS.md`) now describe the wire
  format as snake_case (`legal_name`, `same_as`, `founding_date`, …)
  matching the actual DTO serialization — entity spec OQ-1 resolved:
  no serde rename; snake_case is canonical.

### Removed

- loco scaffolding leftovers: `src/workers/downloader.rs` (TODO stub)
  and its worker registration, plus the empty `src/data/` and
  `src/tasks/` modules.

- **Audit log + event streaming.** `audit_logs` table records every
  create/update/delete (with a JSONB snapshot); a process-global
  in-memory event stream publishes Created/Updated/Deleted events.
  Endpoints: `GET /api/organizations/audit/recent`, `/{pid}/audit`,
  `/events/recent`.
- **Name search.** `GET /api/organizations/search?q=` — case-insensitive
  Postgres `ILIKE` on the denormalised name (Tantivy full-text remains a
  §13 follow-up).
- **OpenAPI + Swagger UI.** Hand-authored OpenAPI 3 spec at
  `/api-docs/openapi.json` (accurately typed `Organization` schema, since
  the matcher crate is `utoipa`-free) and a Swagger UI page at
  `/swagger-ui`.

- **Inaugural scaffold (v0.1.0).** loco.rs organization-identity
  registry (schema.org/Organization).
  - Generated via `loco new` (loco-rs 0.16) and stripped of the auth
    starter; auth is centralized in the authentication-service.
  - `organizations` table (`pid`, denormalised `name`, full
    `Organization` payload as JSONB `data`, `active`, soft-delete) +
    `sea-orm-migration` migrator.
  - CRUD controller: create / list / get / update / soft-delete, plus
    `POST /match` (rank a `{query, candidates}` set) and
    `POST /check-duplicates` (match a query against stored records).
  - **Embeds `organization-matcher` directly**: the API DTO *is*
    `organization_matcher::Organization`, stored verbatim and matched
    with the canonical engine — no separate model or adapter.
  - DB-free tests (`tests/matching.rs`): matcher embedding + JSON
    storage round-trip. Green `cargo build`, clippy clean.

### Notes

- The inaugural v0.1.0 scope was CRUD + matching; the entries above
  (in this Unreleased section) extend it with name search (`ILIKE`),
  audit + event streaming, record merge, OpenAPI/Swagger, Prometheus
  metrics, JWT verification (+ default-off blanket enforcement), and
  request-level tests. Still deferred (spec §13): Tantivy full-text
  search, per-field privacy/GDPR export, JWKS-over-HTTP fetch at boot,
  and richer validation.
