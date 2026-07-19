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
- 2026-07-19 — SVAR moderate fit *not* taken: the suggested
  warranty/maintenance expiry calendar is data-gated — the Thing
  model carries no date-bearing domain fields; adding them is the
  service-side seam.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: the things index grid migrates from `wx-svelte-grid` to
  **@svar-ui/svelte-grid**, with a **@svar-ui/svelte-filter**
  FilterBar above it (client-side filtering). Legacy `wx-svelte-*`
  deps removed.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.
- **`checkDuplicates()` endpoint path.** `ThingRepository.checkDuplicates()` now POSTs to `/api/things/check-duplicates` (the path the Thing Service actually serves, per the service spec §6 / `AGENTS/restful.md` and this project's spec §9). It previously POSTed to `/api/things/duplicates`, which 404s against the real service. Removed the contradicting `/duplicates` claims from `AGENTS/testing.md` and this changelog, and updated the unit test to pin `check-duplicates`.
- **Doc fixes.** `AGENTS/testing.md` "Running" now says `pnpm check` (the actual type-check script; `pnpm svelte-check` was undefined). Corrected the `ApiClient` unit-test example to the real `new ApiClient({ baseUrl, fetch })` object constructor, and the Playwright example to the shipped no-service smoke approach (assert headings/nav, not API-driven health text). Spec §14 corrected to "5 E2E tests" (matches the suite). Stale `T-7` SSR cross-references in spec §7 and §10 corrected to `T-13`. FR-9 reconciled to match the implementation (preview is available but optional; merge is guarded by both-IDs-present-and-distinct + `confirm()`).

### Added

- **Tests.** Expanded `ThingRepository` unit coverage to every method (get/update/softDelete/match/merge/deduplicate/masked/exportGdpr/audit/recentAudit/health) plus the enveloped `{items,total}` search branch. Added `thing-form.test.ts` (FR-4 URL/name validation) and `merge-validation.test.ts` (FR-9 guard). Unit suite is now 32 tests across four files.

### Changed

- **Refactor (no behaviour change).** Extracted the Thing form validator into `src/lib/components/thing-validation.ts` (`validateThing`) and the merge-page guard into `src/lib/components/merge-validation.ts` (`validateMerge`), so FR-4 and FR-9 logic is unit-testable without mounting Svelte components. `ThingForm.svelte` and the merge route import these helpers.

## [0.1.0] — 2026-06-02

Initial scaffold for the Thing Service front-end. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless. Domain types follow [schema.org/Thing](https://schema.org/Thing).

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; things list with name / identifier / additional-type search and SVAR DataGrid (columns: ID, Name, schema.org Type, Primary identifier, URL); create with real-time 409 duplicate detection inline; detail view (identity, additional-type as schema.org URL, identifiers with deep links, alternate names, same-as URLs, images); edit; soft-delete with confirm; per-record audit log; match check (name + description + URL + identifiers + same-as); merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `ThingRepository` binding the [Thing Service REST surface](../thing-service-with-loco/AGENTS/restful.md).
- **TypeScript types.** Snake-case domain types mirroring [`thing-service-with-loco/AGENTS/models.md`](../thing-service-with-loco/AGENTS/models.md): `Thing` with all 13 schema.org/Thing canonical properties (`name`, `alternate_names`, `description`, `disambiguating_description`, `additional_type`, `url`, `identifiers`, `images`, `main_entity_of_page`, `owner`, `same_as`, `subject_of`, `potential_action`); `ThingIdentifier` with schema.org [`PropertyValue`](https://schema.org/PropertyValue) shape (`property_id`, `value`, optional `name`/`url`); `IdentifierType` (Doi/Isbn/Issn/Gtin/Sku/Mpn/SerialNumber/Uri/Uuid/`{Custom: string}`); `DETERMINISTIC_TYPES` constant lists identifier types that short-circuit matching to score 1.0 (Doi/Isbn/Issn/Gtin/Mpn/SerialNumber/Uuid — Sku/Uri/Custom excluded); `MatchResult` + `MatchConfidence` + `MatchBreakdown` (per-component: name / identifier / description / url / same_as / phonetic flag / deterministic flag); `MergeRequest`/`Record`/`Response`; `BatchDeduplicationRequest`/`Response`; `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `ThingGrid` (SVAR `Grid` with `select` + `init`/`select-row`), `ThingIdentifierInput` (dynamic add/remove, Custom-type label sub-field, optional per-identifier URL), `ThingForm` (name + additional_type URL + description + disambiguating description + URL + owner + multi-line alternate names + multi-line same_as URLs + identifier list; client-side validation of HTTP(S) URL fields), `MatchResultsList` with breakdown surfacing name / identifier / description / URL / same-as / phonetic / deterministic short-circuit.
- **Tests.** 5 Vitest unit tests for `ApiClient`, 3 unit tests for `ThingRepository`, 5 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../thing-service-with-loco/spec.md`](../thing-service-with-loco/spec/index.md).
- Service REST contract: [`../thing-service-with-loco/AGENTS/restful.md`](../thing-service-with-loco/AGENTS/restful.md).
- Service model types: [`../thing-service-with-loco/AGENTS/models.md`](../thing-service-with-loco/AGENTS/models.md).
- Service matching reference: [`../thing-service-with-loco/AGENTS/matching.md`](../thing-service-with-loco/AGENTS/matching.md).
