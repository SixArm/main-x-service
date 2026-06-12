# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

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
  OpenAPI, richer validation, and request-level tests are tracked in
  spec §13.
