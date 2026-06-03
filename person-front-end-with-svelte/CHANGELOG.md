# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

Nothing yet.

## [0.1.0] — 2026-06-02

Initial scaffold for the Person Service front-end. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless.

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; persons list with full-text / fuzzy / phonetic search and SVAR DataGrid; create with real-time 409 duplicate detection inline; detail view (identity, identifiers, addresses, telecom, emergency contacts); edit; soft-delete with confirm; per-record audit log; match check; merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError` with `isConflict` / `isNotFound` / `isValidation` shortcuts; `PersonRepository` binding the [Person Service REST surface](../person-service-rust-crate/AGENTS/restful.md) (`GET /api/health`, CRUD on `/api/persons`, `/search`, `/match`, `/check-duplicates`, `/merge`, `/deduplicate`, `/{id}/audit`, `/{id}/masked`, `/{id}/export`, `/api/audit/recent`).
- **TypeScript types.** Snake-case domain types mirroring [`person-service-rust-crate/AGENTS/models.md`](../person-service-rust-crate/AGENTS/models.md): `Person`, `HumanName`, `Address`, `ContactPoint`, `Identifier` (MRN/SSN/DL/NPI/PPN/TAX), `IdentityDocument` (Passport/National-ID/etc.), `EmergencyContact`, `PersonLink`, `MatchResult` + `MatchQuality` + `MatchBreakdown`, `MergeRequest` / `MergeRecord` / `MergeResponse`, `BatchDeduplicationRequest`/`Response`, `ReviewQueueItem`, `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store (`value` / `errors` / `submitting` / `submitError` / `submit()` / `reset()`).
- **Components.** `SearchBox`, `PersonGrid` (SVAR `Grid` with `select` mode and `init` callback subscribing to `select-row`), `HumanNameInput`, `PersonForm`, `MatchResultsList` with per-component breakdown disclosure.
- **Tests.** 5 Vitest unit tests for `ApiClient` envelope + error handling, 3 unit tests for `PersonRepository` wiring, 6 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`). Read via `import.meta.env` so vitest can load the module without the SvelteKit Vite plugin.
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`). The SVAR Grid is browser-only and would break SSR; this is a backend admin UI so SSR adds no value.

### Cross-references

- Service spec: [`../person-service-rust-crate/spec.md`](../person-service-rust-crate/spec.md).
- Service REST contract: [`../person-service-rust-crate/AGENTS/restful.md`](../person-service-rust-crate/AGENTS/restful.md).
- Service model types: [`../person-service-rust-crate/AGENTS/models.md`](../person-service-rust-crate/AGENTS/models.md).
- Service matching reference: [`../person-service-rust-crate/AGENTS/matching.md`](../person-service-rust-crate/AGENTS/matching.md).
