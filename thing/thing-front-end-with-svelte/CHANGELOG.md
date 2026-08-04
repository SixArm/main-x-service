# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added — review-queue screen upgrade (2026-08-04, T-24 / repo FE-4)

- `/review` gained a **status + page-size filter** (`?status=`/`?limit=`
  on `GET /api/things/review-queue`; "all" is the *absence* of `status`
  since the endpoint answers `422 INVALID_STATUS` for a token it does
  not know, and there is no `offset`), a **keyboard-reachable queue
  table** (a `Compare` button per row, real `Confirm`/`Reject` buttons
  in the panel) alongside the existing mouse-only drag-to-decide board,
  and an **inline side-by-side comparison panel** loading both things
  with two parallel `GET /api/things/{id}` calls and rendering id /
  name / additional type / description / url / owner / primary
  identifier / primary same-as, plus the matcher's `score_breakdown`
  as a component/weight/score table and its two boolean flags.
- Confirming a pair does **not** merge it (the decision endpoint is a
  pure status change); the panel now deep-links to
  `/things/merge?main=…&duplicate=…` in either survivor order, and
  `/things/merge` gained the matching `?main=`/`?duplicate=` seed.
- New pure module `src/lib/review.ts` (status vocabulary, `canDecide`,
  the five weighted `MATCH_COMPONENTS`, `breakdownRows`,
  `breakdownFlags`, `mergeHref`), unit-tested in
  `tests/unit/review.test.ts` (19 tests).
- i18n: 44 new keys across all 13 locales (real per-locale
  translations, reusing the existing `results.*` component labels
  rather than duplicating them).
- **Two documented, verified gaps versus person's own review screen**
  (checked against this service's actual `src/api/rest/handlers.rs` /
  `src/db/review_queue.rs`, not assumed byte-identical): this
  service's `review_queue` has **no `provenance` column at all**, so
  the queue surfaces `detection_method` in its place instead of
  fabricating a field the service does not have; and the wire
  `ReviewQueueItem` **never serializes `score_breakdown`** (the stored
  row has the column, but its one writer always writes `None`), so the
  breakdown table renders its documented empty state for every live
  item today — a backend follow-up, not a front-end shortfall. See
  `spec/06-functional-requirements.md` FR-12–FR-18 and
  `spec/13-tasks.md` T-24.

### Doc pass (2026-08-04, DOC-4)

- `.env.example` documented a decommissioned client-held-token model
  (`PUBLIC_API_BASE_URL=http://localhost:8080`, and — copy-pasted from
  another project — labelled "Person Service") with zero references in
  `src/`. The BFF actually reads `THING_API_URL`/`AUTH_API_URL`
  server-side (`src/lib/server/config.ts`, already correctly documented
  in `README.md`); rewrote `.env.example` to match. `index.md`'s
  Environment table had the same stale variable.
- Retroactively documented the BFF auth landing below (2026-07-04,
  `f66ff50f`) — it shipped with no `CHANGELOG.md` entry of its own; the
  only trace here was a later "left behind by recent BFF/auth-era
  edits" aside in a Fixed entry.
- `spec/13-tasks.md` T-18 (batch dedup UI) and T-22 (BFF auth) were
  still unchecked despite both being implemented and covered by the
  entries below; checked them off, and noted the real gaps T-22 leaves
  open (no CSRF) as a new T-23 (no e2e coverage for `/review`,
  `/signin`, `/verify`).
- `spec/08-architecture.md`'s diagram showed the browser calling the
  Thing Service directly; redrawn with the BFF proxy hop.
- `spec/09-api-consumption.md` was missing the review-queue endpoints
  (`GET /api/things/review-queue`, `POST
  /api/things/review-queue/{id}/decision`) and still called
  `POST /api/things/deduplicate` "not yet routed" though `/review`'s
  scan button calls it.
- `spec/15-roadmap.md` v0.3 still read "Auth integration (once Thing
  Service ships auth)" — false; the family-wide auth-service migration
  landed independently of any per-service auth work. Marked done.
- `spec/16-open-questions.md` OQ-3 asked how the UI should redirect on
  401/403 as if the BFF model itself were still hypothetical; it is
  implemented — narrowed the open question to the two things that are
  genuinely still missing (401/403 redirect, CSRF).
- `AGENTS.md` still said "Authentication. Out of scope until the
  service ships auth" under "What does NOT live here" — removed, and
  added a BFF section describing what's actually implemented.
- `README.md`: Prerequisites cited the Thing Service's old
  `http://localhost:8080` default (the Configuration section two
  sections down already correctly says 5150); Stack section still
  named the removed `wx-svelte-grid`/`wx-svelte-core` packages; Project
  layout tree predated `src/lib/server/`, `hooks.server.ts`, and the
  `/review`/`/signin`/`/verify`/`/api/proxy` routes, and mislabelled
  `+layout.svelte` "sidebar nav" though the family-wide rule (and this
  project's own `spec/§5`) is a top nav bar with a hamburger toggle.
  Fixed all four.
- Verified (not merely inferred): `pnpm install`, `pnpm check` (0/0),
  `pnpm test` (44/44), `pnpm build` all pass; the i18n key-parity test
  covers `review.*`/`signin.*`/`verify.*` with real (non-English-stub)
  translations across all 13 locales.
- Not fixed here (flagged, not silently patched): `pnpm lint`
  (`prettier --check src`) currently fails on `src/lib/api/types.ts`
  and `src/lib/svar-filter-augment.d.ts` — pre-existing formatting
  drift, out of this doc-only pass's scope.

### Added — BFF authentication (2026-07-04, `f66ff50f`)

- Adopted the family-wide cookie-session + PASETO v4.public model
  (`agents/share/authentication-sessions.md`): `src/hooks.server.ts`
  resolves the httpOnly `__Host-mxi_session` cookie into
  `locals.sessionId`; `/signin` requests a magic link, `/verify`
  consumes it and establishes the session; `/api/proxy/[...path]` is
  the same-origin reverse proxy that exchanges the session for a
  short-lived PASETO and forwards to the Thing Service with
  `Authorization: Bearer <paseto>`. The browser never holds a token.
  `src/lib/server/{config,session,auth}.ts` hold the server-only
  implementation. CSRF protection is not yet implemented (see T-22).

### Added — drag-to-decide review board (2026-07-19)

- 2026-07-19 — `/review` now loads the **stored** review queue on mount
  (`GET /api/things/review-queue`, a safe read; the scan button still
  runs the destructive-classed batch scan explicitly) and dragging a
  pending card into Confirmed / Rejected records the decision through
  `POST /api/things/review-queue/{id}/decision`. Illegal drags are
  refused client-side and the reload restores the stored truth.
- Repository gains `listReviewQueue()` / `decideReview()`; types gain
  `ReviewDecision` + `ReviewQueueListResponse`.
- e2e: the dashboard smoke spec now opens the hamburger dropdown before
  asserting nav links (the nav is toggle-only at every viewport width;
  the old spec predated that layout).

### Fixed

- 2026-07-19 — dedup-report drift: `ReviewStatus` lowered to the wire tokens; `ReviewQueueItem`
  gains `detection_method` and the review board's cards show it.

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
