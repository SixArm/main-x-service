# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: the places index grid migrates from `wx-svelte-grid` to
  **@svar-ui/svelte-grid**, with a **@svar-ui/svelte-filter**
  FilterBar above it (client-side filtering). Legacy `wx-svelte-*`
  deps removed.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.
- **Docs harmonization.** Removed stale `/api/places/duplicates` copy
  artifacts from `CHANGELOG.md` and `AGENTS/testing.md`; the client,
  unit test, and service contract have always used the hyphenated
  `/api/places/check-duplicates` (entity task E-1, fixed 2026-06-13).
- **Spec consistency.** Corrected `spec/14-implementation-status.md`
  E2E count to 5 tests (was 6); pointed the SSR follow-up cross-refs in
  `spec/07-non-functional-requirements.md` and `spec/10-persistence.md`
  at T-13 (was T-7); fixed the `AGENTS/index.md` shared-docs link labels
  to match their hrefs.
- **Testing docs.** `AGENTS/testing.md` unit-test example now uses the
  options-object `ApiClient` constructor and the `{success, data, error}`
  envelope so it compiles as written.

### Added

- **Unit tests.** Path-pinning tests for the remaining `PlaceRepository`
  methods (`get`, `update`, `softDelete`, `match`, `merge`, `deduplicate`,
  `masked`, `exportGdpr`, `audit`, `recentAudit`, `health`) plus the
  enveloped search-total case; new `form.svelte.test.ts` covering the
  rune-based `createForm` store. Unit suite grows from 8 to 27 tests.

## [0.1.0] — 2026-06-02

Initial scaffold for the Place Service front-end. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless. Domain types follow [schema.org/Place](https://schema.org/Place).

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; places list with name / locality / identifier search and SVAR DataGrid (columns: ID, Name, Type, City, Country, Lat/Lon); create with real-time 409 duplicate detection inline; detail view (identity, address, geo, identifiers, opening hours, amenities); edit; soft-delete with confirm; per-record audit log; match check (name + address + optional geo); merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `PlaceRepository` binding the [Place Service REST surface](../place-service-with-loco/AGENTS/restful.md). **Note:** Place Service uses `POST /api/places/check-duplicates` (hyphenated) for the duplicate-check, distinct from `/match`.
- **TypeScript types.** Snake-case domain types mirroring [`place-service-with-loco/AGENTS/models.md`](../place-service-with-loco/AGENTS/models.md): `Place`, `PostalAddress` (street_address, address_locality, address_region, address_country, postal_code), `GeoCoordinates` (latitude, longitude, optional elevation), `PlaceType` enum (LocalBusiness, CivicStructure, Park, Hospital, … plus `{Other: string}`), `PlaceIdentifier` with `IdentifierType` (GlobalLocationNumber/BranchCode/Fips/Gnis/OpenStreetMap/`{Custom}`), `AmenityFeature`, `OpeningHoursSpecification` + `DayOfWeek`, `MatchResult` + `MatchConfidence` + `MatchBreakdown` (deterministic-match flag for GLN short-circuit), `MergeRequest`/`Record`/`Response`, `BatchDeduplicationRequest`/`Response`, `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `PlaceGrid` (SVAR `Grid` with `select` + `init`/`select-row`), `PostalAddressInput`, `GeoCoordinatesInput` (with lat/lon range validation), `PlaceForm` (name + alternate name + place type + telephone + URL + 13-digit GLN with client-side check + optional address + optional geo), `MatchResultsList` with breakdown surfacing geo / address / identifier / deterministic-GLN.
- **Tests.** 5 Vitest unit tests for `ApiClient`, 3 unit tests for `PlaceRepository` (pins `/check-duplicates`), 5 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../place-service-with-loco/spec.md`](../place-service-with-loco/spec/index.md).
- Service REST contract: [`../place-service-with-loco/AGENTS/restful.md`](../place-service-with-loco/AGENTS/restful.md).
- Service model types: [`../place-service-with-loco/AGENTS/models.md`](../place-service-with-loco/AGENTS/models.md).
- Service matching reference: [`../place-service-with-loco/AGENTS/matching.md`](../place-service-with-loco/AGENTS/matching.md).
