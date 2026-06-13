# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

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

- MVP scope is CRUD + matching. Search (Tantivy), streaming, audit,
  privacy/GDPR, OpenAPI, richer validation, and request-level tests are
  tracked in spec §13.
