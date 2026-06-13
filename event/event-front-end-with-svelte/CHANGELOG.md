# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

Nothing yet.

## [0.1.0] — 2026-06-02

Initial scaffold for the Event Service front-end. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless. Domain types follow [schema.org/Event](https://schema.org/Event).

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; events list with name / organizer / identifier full-text search **plus date-range and status / type / mode dropdown filters** and SVAR DataGrid (columns: ID, Name, Start, Type, Status, Mode); create with real-time 409 duplicate detection inline; detail view (identity, location entries with kind dispatch, organizers/performers as `Party` records, identifiers, offers); edit; soft-delete with confirm; per-record audit log; match check (name + start_date + optional end_date + optional organizer); merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `EventRepository` binding the [Event Service REST surface](../event-service-rust-crate/AGENTS/restful.md). **Note:** Event Service mounts REST under **`/api/v1/`** (not `/api/`), so all routes are versioned: `/api/v1/events`, `/api/v1/events/search`, `/api/v1/events/match`, `/api/v1/events/check-duplicates`, `/api/v1/events/merge`, `/api/v1/events/{id}/audit`, `/api/v1/audit/recent`, `/api/v1/health`.
- **TypeScript types.** Snake-case domain types mirroring [`event-service-rust-crate/AGENTS/models.md`](../event-service-rust-crate/AGENTS/models.md): `Event` with the schema.org/Thing inheritance (name, alternate_names, description, url, image, same_as, keywords) plus the time window (`start_date` **required**, `end_date`, `door_time`, `duration` ISO 8601, `previous_start_date`, `time_zone`, `all_day`); `EventStatus` (scheduled/cancelled/moved_online/postponed/rescheduled/completed); `EventAttendanceMode` (offline/online/mixed); `EventType` enumerating 29 schema.org/Event subtypes plus operational subtypes (`appointment` / `encounter` / `shift` / `incident` / …); discriminated `Location` union (`{kind: "place"}` / `"postal_address"` / `"virtual"` / `"text"`) mirroring schema.org's `Place | PostalAddress | VirtualLocation | Text`; `Party` with `kind: "person" | "organization"` and optional external service ID; `Reference` for `about` / `works`; `Offer` with availability enum; `Identifier` + `IdentifierType` (BookingNumber/ConfirmationCode/TicketNumber/EncounterId/TransactionId/ExternalRef/Tax/Other); `STRONG_IDENTIFIER_TYPES` constant listing the five identifier types that short-circuit matching to score 1.0; `EventLink`; `MatchResult` + `MatchQuality` + `MatchBreakdown`; `MergeRequest`/`Record`/`Response`; `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `EventGrid` (SVAR `Grid` with `select` + `init`/`select-row`), `EventForm` with `datetime-local` inputs for start / end / door_time, full-validation (start required, `end >= start`, `door <= start`), status/type/mode dropdowns, capacity-breakdown fields (total/physical/virtual), comma-separated keywords, ISO 639-1 language list, `MatchResultsList`.
- **Tests.** 5 Vitest unit tests for `ApiClient`; 4 unit tests for `EventRepository` (pins `/api/v1/events` route, `date_from` / `date_to` query params, `/api/v1/health` endpoint); 5 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../event-service-rust-crate/spec.md`](../event-service-rust-crate/spec/index.md).
- Service REST contract: [`../event-service-rust-crate/AGENTS/restful.md`](../event-service-rust-crate/AGENTS/restful.md).
- Service model types: [`../event-service-rust-crate/AGENTS/models.md`](../event-service-rust-crate/AGENTS/models.md).
- Service matching reference: [`../event-service-rust-crate/AGENTS/matching.md`](../event-service-rust-crate/AGENTS/matching.md).
