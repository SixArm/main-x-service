# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

Nothing yet.

## [0.1.0] — 2026-06-02

Initial scaffold for the Place Service front-end. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless. Domain types follow [schema.org/Place](https://schema.org/Place).

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; places list with name / locality / identifier search and SVAR DataGrid (columns: ID, Name, Type, City, Country, Lat/Lon); create with real-time 409 duplicate detection inline; detail view (identity, address, geo, identifiers, opening hours, amenities); edit; soft-delete with confirm; per-record audit log; match check (name + address + optional geo); merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `PlaceRepository` binding the [Place Service REST surface](../place-service-rust-crate/AGENTS/restful.md). **Note:** Place Service uses `POST /api/places/duplicates` (not `/check-duplicates`).
- **TypeScript types.** Snake-case domain types mirroring [`place-service-rust-crate/AGENTS/models.md`](../place-service-rust-crate/AGENTS/models.md): `Place`, `PostalAddress` (street_address, address_locality, address_region, address_country, postal_code), `GeoCoordinates` (latitude, longitude, optional elevation), `PlaceType` enum (LocalBusiness, CivicStructure, Park, Hospital, … plus `{Other: string}`), `PlaceIdentifier` with `IdentifierType` (GlobalLocationNumber/BranchCode/Fips/Gnis/OpenStreetMap/`{Custom}`), `AmenityFeature`, `OpeningHoursSpecification` + `DayOfWeek`, `MatchResult` + `MatchConfidence` + `MatchBreakdown` (deterministic-match flag for GLN short-circuit), `MergeRequest`/`Record`/`Response`, `BatchDeduplicationRequest`/`Response`, `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `PlaceGrid` (SVAR `Grid` with `select` + `init`/`select-row`), `PostalAddressInput`, `GeoCoordinatesInput` (with lat/lon range validation), `PlaceForm` (name + alternate name + place type + telephone + URL + 13-digit GLN with client-side check + optional address + optional geo), `MatchResultsList` with breakdown surfacing geo / address / identifier / deterministic-GLN.
- **Tests.** 5 Vitest unit tests for `ApiClient`, 3 unit tests for `PlaceRepository` (pins `/duplicates` not `/check-duplicates`), 5 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../place-service-rust-crate/spec.md`](../place-service-rust-crate/spec/index.md).
- Service REST contract: [`../place-service-rust-crate/AGENTS/restful.md`](../place-service-rust-crate/AGENTS/restful.md).
- Service model types: [`../place-service-rust-crate/AGENTS/models.md`](../place-service-rust-crate/AGENTS/models.md).
- Service matching reference: [`../place-service-rust-crate/AGENTS/matching.md`](../place-service-rust-crate/AGENTS/matching.md).
