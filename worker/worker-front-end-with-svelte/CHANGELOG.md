# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- 2026-07-19 — SVAR moderate fit: new **/review** route (nav-linked): the batch-deduplication
  scan's review queue as SVAR Kanban columns (Pending / Confirmed /
  Rejected / AutoMerged). The scan runs only on the button (POST
  /deduplicate is destructive-classed, never a page-load side
  effect). Read-only: the service exposes no review-decision
  endpoint yet — that endpoint is the seam that would make the
  columns drag targets. +nav.review/review.run x 13 locales.
- 2026-07-19 — SVAR moderate fit: new **/expiry** route (nav-linked):
  credential/registration document expiry dates as all-day calendar
  events (read-only); selecting an entry opens the worker.
  +nav.expiry x 13.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: the workers index grid migrates from `wx-svelte-grid` to
  **@svar-ui/svelte-grid**, with a **@svar-ui/svelte-filter**
  FilterBar above it (client-side filtering). Legacy `wx-svelte-*`
  deps removed.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.

Nothing yet.

## [0.1.0] — 2026-06-02

Initial scaffold for the Worker Service front-end. Copy-adapted from `person-front-end-with-svelte`. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless.

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; workers list with full-text / fuzzy / phonetic search and SVAR DataGrid; create with real-time 409 duplicate detection inline; detail view (identity, identifiers, addresses, telecom, emergency contacts); edit; soft-delete with confirm; per-record audit log; match check; merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `WorkerRepository` binding the [Worker Service REST surface](../worker-service-with-loco/AGENTS/restful.md) (`GET /api/health`, CRUD on `/api/workers`, `/search`, `/match`, `/check-duplicates`, `/merge`, `/deduplicate`, `/{id}/audit`, `/{id}/masked`, `/{id}/export`, `/api/audit/recent`).
- **TypeScript types.** Snake-case domain types mirroring [`worker-service-with-loco/AGENTS/models.md`](../worker-service-with-loco/AGENTS/models.md): `Worker`, `HumanName`, `Address`, `ContactPoint`, `Identifier` (MRN/SSN/DL/NPI/PPN/TAX), `IdentityDocument`, `EmergencyContact`, `WorkerLink`, `MatchResult`, `MergeRequest`/`Record`/`Response`, `BatchDeduplicationRequest`/`Response`, `ReviewQueueItem`, `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `WorkerGrid` (SVAR `Grid` with `select` mode + `init`/`select-row`), `HumanNameInput`, `WorkerForm`, `MatchResultsList`.
- **Tests.** 5 Vitest unit tests for `ApiClient`, 3 unit tests for `WorkerRepository`, 6 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../worker-service-with-loco/spec.md`](../worker-service-with-loco/spec/index.md).
- Service REST contract: [`../worker-service-with-loco/AGENTS/restful.md`](../worker-service-with-loco/AGENTS/restful.md).
- Service model types: [`../worker-service-with-loco/AGENTS/models.md`](../worker-service-with-loco/AGENTS/models.md).
- Service matching reference: [`../worker-service-with-loco/AGENTS/matching.md`](../worker-service-with-loco/AGENTS/matching.md).
