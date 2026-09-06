# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed — `/verify` crashed with a raw 500 when the authentication service was unreachable (PF-T19)

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
pass with it restored. See spec §13 PF-T19.

### Fixed — the EDD calendar never showed an expected-discharge event (PF-T24)

`@svar-ui/calendar-store` requires an all-day event's `end` to be
strictly after `start`, but `/edd`'s `+page.svelte` passed `end: day`
— the same `Date` object as `start` — so the SVAR Calendar widget
silently dropped every expected-discharge event it was ever asked to
show, and the route had zero test coverage to catch it. The identical
bug was already found and fixed in worker-front-end's and
person-front-end's `/expiry` calendars. Fixed by computing the
following calendar day as `end`. New Playwright test
(`tests/e2e/board.spec.ts`) stubs one occupied bed with an EDD on the
current month, asserts the event renders, and asserts selecting it
navigates to `/stays/{pid}`. See `spec/tasks.md` PF-T24.

### Security — root sign-in gate (PF-T22)

`src/hooks.server.ts` stashed `locals.sessionId` from the cookie but
nothing redirected an anonymous visitor away from a clinical/PII route
— the whiteboard, stay detail, bed-request board, and locate were all
reachable with no session, with `PATIENT_FLOW_REQUIRE_AUTH` (default
off) the only real gate. New root `src/routes/+layout.server.ts`
redirects to `/signin` when `locals.sessionId === null`, exempting
`/signin`, `/verify`, and any path ending `/kiosk` — the whiteboard's
wall-touchscreen display has no interactive session, so it stays
reachable (with or without `?masked=1`) exactly as before.
`tests/e2e/board.spec.ts` gained a `sign-in gate` describe (anonymous
redirect + kiosk-stays-reachable coverage) alongside the existing
suite, now wrapped in a `signed-in smoke coverage` describe that
injects a fake session cookie. See spec `tasks.md` PF-T22 (repo
`tasks.md` WEB-1).

### Added

- 2026-07-19 — SVAR strong fit: new **/edd** route (nav-linked): every occupied bed's expected
  discharge date across the estate as all-day events in the SVAR
  Calendar (month view, read-only, `as_of`-stamped); selecting an
  entry opens the stay detail.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: new **/wards** index route (linked in the nav): the ward estate in
  the SVAR DataGrid (**@svar-ui/svelte-grid**) with a
  **@svar-ui/svelte-filter** FilterBar (code / ward / kind /
  specialty); row selection opens the ward's whiteboard.

- 2026-07-19 — Lily Design System: the Lily Design System lands in the chrome (non-kiosk nav only):
  a **ThemeSelect** with the full 45-theme catalogue incl. the NHS
  design-system themes (stylesheets via the `static/assets/themes`
  symlink; persisted to `mxi.patient-flow.theme`), a standalone
  **LocaleSelect** owning `lang`/`dir` (RTL for ar/ur; persisted to
  `mxi.patient-flow.locale`; the i18n-ready seam — no translation
  catalogue yet), and the **Lily headless** component library as a
  dependency.

- 2026-07-18 — PF-T17/PF-T18 follow-through: the whiteboard poll is
  now an **ETag conditional GET** (`If-None-Match`; a `304` keeps the
  current render — an idle wall screen costs no body bandwidth; the
  proxy forwards `etag` both ways), and the **BFF session flow**
  landed (copy-adapted from the case front-end): `/signin` magic-link
  request, `/verify` server-side token exchange → httpOnly
  `__Host-mxi_session` cookie, `/signout`, and the proxy exchanges
  the session for a short-lived PASETO bearer. Inert until
  `PATIENT_FLOW_REQUIRE_AUTH` + the auth service are deployed; no
  token ever reaches browser JS.
- 2026-07-18 — PF-T15/T16 implementation round: SvelteKit 2 + Svelte
  5 runes SPA (drift-accepted, copy-adapted from the case front-end)
  with a same-origin BFF proxy (`/api/proxy/*`, `Accepts-version`
  stamped, PF-T18 seam for the PASETO exchange). Routes: home ward
  list, ward whiteboard with polled bed cards (state colours, EDD /
  CCD / pathway / Red2Green / DTOC / infection / alert chips,
  clean-cycle actions), chrome-less kiosk mode (`?masked=1` corridor
  rendering), stay detail (SAFER fields, Red2Green recording,
  infection flags, transfer, discharge-ready, discharge), hospital
  at-a-glance tiles + ward table, bed-request board (queue, ranked
  eligible beds, allocate, cancel, new-request form), patient
  locate, audits + ward handover filter. Tests: 22 vitest `BedCard`
  state × flags cases + 7 Playwright e2e specs over a
  `page.route`-stubbed API; `svelte-check` 0 errors.
- 2026-07-17 — PF-T0 specification round: edition doc scaffold;
  target routes and the bed-card contract fixed in the cross-cutting
  spec ([../spec/whiteboard.md](../spec/whiteboard.md)). No code yet
  — implementation is PF-T15/T16 after the service phases.
