# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed — `/verify` crashed with a raw 500 when the authentication service was unreachable (CRM-T24)

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
pass with it restored. See spec §13 CRM-T24.

### Fixed — the follow-ups calendar never showed an upcoming event (CRM-T28)

`@svar-ui/calendar-store` requires an all-day event's `end` to be
strictly after `start`, but `/followups`'s `+page.svelte` passed
`end: day` — the same `Date` object as `start` — so the SVAR Calendar
widget silently dropped every upcoming follow-up it was ever asked to
show. The existing test only asserted the calendar's wrapper was
visible, not that an event actually rendered inside it. The identical
bug was already found and fixed in worker-front-end's/person-front-end's
`/expiry` calendars and ppm-front-end's `/calendar`. Fixed by computing
the following calendar day as `end`; strengthened the test to assert
the calendar renders the event text, with the stubbed due date
computed relative to the actual test-run date. See `spec/tasks.md`
CRM-T28.

### Added — root sign-in gate (CRM-T26)

No `+layout.server.ts` existed anywhere under `src/routes`, so a
visitor with no session reached every page — the deal board, ticket
queue, executive dashboard, all of them — and only discovered they
were signed out once an API call silently failed through the BFF
proxy. Ported workforce-planning-management-front-end's identical
WPM-T38 fix (same underlying architecture): new root
`src/routes/+layout.server.ts` redirects to `/signin` (303) when
`locals.sessionId` is `null`, excluding `/signin`/`/verify`. `tests/e2e/smoke.spec.ts`
gained a `signIn()` helper (injects a fake `__Host-mxi_session` cookie
via Playwright) and a `"sign-in gate (CRM-T26)"` describe; all 12
pre-existing tests moved under a `"signed-in smoke coverage"` describe
whose `beforeEach` now signs in first, verified green (15 Playwright +
5 vitest tests pass). See `../spec/tasks.md` CRM-T26.

### Added — engagement + partners areas (CRM-T20, 2026-07-20)

- `/engagement` (cadence aging, workload with recorded sentiment,
  member health with the silent list) and `/partners` (stakeholder
  register + power–interest grid, partnership register, membership
  renewals). The deal board gains a pipeline selector + the honest
  funnel strip; follow-ups gains a kind filter; DPO gains
  consent-by-account. Nine client functions with path pins; three new
  Playwright specs + extended stubs.

### Added — boards + insight areas (CRM-T19, 2026-07-20)

- `/leads/board` and `/tickets/board` (SVAR Kanban; drag = the
  existing status transitions, SLA/score badges), `/followups`
  (overdue aging + SVAR Calendar), `/executive` (period pack +
  stale deals + hygiene findings + forecast trend), `/dpo` (consent
  coverage + sources + duplicate hygiene). `leadStatus` + seven
  insight client functions with path pins; nav + keys in 13 locales;
  five stubbed Playwright specs.

### Added

- 2026-07-19 — SVAR strong fit: the **/deals** board upgrades from custom CSS columns to the SVAR
  Kanban: columns are the pipeline's stage rows (probability
  labelled), drag = the stage-move API (a lost target carries a
  reason), and the forecast strip still re-reads the derived number
  after every move.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: SVAR index routes: **/contacts** and **/leads** and **/tickets**
  upgrade from plain tables to the SVAR DataGrid with FilterBars
  (the lead score breakdown and ticket status actions move to a
  selection panel under each grid; the breach banner keeps its
  testid), and a new **/accounts** index route (name / tier /
  industry) joins the nav.

- 2026-07-19 — Lily Design System: the Lily Design System lands in the chrome: the hand-rolled
  locale `<select>` is replaced by **LocaleSelect** (wired to the
  i18n store; `applyDir` off), a **ThemeSelect** offers the full
  45-theme catalogue (stylesheets via the `static/assets/themes`
  symlink; choice persisted to `mxi.crm.theme`), and the **Lily
  headless** component library is a dependency.

- 2026-07-18 — CRM-T17/T18 implementation round: SvelteKit 2 +
  Svelte 5 runes SPA with same-origin BFF proxy, 13-locale i18n
  (45 keys, parity-tested, RTL ar/ur), typed API client + honest
  `money()`, and the module views (KPI dashboard, contacts +
  consent + timeline, lead queue + score breakdown, deal board +
  forecast, campaigns + funnel/ROI, tickets + breach flags, KB).
  svelte-check clean; 5 vitest + 4 Playwright tests green.

- 2026-07-18 — CRM-T0 specification round: cross-cutting spec
  (`../spec/`) and this edition's doc scaffold. No code yet; this
  edition is CRM-T17/T18 in the queue.
