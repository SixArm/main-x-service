# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

Nothing yet.

## [0.1.0] — 2026-06-02

Initial scaffold for the Worker Service front-end. Copy-adapted from `person-front-end-with-svelte`. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless.

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; workers list with full-text / fuzzy / phonetic search and SVAR DataGrid; create with real-time 409 duplicate detection inline; detail view (identity, identifiers, addresses, telecom, emergency contacts); edit; soft-delete with confirm; per-record audit log; match check; merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `WorkerRepository` binding the [Worker Service REST surface](../worker-service-rust-crate/AGENTS/restful.md) (`GET /api/health`, CRUD on `/api/workers`, `/search`, `/match`, `/check-duplicates`, `/merge`, `/deduplicate`, `/{id}/audit`, `/{id}/masked`, `/{id}/export`, `/api/audit/recent`).
- **TypeScript types.** Snake-case domain types mirroring [`worker-service-rust-crate/AGENTS/models.md`](../worker-service-rust-crate/AGENTS/models.md): `Worker`, `HumanName`, `Address`, `ContactPoint`, `Identifier` (MRN/SSN/DL/NPI/PPN/TAX), `IdentityDocument`, `EmergencyContact`, `WorkerLink`, `MatchResult`, `MergeRequest`/`Record`/`Response`, `BatchDeduplicationRequest`/`Response`, `ReviewQueueItem`, `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `WorkerGrid` (SVAR `Grid` with `select` mode + `init`/`select-row`), `HumanNameInput`, `WorkerForm`, `MatchResultsList`.
- **Tests.** 5 Vitest unit tests for `ApiClient`, 3 unit tests for `WorkerRepository`, 6 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../worker-service-rust-crate/spec.md`](../worker-service-rust-crate/spec.md).
- Service REST contract: [`../worker-service-rust-crate/AGENTS/restful.md`](../worker-service-rust-crate/AGENTS/restful.md).
- Service model types: [`../worker-service-rust-crate/AGENTS/models.md`](../worker-service-rust-crate/AGENTS/models.md).
- Service matching reference: [`../worker-service-rust-crate/AGENTS/matching.md`](../worker-service-rust-crate/AGENTS/matching.md).
