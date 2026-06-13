# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

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
