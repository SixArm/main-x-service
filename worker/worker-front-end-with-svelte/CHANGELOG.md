# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Fixed — `/verify` crashed with a raw 500 when the authentication service was unreachable (T-31)

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
pass with it restored. See spec §13 T-31.

### Fixed — `/expiry` silently truncated with no signal to the operator (T-29)

`onMount` called `repo.search({ q: "*", limit: 200 })` and only read
`res.items`, discarding `res.total` even though `WorkerRepository.search()`
already returns `{ items, total }`. When more than 200 workers carried
documents, the calendar silently showed a partial window — the code's
own comment named this ("a window, not a promise of completeness") but
nothing in the UI said so. A visible notice now appears above the
calendar whenever `total > items.length` ("Showing up to N of M
workers", new i18n key `expiry.truncationNotice` across all 13
locales). New Playwright test, verified to fail without the fix and
pass with it; the existing T-28 test now also asserts the notice is
absent when the window is complete. See spec §13 T-29.

### Fixed — the expiry calendar never actually showed any events (T-28)

`/expiry`'s per-document event built `end: day` — the *same* `Date`
object as `start` — but `@svar-ui/calendar-store` requires an all-day
event's `end` to be strictly *after* `start`; an equal `start`/`end`
silently filtered out **every** expiry event this calendar was ever
asked to show, since the route shipped. Found while writing its first
Playwright test (T-28's actual acceptance criterion — "the only route
in the route map with zero e2e coverage" — turned into "the only route
whose one feature never worked"), verified directly against the
compiled library before touching the app code. Fixed by making `end`
the following calendar day, the minimum exclusive span a one-day
all-day event needs. New Playwright test stubs one worker with one
document expiring on the 15th of the *current* month (the widget
always opens on today's month, so a fixed future date would eventually
scroll out of view), asserts the rendered event, clicks it, and
asserts the resulting `/workers/{id}` navigation. See spec/13-tasks.md
T-28.

### Added — test coverage for the phonetic search toggle (T-30)

`SearchOptions.phonetic` was wired all the way to the
`/api/workers/search` query string (`src/lib/api/workers.ts`), same as
`fuzzy`, but only `fuzzy` had a test pinning it reached the wire. New
`WorkerRepository` test in `tests/unit/workers.test.ts` asserts
`phonetic=true` (and `fuzzy=true`) both appear in the request URL. See
spec §13 T-30.

### Added — GDPR export download on the detail page (T-20)

A button on `/workers/[id]` fetches `GET /api/workers/{id}/export` through the
existing `WorkerRepository.exportGdpr(id)` and saves the payload as
`worker-<id>-export.json` (Blob object URL + synthetic anchor; the button is
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

A toggle button on `/workers/[id]` re-fetches through the existing
`WorkerRepository.masked(id)` (`GET /api/workers/{id}/masked`) instead
of redacting fields client-side. Shows a status banner while the
masked view is active. New i18n keys across all 13 locales; a
repository test pins the endpoint, and a Playwright smoke test
exercises the toggle end to end with visibly different stubbed
responses. Mirrors person-front-end's identical T-19 delivery.

### Fixed — T-18 closed as duplicate

T-18 ("batch deduplicate-scan results UI") was never itself ticked
when the `/review` board landed under T-25's task number instead.
Verified `src/routes/review/` is live and closed T-18 as a duplicate
rather than leaving it open against work that already shipped.

### Added — duplicate review-queue screen (2026-08-04, repo FE-4, T-25)

- `/review` gains a **status + page-size filter** wired to `?status=` /
  `?limit=` (`WorkerRepository.listReviewQueue` gained a
  `ReviewQueueOptions` argument); "all" is the *absence* of `status`,
  since the endpoint answers `422 INVALID_STATUS` for a token it does
  not know — confirmed against
  `worker-service-with-loco/src/api/rest/handlers.rs::get_review_queue`,
  which is byte-for-byte the same guard as the person service's, and
  there is no `offset` so page size is the whole pagination story.
- A **keyboard-reachable path**: the pre-existing SVAR Kanban board's
  drag-to-decide interaction is mouse-only, so a native queue table
  (`data-testid="review-list"`) with a per-row `Compare` button was
  added alongside it — drag still works, it is simply no longer the
  only way in.
- An **inline side-by-side comparison panel**
  (`data-testid="review-compare"`) fetching both sides of a pair with
  two parallel `GET /api/workers/{id}` calls (`Promise.allSettled`, so a
  soft-deleted side still renders the other) and rendering the
  matcher's `score_breakdown` as a component / weight / score table.
  The seven components and weights (name 0.30, birth date 0.25, gender
  0.10, address 0.10, identifier 0.10, tax id 0.10, document 0.05) are
  identical in name and value to the person service's own in-service
  matcher (confirmed against `worker-service-with-loco/src/matching/
  mod.rs::MatchScoreBreakdown`) — a distinct, much simpler struct from
  the `worker-matcher` reference crate's ~50-component breakdown, which
  does not power this queue.
- Real `Confirm` / `Reject` buttons in the panel, disabled for a
  non-`pending` item (`canDecide`) rather than offering a request
  guaranteed to answer `422 INVALID_REVIEW_TRANSITION` (first-writer-
  wins).
- **Confirming does not merge.** The decision endpoint is a pure status
  change; the panel deep-links to `/workers/merge?main=…&duplicate=…`
  in either survivor order (a review item names an unordered pair), and
  `/workers/merge` gained a one-line `?main=`/`?duplicate=` query-string
  seed it did not have before (`page.url.searchParams`, matching the
  person front-end's own merge page).
- New pure module `src/lib/review.ts`: the four-status vocabulary,
  `canDecide`, the seven weighted `MATCH_COMPONENTS`, `breakdownRows`
  (omits an absent component rather than showing it as zero — "not
  compared" must not read as "compared and did not match"), and
  `mergeHref`.
- **Known gap, verified rather than assumed**: unlike the person
  service, `worker-service-with-loco`'s `review_queue` table carries no
  `provenance` column, so this screen cannot surface a provenance badge
  the service does not send. This is a backend gap tracked for the
  service crate, not a front-end omission — see `spec/13-tasks.md` T-25.
- Tests: `tests/unit/review.test.ts` (15 new — weight sum, descending
  order, the null/non-object/unknown-key breakdown paths, the decidable
  guard, both merge-link orders); four new `WorkerRepository` tests in
  `tests/unit/workers.test.ts` (bare call sends no query string, both
  filters reach the wire, an absent status is omitted rather than sent
  as `"undefined"`, the decision body's field is `status`); an
  i18n-parity extension for 57 new `review.*` keys across all 13
  locales; two new Playwright smoke assertions in
  `tests/e2e/workers.spec.ts` (the board + queue + comparison panel, and
  the merge page's query-param seed).

### Added — BFF: cookie sessions + PASETO proxy (2026-06-18)

- Retroactive entry (DOC-4, 2026-08-04): this landed in `f66ff50f`
  alongside a family-wide rename pass but was never logged here. The
  front-end became a **Backend-For-Frontend** per
  [`authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  §6: `/signin` + `/verify` per-app magic-link pages
  (`src/routes/signin/`, `src/routes/verify/+page.server.ts`), an
  httpOnly `__Host-mxi_session` cookie read by `src/hooks.server.ts`
  into `event.locals.sessionId`, and a same-origin reverse proxy
  (`src/routes/api/proxy/[...path]/+server.ts`) that exchanges the
  session for a short-lived PASETO (`src/lib/server/auth.ts`) and
  forwards to the Worker Service. The browser never holds a token —
  `ApiClient`'s base URL points at the proxy (`src/lib/config.ts`),
  unchanged for page code. Server-only config moved to
  `src/lib/server/config.ts` (`WORKER_API_URL`, `AUTH_API_URL`),
  replacing the client-exposed `PUBLIC_API_BASE_URL`. CSRF protection
  on mutating browser→BFF calls (§4 of the same doc) was not part of
  this change — see `spec/13-tasks.md` T-22.

### Added — cross-service links panel (2026-08-03, FE-2)

- The worker detail route gains a **Cross-service links** panel
  (`src/lib/components/LinksPanel.svelte`): it lists the worker's active
  outbound `entity_links` edges, lets an operator assert a new one, and
  withdraws one behind a `confirm()`. These are edges to records in
  *other* services — not the within-service `Worker.links`, which is
  untouched.
- Only the two kinds the service permits a worker to originate are
  offered: `same_identity` (→ a `person` record, the federation
  backbone) and `employed_by` (→ an `organization`, where `role` is the
  job title). Optional `confidence`, `provenance`, `valid_from` and
  `valid_to` are exposed; blank `provenance` defaults to `operator`
  server-side.
- `src/lib/api/links.ts` mirrors the service's `validate_edge` as pure
  functions (`checkToRef`, `checkConfidence`), so a malformed URN or a
  wrong target type is explained inline instead of coming back as a 422.
  The server stays the authority — an unanticipated 422's reason string
  is surfaced verbatim.
- Repository gains `listLinks()` / `createLink()` / `deleteLink()`;
  types gain `EntityLink`, `CreateLinkRequest`, `WorkerEdgeKind`, and
  `EntityRefUrn`.
- i18n: 32 new `links.*` keys across all 13 locales. The `provenance`
  placeholder is deliberately left untranslated — `operator` is the
  literal value the service stores, not UI prose.
- Tests: `tests/unit/links-validation.test.ts` pins the accept/reject
  matrix against the Rust `validate_edge` cases; `tests/unit/workers.test.ts`
  pins the three endpoint URLs, methods, and the 422 reason path; the e2e
  smoke spec stubs the two API calls at the network layer so the panel is
  asserted without a running service.

### Added — drag-to-decide review board (2026-07-19)

- 2026-07-19 — `/review` now loads the **stored** review queue on mount
  (`GET /api/workers/review-queue`, a safe read; the scan button still
  runs the destructive-classed batch scan explicitly) and dragging a
  pending card into Confirmed / Rejected records the decision through
  `POST /api/workers/review-queue/{id}/decision`. Illegal drags are
  refused client-side and the reload restores the stored truth.
- Repository gains `listReviewQueue()` / `decideReview()`; types gain
  `ReviewDecision` + `ReviewQueueListResponse`.
- e2e: the dashboard smoke spec now opens the hamburger dropdown before
  asserting nav links (the nav is toggle-only at every viewport width;
  the old spec predated that layout).

### Fixed

- 2026-07-19 — dedup-report drift: `ReviewStatus` lowered to the wire tokens (see the person entry),
  and the i18n catalogue gains `detail.loading` (x 13 locales) so
  the expiry page uses the family's key rather than the local
  `common.loading` variant.

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
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `WorkerRepository` binding the [Worker Service REST surface](../worker-service-with-loco/agents/restful.md) (`GET /api/health`, CRUD on `/api/workers`, `/search`, `/match`, `/check-duplicates`, `/merge`, `/deduplicate`, `/{id}/audit`, `/{id}/masked`, `/{id}/export`, `/api/audit/recent`).
- **TypeScript types.** Snake-case domain types mirroring [`worker-service-with-loco/agents/models.md`](../worker-service-with-loco/agents/models.md): `Worker`, `HumanName`, `Address`, `ContactPoint`, `Identifier` (MRN/SSN/DL/NPI/PPN/TAX), `IdentityDocument`, `EmergencyContact`, `WorkerLink`, `MatchResult`, `MergeRequest`/`Record`/`Response`, `BatchDeduplicationRequest`/`Response`, `ReviewQueueItem`, `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `WorkerGrid` (SVAR `Grid` with `select` mode + `init`/`select-row`), `HumanNameInput`, `WorkerForm`, `MatchResultsList`.
- **Tests.** 5 Vitest unit tests for `ApiClient`, 3 unit tests for `WorkerRepository`, 6 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../worker-service-with-loco/spec.md`](../worker-service-with-loco/spec/index.md).
- Service REST contract: [`../worker-service-with-loco/agents/restful.md`](../worker-service-with-loco/agents/restful.md).
- Service model types: [`../worker-service-with-loco/agents/models.md`](../worker-service-with-loco/agents/models.md).
- Service matching reference: [`../worker-service-with-loco/agents/matching.md`](../worker-service-with-loco/agents/matching.md).
