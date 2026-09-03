# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added — GDPR export download on the detail page (T-20)

A button on `/places/[id]` fetches `GET /api/places/{id}/export` through the
existing `PlaceRepository.exportGdpr(id)` and saves the payload as
`place-<id>-export.json` (Blob object URL + synthetic anchor; the button is
disabled while the request is in flight). New i18n keys across all 13
locales; a repository test pins the endpoint and a Playwright smoke test
asserts the real browser download — filename and saved bytes.
Copy-adapted from person's reference (repo `tasks.md` WEB-5).

### Changed — T-13, T-16, T-17 closed (2026-09-03, repo WEB-6)

Three tasks open word-for-word in six front-ends, closed with reasons
rather than left as identical unticked rows: T-13 (SSR-safe loads)
contradicts this project's CSR-only + BFF design (`ssr = false`); T-16
(a theme module) is superseded by the Lily `ThemePicker` adopted
2026-07-31; T-17 (check-duplicates preview) is already delivered by
T-6 — `POST` answers `409` with the candidates without creating, and
the form shows them. T-17's investigation found a real gap that is
the service's, not this app's: no override exists to create past a
`409`. Doc-only; no behaviour.

### Added — masked-view toggle on the detail page (T-19)

A toggle button on `/places/[id]` re-fetches through the existing
`PlaceRepository.masked(id)` (`GET /api/places/{id}/masked`) instead of
redacting fields client-side. Shows a status banner while the masked
view is active. New i18n keys across all 13 locales; a Playwright smoke
test exercises the toggle end to end with visibly different stubbed
`telephone` values. The repository method and its unit test already
existed — this closes the gap where it was never surfaced in any
route's UI. Mirrors person's and worker's identical T-19 delivery.

### Fixed — DOC-4 doc audit (2026-08-04)

- **`.env.example` was stale and wrong.** It documented `PUBLIC_API_BASE_URL`
  (a pre-BFF client-held var read by nothing in `src/`); the real,
  server-only vars `src/lib/server/config.ts` reads are `PLACE_API_URL`
  and `AUTH_API_URL` (both default `http://localhost:5150`). Fixed
  directly (config bug, not a behavioural claim).
- **`AGENTS.md` overclaimed auth was out of scope.** T-22 (BFF auth:
  `/signin`, `/verify`, `/api/proxy`, `src/lib/server/{session,auth,config}.ts`)
  shipped without this file being updated. Added a "BFF pattern" section
  and a `src/lib/server/` row to "what lives where"; removed the false
  "Authentication out of scope" bullet.
- **`spec/13-tasks.md`** T-18 (batch dedup review UI) and T-22 (BFF auth)
  were still unchecked despite being implemented (`/review`,
  `/signin`+`/verify`+`/api/proxy`). Marked done with landing notes.
- **`spec/14-implementation-status.md`** test counts were stale: it said
  8 unit / 6 e2e tests; the suite is actually 40 unit tests across 5
  files and 5 e2e tests (verified via `pnpm test` / grep this session).
  Added rows for the review board and BFF auth; verified `pnpm check`
  (0 errors/0 warnings), `pnpm install`, `pnpm test`, `pnpm build`.
- **`spec/09-api-consumption.md`** still listed `POST /api/places/deduplicate`
  as "not yet routed" — it drives the `/review` scan button. Added the
  two review-queue endpoints (`GET .../review-queue`,
  `POST .../review-queue/{id}/decision`) that were missing entirely, and
  a note that calls now go through `/api/proxy`.
- **`spec/08-architecture.md`** diagram showed the browser calling the
  Place Service directly; added the BFF proxy hop.
- **`spec/01-purpose-and-vision.md`, `spec/03-stakeholders-and-users.md`,
  `spec/12-compliance.md`, `spec/15-roadmap.md`** all still framed auth
  as future/deferred ("out of scope until the service ships it",
  "deferred until auth lands"); reworded now that T-22 has landed.
  `spec/15-roadmap.md`'s v0.4 line was also a leftover copy-paste
  artifact ("sibling scaffolds for Worker/Place/Thing/Event front-ends"
  — this project *is* one of those siblings, and all exist already);
  replaced with the actual remaining tasks (T-17, T-19, T-20, T-21).
  Fixed a stray "Placea" table-header typo (→ "Persona") in
  `spec/03-stakeholders-and-users.md` while there.
- **`README.md`** Stack/SVAR-DataGrid sections still named the removed
  `wx-svelte-grid`/`wx-svelte-core` deps; the 2026-07-19 migration to
  `@svar-ui/svelte-grid` (+ `svelte-filter`, `svelte-kanban`) was never
  reflected there. Updated to match `package.json`.
- **`index.md`** (June 18 vintage) predated the BFF: missing `/review`,
  `/signin`, `/verify` from the route map, and its Environment section
  had the same stale `PUBLIC_API_BASE_URL` as `.env.example`. Fixed both.
- Verified: no client-side `id`/UUID generation workaround exists in the
  create path (`PlaceRepository.create` posts the form's `Place` as-is;
  `id` is optional in `types.ts` and left unset) — nothing to undo for
  place-service's `QA-SERVER-FIELDS` fix (server-owned fields on
  `POST /api/places` are no longer required). i18n: the 13-locale parity
  test (`tests/unit/i18n.test.ts`, "every locale defines every English
  key") passes, and spot-checked recently-added `review.*` strings carry
  real per-locale translations, not English stubs.

### Added — drag-to-decide review board (2026-07-19)

- 2026-07-19 — `/review` now loads the **stored** review queue on mount
  (`GET /api/places/review-queue`, a safe read; the scan button still
  runs the destructive-classed batch scan explicitly) and dragging a
  pending card into Confirmed / Rejected records the decision through
  `POST /api/places/review-queue/{id}/decision`. Illegal drags are
  refused client-side and the reload restores the stored truth.
- Repository gains `listReviewQueue()` / `decideReview()`; types gain
  `ReviewDecision` + `ReviewQueueListResponse`.
- e2e: the dashboard smoke spec now opens the hamburger dropdown before
  asserting nav links (the nav is toggle-only at every viewport width;
  the old spec predated that layout).

### Fixed

- 2026-07-19 — dedup-report drift: `ReviewStatus` lowered to the wire tokens; `ReviewQueueItem`
  gains `detection_method` (the service now sends it) and the
  review board's cards show it again.

### Added

- 2026-07-19 — SVAR moderate fit: new **/review** route (nav-linked): the batch-deduplication
  scan's review queue as SVAR Kanban columns (Pending / Confirmed /
  Rejected / AutoMerged). The scan runs only on the button (POST
  /deduplicate is destructive-classed, never a page-load side
  effect). Read-only: the service exposes no review-decision
  endpoint yet — that endpoint is the seam that would make the
  columns drag targets. +nav.review/review.run x 13 locales.

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
  artifacts from `CHANGELOG.md` and `agents/testing.md`; the client,
  unit test, and service contract have always used the hyphenated
  `/api/places/check-duplicates` (entity task E-1, fixed 2026-06-13).
- **Spec consistency.** Corrected `spec/14-implementation-status.md`
  E2E count to 5 tests (was 6); pointed the SSR follow-up cross-refs in
  `spec/07-non-functional-requirements.md` and `spec/10-persistence.md`
  at T-13 (was T-7); fixed the `agents/index.md` shared-docs link labels
  to match their hrefs.
- **Testing docs.** `agents/testing.md` unit-test example now uses the
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
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `PlaceRepository` binding the [Place Service REST surface](../place-service-with-loco/agents/restful.md). **Note:** Place Service uses `POST /api/places/check-duplicates` (hyphenated) for the duplicate-check, distinct from `/match`.
- **TypeScript types.** Snake-case domain types mirroring [`place-service-with-loco/agents/models.md`](../place-service-with-loco/agents/models.md): `Place`, `PostalAddress` (street_address, address_locality, address_region, address_country, postal_code), `GeoCoordinates` (latitude, longitude, optional elevation), `PlaceType` enum (LocalBusiness, CivicStructure, Park, Hospital, … plus `{Other: string}`), `PlaceIdentifier` with `IdentifierType` (GlobalLocationNumber/BranchCode/Fips/Gnis/OpenStreetMap/`{Custom}`), `AmenityFeature`, `OpeningHoursSpecification` + `DayOfWeek`, `MatchResult` + `MatchConfidence` + `MatchBreakdown` (deterministic-match flag for GLN short-circuit), `MergeRequest`/`Record`/`Response`, `BatchDeduplicationRequest`/`Response`, `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `PlaceGrid` (SVAR `Grid` with `select` + `init`/`select-row`), `PostalAddressInput`, `GeoCoordinatesInput` (with lat/lon range validation), `PlaceForm` (name + alternate name + place type + telephone + URL + 13-digit GLN with client-side check + optional address + optional geo), `MatchResultsList` with breakdown surfacing geo / address / identifier / deterministic-GLN.
- **Tests.** 5 Vitest unit tests for `ApiClient`, 3 unit tests for `PlaceRepository` (pins `/check-duplicates`), 5 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../place-service-with-loco/spec.md`](../place-service-with-loco/spec/index.md).
- Service REST contract: [`../place-service-with-loco/agents/restful.md`](../place-service-with-loco/agents/restful.md).
- Service model types: [`../place-service-with-loco/agents/models.md`](../place-service-with-loco/agents/models.md).
- Service matching reference: [`../place-service-with-loco/agents/matching.md`](../place-service-with-loco/agents/matching.md).
