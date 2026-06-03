# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- **Live-service integration test suite** at `tests/integration/golden-paths.spec.ts`. 8 Playwright tests driving the live SvelteKit preview against a running `person-service-rust-crate` over real HTTP. Covers spec §6 FR-1 (search-finds-record), FR-3 (create lands on detail page) + the 409-duplicate inline path, FR-5 (detail renders nested fields), FR-6 (edit PUTs the full Person), FR-7 (soft-delete hides record), FR-8 (match check renders score), FR-9 (merge soft-deletes the duplicate), and per-record audit log presence.
- `playwright.config.ts` refactored to two projects: `smoke` (existing, no service needed; `tests/e2e/`) and `integration` (new, live service required; `tests/integration/`). The webServer command now bakes `PUBLIC_API_BASE_URL` into the preview build so the front-end talks to the configured service.
- `bin/e2e` wrapper script that health-checks the service at `PUBLIC_API_BASE_URL/api/health` before running Playwright. Forwards extra args (`--headed`, `--ui`, etc.) to playwright.
- `package.json` adds `test:integration` script. `test:e2e` now scoped to the `smoke` project.

### Documentation

- `spec.md §11 Testing Strategy` extended with a 3-row layer table (unit / smoke / integration) plus run commands and a section on test idempotency.
- `spec.md §13 Tasks` marks T-12a integration suite as complete with harness-level validation (svelte-check clean, playwright `--list` discovers all 9 tests, smoke project still 6/6, `bin/e2e` exits 1 with a clear message when the service is down).
- `spec.md §16 OQ-5` records the live-stack validation blocker: the `person-service-rust-crate` Dockerfile build fails (missing apt packages in debian:bookworm-slim) and `cargo run --release` fails (no `[[bin]]` target, no `src/main.rs`) even though `target/release/person_service` is referenced by the Dockerfile. The service crate needs a `main.rs` wiring `api::rest::serve` to a runnable binary before live integration can be exercised end-to-end.

### Validation status

| Stage | Result |
| --- | --- |
| `svelte-check` | ✅ 0 errors, 0 warnings (352 files) |
| `pnpm test` (vitest unit) | ✅ 8/8 |
| `pnpm test:e2e` (smoke; refactored config) | ✅ 6/6 |
| `pnpm exec playwright test --project=integration --list` | ✅ 9 tests discovered |
| `bin/e2e` with no service running | ✅ exits 1, prints the bring-up instructions |
| `bin/e2e` against a live service | ⛔ blocked on OQ-5 — service crate has no runnable binary today |

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
