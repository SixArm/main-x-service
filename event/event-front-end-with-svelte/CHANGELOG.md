# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Fixed — `/verify` crashed with a raw 500 when the authentication service was unreachable (T-29)

`src/routes/verify/+page.server.ts` called `await verifyMagicLink(fetch,
token)` with no `try`/`catch`. A network-level failure (the
authentication service unreachable, timed out, connection reset) makes
`fetch` throw rather than resolve — uncaught, that propagated out of
`load` and SvelteKit rendered its generic 500 error page instead of
this route's own friendly UI. The same bug class was found and fixed
first in `place-front-end-with-svelte` (T-26) and
`thing-front-end-with-svelte` (T-23); ported here: a `try`/`catch`
around the call, a new `"serviceUnavailable"` error variant, and its
message in `+page.svelte`. New `tests/unit/verify.test.ts` unit-tests
the `load` function directly (missing token / service unavailable /
invalid token), verified to fail with the `try`/`catch` reverted and
pass with it restored. See spec §13 T-29.

### Fixed — the calendar never showed a same-day all-day event (T-27)

`/calendar` had zero test coverage, and writing the required "an event
actually renders" Playwright assertion surfaced a genuine, previously
shipped bug: `@svar-ui/calendar-store` requires an all-day event's
`end` to be strictly *after* `start`, but `+page.svelte` passed a
same-day all-day event's `end_date` straight through unchanged
(equal to `start_date`, or absent) — the widget silently dropped every
such event. This is the same root cause worker-front-end's `/expiry`
calendar had. Fixed by computing an exclusive end one calendar day past
the later of the event's start/end day for all-day events only; timed
events are unchanged. New Playwright test in `tests/e2e/events.spec.ts`
stubs a timed and a same-day-all-day event, asserts both render, and
asserts selecting the all-day one navigates to its detail page. See
spec §13 T-27, including the residual (no coverage yet for the
drag-driven `PUT` on reschedule — SVAR exposes no headless hook for its
drag gesture).

### Added — GDPR export download on the detail page (T-20)

A button on `/events/[id]` fetches `GET /api/events/{id}/export` through the
existing `EventRepository.exportGdpr(id)` and saves the payload as
`event-<id>-export.json` (Blob object URL + synthetic anchor; the button is
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

### Fixed

- 2026-08-04 — **DOC-4 doc audit.** `.env.example` was a stale copy of person-service's template (`PUBLIC_API_BASE_URL=http://localhost:8080`) — this app's real BFF server reads `EVENT_API_URL`/`AUTH_API_URL` (both defaulting to `:5150`); fixed to match `src/lib/server/config.ts`. `AGENTS.md`, `spec/02-scope.md`, `spec/13-tasks.md`, `spec/15-roadmap.md`, `spec/16-open-questions.md`, `spec/08-architecture.md`, and `index.md` still described auth/i18n/the calendar route as unimplemented or future work; the BFF (magic-link `/signin` + `/verify`, httpOnly session cookie, server-side PASETO exchange, `/api/proxy` reverse proxy) and 13-locale i18n are both landed — updated docs to match, split T-23 into T-23a (done) / T-23b (CSRF, still open), and added T-24 for the still-English-only `/signin`/`/verify` copy. Fixed `README.md`'s stale `wx-svelte-grid`/`wx-svelte-core` references (migrated to `@svar-ui/svelte-grid`/`svelte-filter` 2026-07-19) and its internally-contradictory `:8080` default (the Configuration table two sections down already said `:5150`). Fixed `index.md`'s route map (missing `/calendar`, `/signin`, `/verify`) and a factual error carried over from the matcher docs — the match-check worked-flow claimed "window-overlap" scoring, which `event-matcher` does not implement (Gaussian endpoint-decay instead; window-overlap is its OQ-C, still open).

### Added

- 2026-07-19 — SVAR strong fit: new **/calendar** route (nav-linked): Event time windows in the
  SVAR Calendar (month view); dragging an event writes the new
  window back through the normal update endpoint and reloads the
  truth; selecting an event opens its detail page. One new i18n key
  (`nav.calendar`) across all 13 locales.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: the events index grid migrates from `wx-svelte-grid` to
  **@svar-ui/svelte-grid**, with a **@svar-ui/svelte-filter**
  FilterBar above it (client-side filtering). Legacy `wx-svelte-*`
  deps removed.

### Changed

- **De-versioned API URLs.** Dropped the `/api/v1` segment from every client/proxy/test path (now `/api/…`); the BFF proxy negotiates the API version via the `Accepts-version: 1.0` request header instead (see `agents/share/api-versioning.md`).

### Added

- **Fuzzy search toggle (FR-2).** The events list page now exposes a **Fuzzy** checkbox wired into `SearchOptions.fuzzy` and the `GET /api/events/search?fuzzy=…` query. Phonetic search is **not** offered: it is not a service search parameter (Soundex is internal to the matcher's name scoring), so it is deferred as spec §13 T-22.
- **Tests.** New `tests/unit/form.test.ts` covering the `createForm` rune store and the Event time-window validation rules (FR-4: name + start required, end ≥ start, door ≤ start). New `EventRepository` unit tests for the `fuzzy` query param, the merge request body shape, and the per-ID merge-preview GET (FR-9). New Playwright smoke tests for the detail (FR-5), edit (FR-6), and audit route shells.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.
- **Doc/spec harmonisation.** Dropped the unimplemented "phonetic" search claim from spec §02 / §06 FR-2, `README.md`, and `index.md`; FR-2 now matches the implemented fuzzy toggle. Corrected the spec §14 test-count line and the §08 architecture diagram path (`lib/api/events.ts`, not `lib/api/v1/events.ts`). Fixed `agents/testing.md`: the `ApiClient` example now uses the `{ baseUrl, fetch }` options object, and the type-check command is `pnpm check` (not `pnpm svelte-check`).

## [0.1.0] — 2026-06-02

Initial scaffold for the Event Service front-end. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless. Domain types follow [schema.org/Event](https://schema.org/Event).

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; events list with name / organizer / identifier full-text search **plus date-range and status / type / mode dropdown filters** and SVAR DataGrid (columns: ID, Name, Start, Type, Status, Mode); create with real-time 409 duplicate detection inline; detail view (identity, location entries with kind dispatch, organizers/performers as `Party` records, identifiers, offers); edit; soft-delete with confirm; per-record audit log; match check (name + start_date + optional end_date + optional organizer); merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `EventRepository` binding the [Event Service REST surface](../event-service-with-loco/agents/restful.md). **Note:** Event Service mounts REST under **`/api/v1/`** (not `/api/`), so all routes are versioned: `/api/v1/events`, `/api/v1/events/search`, `/api/v1/events/match`, `/api/v1/events/check-duplicates`, `/api/v1/events/merge`, `/api/v1/events/{id}/audit`, `/api/v1/audit/recent`, `/api/v1/health`.
- **TypeScript types.** Snake-case domain types mirroring [`event-service-with-loco/agents/models.md`](../event-service-with-loco/agents/models.md): `Event` with the schema.org/Thing inheritance (name, alternate_names, description, url, image, same_as, keywords) plus the time window (`start_date` **required**, `end_date`, `door_time`, `duration` ISO 8601, `previous_start_date`, `time_zone`, `all_day`); `EventStatus` (scheduled/cancelled/moved_online/postponed/rescheduled/completed); `EventAttendanceMode` (offline/online/mixed); `EventType` enumerating 29 schema.org/Event subtypes plus operational subtypes (`appointment` / `encounter` / `shift` / `incident` / …); discriminated `Location` union (`{kind: "place"}` / `"postal_address"` / `"virtual"` / `"text"`) mirroring schema.org's `Place | PostalAddress | VirtualLocation | Text`; `Party` with `kind: "person" | "organization"` and optional external service ID; `Reference` for `about` / `works`; `Offer` with availability enum; `Identifier` + `IdentifierType` (BookingNumber/ConfirmationCode/TicketNumber/EncounterId/TransactionId/ExternalRef/Tax/Other); `STRONG_IDENTIFIER_TYPES` constant listing the five identifier types that short-circuit matching to score 1.0; `EventLink`; `MatchResult` + `MatchQuality` + `MatchBreakdown`; `MergeRequest`/`Record`/`Response`; `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `EventGrid` (SVAR `Grid` with `select` + `init`/`select-row`), `EventForm` with `datetime-local` inputs for start / end / door_time, full-validation (start required, `end >= start`, `door <= start`), status/type/mode dropdowns, capacity-breakdown fields (total/physical/virtual), comma-separated keywords, ISO 639-1 language list, `MatchResultsList`.
- **Tests.** 5 Vitest unit tests for `ApiClient`; 4 unit tests for `EventRepository` (pins `/api/v1/events` route, `date_from` / `date_to` query params, `/api/v1/health` endpoint); 5 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../event-service-with-loco/spec.md`](../event-service-with-loco/spec/index.md).
- Service REST contract: [`../event-service-with-loco/agents/restful.md`](../event-service-with-loco/agents/restful.md).
- Service model types: [`../event-service-with-loco/agents/models.md`](../event-service-with-loco/agents/models.md).
- Service matching reference: [`../event-service-with-loco/agents/matching.md`](../event-service-with-loco/agents/matching.md).
