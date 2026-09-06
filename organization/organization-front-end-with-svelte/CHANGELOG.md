# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added — masked view + GDPR export UI on the detail page (ORGFE-T2)

The service exposes `GET /{pid}/masked` and `GET /{pid}/export`, but
nothing in this front-end called either. Added
`OrganizationRepository.masked(pid)` / `.exportGdpr(pid)`, a
detail-page "Show masked"/"Show full" toggle that re-fetches through
the masked endpoint on click rather than redacting client-side
(copy-adapted from place/event-front-end's equivalent), and an "Export
data (GDPR)" button that downloads the export envelope as
`organization-<pid>-export.json`. New i18n keys `detail.showMasked` /
`detail.showFull` / `detail.maskedNotice` / `detail.exportGdpr` /
`detail.exportingGdpr` across all 13 locales. New unit + e2e tests,
verified to fail without the fix. `pnpm test` 77/77 (was 75); `pnpm
exec playwright test` 12/12 (was 10). See spec/index.md ORGFE-T2.

### Fixed — `/verify` crashed with a raw 500 when the authentication service was unreachable (ORGFE-T6)

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
pass with it restored. See spec §13 ORGFE-T6.

## [0.1.0] - 2026-08-04
### Added — `/review` upgraded to the person T-25 standard (2026-08-04, FE-4)

- `?status=`/`?limit=` filters on `GET /api/organizations/review-queue`
  (there is no `offset`; `"all"` is the *absence* of `status` — a
  literal `"all"` gets the service's own `422`, confirmed against
  `controllers/organizations.rs::get_review_queue` rather than assumed).
- A keyboard-reachable `<table>` of the same queue items, each with a
  real `Compare` button, alongside the existing SVAR Kanban
  drag-to-decide board (mouse-only drag is not a keyboard path).
  `provenance` is now surfaced on both the Kanban card description and
  the table.
- An inline (non-modal) side-by-side comparison panel: two parallel
  `GET /api/organizations/{pid}` calls, plus a **live** per-component
  score breakdown from `POST /api/organizations/match` — this crate's
  stored `ReviewQueueItem` never carries `score_breakdown` on the wire
  (the DB column exists; the controller's response struct omits it), so
  the panel recomputes one against the loaded pair rather than reading a
  value that never arrives. Real `Confirm`/`Reject` buttons, disabled
  once an item is no longer `pending` (mirrors the service's
  first-writer-wins guard).
- Confirming does **not** merge (the decision endpoint is a pure status
  change). A confirmed item now shows a deep link to `/merge` with both
  ids pre-filled, in either survivor order — and `/merge` itself gained
  the `?main=&duplicate=` prefill (`$app/state`'s `page.url`) that link
  relies on.
- New `src/lib/review.ts`: pure helpers (`REVIEW_STATUSES`,
  `REVIEW_LIMITS`, `isReviewStatus`, `canDecide`, `MATCH_COMPONENTS` —
  this matcher's six weighted components summing to 1.00 —
  `breakdownRows`, `mergeHref`).
- `ReviewQueueItem` gained the `provenance` field it was missing in
  TypeScript (the server has carried it since BLK-5); `OrganizationRepository`
  gained `matchAgainst()` and a `listReviewQueue(options)` overload;
  `types.ts` gained `MatchBreakdown`/`MatchResult`/`MatchRankResult`.
- 57 new i18n keys (`review.*`) across all 13 locales, coverage-tested.
- Tests: `tests/unit/review.test.ts` (15, new), extended
  `tests/unit/organizations.test.ts` (`listReviewQueue` query-string
  pins + `matchAgainst`), three new Playwright cases (the keyboard
  table, the live-breakdown comparison, the merge query-string
  prefill). Suite is now 72 vitest + 10 Playwright.

### Added — `/merge` record-merge UI (2026-08-03, FE-1)

- `/merge` — fold a duplicate into a surviving main record: main pid +
  duplicate pid + optional reason, an optional side-by-side preview
  (`GET /{pid}` × 2), a `confirm()`-gated `POST /api/organizations/merge`,
  and a merge-history table from `GET /api/organizations/merges/recent`
  (loads on mount, refreshes after a merge).
- This service's merge wire shape has **no** `merge_record` wrapper —
  the response is `{main_pid, duplicate_pid, main}` — so the completion
  panel links straight to the survivor rather than quoting a
  merge-record id, unlike the six sibling front-ends (person, worker,
  place, thing, event, course) that do carry that wrapper.
- Validation (`src/lib/components/merge-validation.ts`) is a pure,
  unit-tested guard (both ids present, must differ) returning an i18n
  **key**, not a hardcoded English string.
- Repository gains `merge()` / `recentMerges()`; types gain
  `MergeRequest`, `MergeResponse`, `MergeRecordRow`; nav gains Merge;
  i18n gains 27 keys (`nav.merge` + 26 `merge.*`) across all 13
  locales.
- Tests: `tests/unit/merge-validation.test.ts` (4), extended
  `tests/unit/organizations.test.ts` (`merge`/`recentMerges` path
  pins), and three new Playwright smoke cases in `tests/e2e/smoke.spec.ts`
  covering the merge page and its nav entry. Suite is now 54 vitest +
  7 Playwright.

### Added — paged collection reads (2026-08-01)

- **`ApiClient.getPage()`** returns `{ items, total, limit, offset }`,
  reading the service's `X-Total-Count` / `X-Limit` / `X-Offset` headers.
  The plain `get()` throws response headers away, which is fine for one
  record and useless for a collection. A service that predates the
  headers still works: the page length is the fallback.
- **`OrganizationRepository.listPage()`** wraps it; `list()` is unchanged
  for callers that just want the default page.
- The organizations list route shows the row count — `shown / total` when
  the page is not the whole collection, so a first page of a long list
  cannot be mistaken for a short list.


### Added — drag-to-decide review board (2026-07-19)

- `/review` — the stored review queue as a SVAR Kanban board
  (Pending / Confirmed / Rejected / AutoMerged), mirroring the
  person/worker/place/thing boards: the queue loads on mount (safe
  GET), the destructive-classed batch scan runs only on the button,
  and dragging a pending card into Confirmed / Rejected records the
  decision through the decision endpoint.
- Repository gains `deduplicate()` / `listReviewQueue()` /
  `decideReview()`; types gain `ReviewStatus`, `ReviewDecision`,
  `ReviewQueueItem`, `ReviewQueueListResponse`,
  `BatchDeduplicationResponse`; nav gains Review; i18n gains
  `nav.review` / `review.run` in all 13 locales.
- e2e: a stubbed review-board smoke pins that loading the page renders
  the stored queue and never fires the destructive scan.

### Added

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: new **/organizations** index route: the organization list in the
  SVAR DataGrid (**@svar-ui/svelte-grid**) with a
  **@svar-ui/svelte-filter** FilterBar (client-side name filter);
  row selection opens the detail route.

- 2026-07-19 — Lily Design System: the hand-rolled locale `<select>` is replaced by the Lily
  **LocaleSelect** (wired to the i18n store; `applyDir` off), and
  the **Lily headless** component library is now a dependency
  alongside the existing ThemeSelect.

### Changed

- **Auth pivot — BFF + cookie session + PASETO (spec-level; code
  follow-up pending).** The family is moving off the browser-held RS256
  JWT (cross-origin `#access_token` fragment handoff,
  `localStorage["mxi_access_token"]`) to a **Backend-For-Frontend**: the
  browser holds only an httpOnly `__Host-mxi_session` cookie, the
  front-end's own SvelteKit server exchanges the session for a
  short-lived **PASETO v4.public** token and calls the organization
  service server-side, and mutating requests are CSRF-protected. RS256
  JWT + JWKS are decommissioned. Human-facing docs (README/agents/index)
  updated to describe the target model; the current runtime still uses
  the older client-held-token flow and the code follow-up is tracked in
  spec §13. Source of truth:
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).

- **Docs/tests harmonization pass.** Brought the doc set back in line
  with the implemented bearer-token + SSO handoff increments: spec §2
  now scopes in the opt-in session/SSO (was "auth out of scope"), §8
  enumerates the payload incl. `telephone`/`email` and points at the new
  `build.ts`, and §11/§13 record the suite at 49 tests across 5 files.
  AGENTS.md gained `auth.svelte.ts`, `build.ts`, `VITE_AUTH_FRONTEND_URL`/
  `signInUrl`, the `tests/` tree, a Session/SSO section, and
  `pnpm test`/`pnpm test:e2e`. README documents `telephone`/`email` and
  the test commands. index.md adds a worked SSO-handoff diagram and an
  Organization payload JSON example (incl. the `{Custom: label}`
  identifier variant).

### Added

- **Form/payload core extracted + tested.** `OrganizationForm.build()`
  and its helpers moved into a pure `src/lib/api/build.ts`
  (`buildOrganization` + `splitList`/`blankToUndef`) so the spec §8 core
  is unit-testable without mounting the component; the §6.6 self-match
  filter is now `excludeSelf` in the same module (used by the detail
  route). New `tests/unit/build.test.ts` (14) covers comma-list
  splitting, blank→null clearing, contact fields, all-or-nothing
  address, dropping empty identifier rows, and self-match exclusion.
  `tests/unit/auth.test.ts` gained `captureFromLocation` coverage
  (store-write + fragment-strip / no-op). Suite is now 49 unit tests.
- **Cross-origin SSO token handoff (consumer side).** The operator now
  obtains a token from the central authentication front-end instead of
  pasting it. `signInUrl()` (`src/lib/config.ts`, new
  `VITE_AUTH_FRONTEND_URL`) builds
  `${VITE_AUTH_FRONTEND_URL}/signin?return_to=<encoded origin+base>`; the
  layout shows a primary **Sign in** button when signed out. On app load
  the layout's `onMount` runs `captureFromLocation()` (new in
  `auth.svelte.ts`) before any API call: `captureTokenFromHash` parses
  `…#access_token=<jwt>` out of the URL fragment (URL-decoded), `setToken`
  stores it, and `history.replaceState` strips the fragment. The manual
  paste field is kept (behind a "Paste a token" disclosure) as a dev
  fallback. vitest covers `captureTokenFromHash` (extract / decode /
  null cases) and `signInUrl` (encoded `return_to`, trailing-slash
  safe). Implements the "Token acquisition handoff" section of
  `agents/share/jwt-enforcement.md`.
- **Bearer-token session.** New reactive token store
  `src/lib/auth.svelte.ts` (`setToken`/`clearToken`/`token`), hydrated
  from the family-shared `localStorage["mxi_access_token"]` key and
  guarded for SSR/preview. `ApiClient` now attaches `Authorization:
  Bearer <token>` from the store on every request when signed in and
  omits it otherwise (an explicit per-request `token` still overrides;
  pass `null` to suppress). A minimal session affordance in the layout
  sidebar lets an operator paste/clear the token ("Use token" /
  "Sign out"). The token is obtained out-of-band from the central
  authentication-service; full magic-link redirect is a follow-up.
  vitest covers store round-trip + store-driven / override / cleared
  header attachment. Implements `agents/share/jwt-enforcement.md`
  (service enforcement stays off by default).
- **Test suites (T-11).** vitest unit tests (`tests/unit/`) for the
  `ApiClient` and `OrganizationRepository` — verb/path/body/bearer-token,
  error classification, and a regression pinning the `check-duplicates`
  path. Playwright smoke tests (`tests/e2e/`, 4) load the four routes
  with the API stubbed via `page.route`; they run against the
  production build (`vite preview`) to dodge the `vite dev` cold-start
  module-load race. `playwright.config.ts` added.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.
- Copy-paste artifacts from the scaffold source: `client.ts` header
  said "Authentication Service"; `app.html` description said "Course
  Service" — both now read "Organization Service".

### Added (scaffold)

- **Inaugural scaffold (v0.1.0).** SvelteKit 2 / Svelte 5 (runes) SPA
  for the Organization Service, copy-adapted from
  authentication-front-end-with-svelte (same loco raw-JSON client).
  - Routes: `/` (list), `/new` (create), `/[pid]` (detail + delete +
    check-duplicates), `/[pid]/edit` (edit).
  - Lean API client extended with `put`/`delete`; `OrganizationRepository`
    (list/get/create/update/remove/checkDuplicates).
  - `types.ts` mirrors `organization_matcher::Organization` (the service
    DTO), including `IdentifierScheme` and `PostalAddress`.
  - `OrganizationForm` editing scalars, comma-list fields, a postal
    address, and a simple identifiers editor (unit-variant schemes).
  - SPA mode (`+layout.ts`); dependency-light (no SVAR/Lily). `pnpm run
    check` clean (0/0); production build succeeds.

### Configuration

- `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).
- `VITE_AUTH_FRONTEND_URL` (default `http://localhost:5173`) — base URL of
  the central authentication front-end for the SSO sign-in handoff.
