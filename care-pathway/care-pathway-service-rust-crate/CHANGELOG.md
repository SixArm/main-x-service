# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

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
