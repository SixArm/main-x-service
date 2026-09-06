## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `event-service-with-loco/agents/models.md`.
- [x] T-3: `ApiClient` + `EventRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `EventRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [x] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Contradicts a decision this project already made and states in `AGENTS.md`: `src/routes/+layout.ts` sets `ssr = false` and `prerender = false`, every page is client-rendered, and every entity-API call goes through the same-origin BFF proxy with the browser's session cookie — there is no server-side render in which an `event.fetch` load could run, and no `+page.ts` load exists anywhere in `src/routes/`. The "warm-cache win" would mean re-introducing SSR against the CSR-only + BFF design; if that is ever wanted it is a new decision, not this task.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Sub-record edit for identifiers / locations / parties (organizers, performers) / offers (currently read-only on detail; edit form re-PUTs the whole record but has no UI to add/remove sub-records). (Rewritten 2026-06-13: previous wording named person-service sub-records — addresses / emergency contacts — which Event does not have.)
- [x] T-16: Theming tokens in `app.css` extracted to a small theme module. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Superseded by the Lily `ThemePicker` adopted 2026-07-31 (`src/routes/+layout.svelte` imports it from `lily-design-system-svelte-theme-picker` and offers the DaisyUI theme ids), which is the theming mechanism now. The 42 `--mxi-*` custom properties in `src/app.css` `:root` are the contract components consume, and CSS is where custom properties belong; a JS "theme module" would re-export what the stylesheet already provides and add a second place for the same values to drift.
- [x] T-17: `check-duplicates` endpoint wired into create form (preview before commit). *(closed as superseded by T-6, 2026-09-03 — repo `tasks.md` WEB-6)* `POST /api/events` already runs the identical duplicate check (`check_duplicates_internal` is the same function behind both routes) and answers `409` **with the candidate matches, creating nothing** — and `/events/new` renders them inline (T-6). The create is therefore already the preview: a pre-submit `checkDuplicates()` call would run the matcher a second time to show the same list. What the investigation did surface is different and belongs to the **service**, not this front-end: neither the API nor the form offers any way to create past a `409` — a legitimate near-duplicate can never be created through the UI, because no override exists to wire a button to. That is an authorisation question (an override is `destructive`-class work), recorded under WEB-6 for the service specs to take up if a deployment needs it.
- [ ] T-18: Batch deduplicate-scan results UI.
- [x] T-19: Masked-view toggle on detail page. *(resolved 2026-09-06)*
  `EventRepository.masked()` (`GET /api/events/{id}/masked`) already
  existed but was never wired into `/events/[id]/+page.svelte` — the
  detail page only ever called `repo.get(id)`, unlike place-front-end's
  equivalent T-19, which was the copy-adapt source here. A button in
  the header (`aria-pressed={masked}`, labelled "Show masked"/"Show
  full") re-fetches through the masked endpoint on toggle rather than
  redacting client-side, and a banner ("Showing the masked view — some
  fields are redacted.") appears while the masked view is shown. New
  i18n keys `detail.showMasked` / `detail.showFull` /
  `detail.maskedNotice` across all 13 locales (reusing place-front-end's
  translations for parity). A new `tests/unit/events.test.ts` case pins
  `masked()` GETs `/api/events/{id}/masked`; a new
  `tests/e2e/events.spec.ts` case stubs plain vs. masked responses
  (distinguished by `/masked` in the URL, different `description`
  values) and toggles both ways, verified to fail (timeout waiting for
  the "Show masked" button) with the page change reverted and pass with
  it restored. `npm test` 61/61 (was 60); `npx playwright test` 14/14
  (was 13); `npm run check` / `npm run lint` clean.
- [x] T-20: GDPR-export download button. *(2026-09-03, repo `tasks.md` WEB-5 — copy-adapted from person's reference)* A button on `/events/[id]` calls the existing `EventRepository.exportGdpr(id)` (`GET /api/events/{id}/export`) and hands the service-defined payload to the browser as a downloaded `event-<id>-export.json` — serialised verbatim, never interpreted, through a Blob object URL and a synthetic anchor (revoked once the click has fired). An `exporting` state disables the button while the request is in flight; errors go to the existing banner. New keys `detail.exportGdpr` / `detail.exportingGdpr` ×13 locales.
  - **Acceptance:** `tests/unit/events.test.ts` pins `exportGdpr()` GETs `/api/events/{id}/export` and returns the payload unchanged; a Playwright smoke test stubs the endpoint, clicks the button, awaits the browser `download` event, and asserts both the suggested filename and that the saved bytes parse back to the stubbed payload.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [ ] T-22: Phonetic search toggle on the list page — **blocked on the service**. `event-service-with-loco` exposes `q` / `fuzzy` / `mask_sensitive` / date / status / type on `GET /events/search` but no `phonetic` search parameter (Soundex is internal to the matcher's name scoring, not a search query param). Surface a phonetic toggle here only once the service search query accepts one; until then `SearchOptions` carries no `phonetic` field.
- [x] T-23a: Auth — BFF + httpOnly cookie: `/signin` + `/verify` (per-app magic-link), `src/lib/server/{session,auth,config}.ts`, `/api/proxy/[...path]` reverse proxy injecting a server-exchanged PASETO. The browser holds only `__Host-mxi_session`; no `mxi_access_token`/`localStorage` bearer, no fragment handoff (per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
- [x] T-23b: CSRF protection on mutating browser→BFF calls (double-submit cookie per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md) §4) — the remaining half of the original T-23, split out once the cookie/PASETO half landed. Landed 2026-08-28: `__Host-mxi_csrf` (non-httpOnly) set alongside the session cookie at `/verify`, echoed as `X-CSRF-Token` by `ApiClient` on mutating requests, verified by the `/api/proxy` handler (double-submit + Origin/Referer backstop), cleared on signout. See §16 OQ-3.
- [ ] T-24: i18n coverage for `/signin` and `/verify` — currently plain English only (see the in-file comment on `src/routes/signin/+page.svelte`), unlike the rest of the app's 13-locale coverage.
- [x] T-25 (2026-08-29, PRO-H10): **Page-visit guard.** Redirect an
  unauthenticated visitor away from every page whose sole purpose is
  submitting a mutation — `/events/new`, `/events/[id]/edit`,
  `/events/merge` each gained a `+page.server.ts` calling the new
  `requireSignedIn(locals)` (`src/lib/server/session.ts`),
  `redirect(303, "/signin")` on no session. Read/list/search/view
  pages stay public — this mirrors the backend's own default-allow-read
  / mutation-deny ABAC posture rather than a separate front-end policy.
  `/calendar` also stays public: despite writing back on drag-to-
  reschedule, its entire purpose is viewing the schedule, not
  submitting a mutation. Event has no `/events/bulk` or `/review`
  route (no bulk import/export, no review queue for this entity), so
  neither exists to guard, unlike `person-front-end-with-svelte`'s
  parallel task. `locals.sessionId` is presence-only, a UX convenience
  in front of the backend's real enforcement, not a substitute for it.
  Deliberately does not thread a `next` param back through `/signin` in
  v1 (see `AGENTS.md`'s "Page-visit guard" section for why — the
  magic-link round trip does not carry one today). Tests:
  `tests/unit/session.test.ts` (+2 — `requireSignedIn` throws a
  303-to-`/signin` redirect when signed out, passes through silently
  when signed in).
- [ ] T-26: Doc sync — CSRF and GDPR-export are landed but several docs
  still describe them as pending or out of scope. `AGENTS.md`'s "What
  does NOT live here" still lists "GDPR-export download UI. Out of
  scope for MVP." though T-20 landed it 2026-09-03; `spec/02-scope.md`
  and `spec/12-compliance.md` echo the same stale claim. Separately,
  `index.md`, `spec/02-scope.md`, `spec/03-stakeholders-and-users.md`,
  `spec/08-architecture.md`, `spec/12-compliance.md`,
  `spec/14-implementation-status.md`, and `spec/15-roadmap.md` all
  still say CSRF is "not yet implemented" / "not yet done" even though
  T-23b (above) has been `[x]` since 2026-08-28. `spec/14-implementation-status.md`'s
  test count is also stale (says 34; the suite is actually 62
  `it`/`test` cases across 7 files). *(verified: `grep -n "CSRF"
  index.md spec/*.md` and `grep -n "GDPR" AGENTS.md spec/*.md` both
  return hits contradicting T-20/T-23b's `[x]` status; `grep -c
  "it(\|test(" tests/unit/*.test.ts` sums to 62, not 34.)*
  - **Acceptance:** every doc listed above reflects T-20 and T-23b as
    landed (with their dates), `spec/14-implementation-status.md`'s
    Auth — CSRF row flips to ✅ and the test-count/file-list lines
    match the real suite, and `README.md`'s Project layout test list
    gains `proxy.test.ts` + `session.test.ts`. Doc-only; no behaviour
    change.
- [x] T-27: `/calendar` had zero test coverage — no Playwright smoke
  test and no unit test, despite writing back through the update
  endpoint on drag-to-reschedule. *(2026-09-06)* Writing the required
  "the calendar actually shows an event" assertion surfaced a genuine,
  previously-undiscovered product bug rather than a mere coverage gap:
  `@svar-ui/calendar-store` requires an all-day event's `end` to be
  strictly *after* `start` (confirmed against the compiled library
  source, the same root cause worker-front-end's `/expiry` calendar
  had), and `+page.svelte` was passing an all-day event's `end_date`
  straight through with no exclusive-end adjustment — so a same-day
  all-day event (`end_date` equal to `start_date`, or absent) was
  silently dropped by the widget every time. Fixed in
  `src/routes/calendar/+page.svelte`: all-day events now get a
  computed exclusive end one calendar day past the later of their
  start/end day, so a same-day event gets a one-day span and a
  genuinely multi-day all-day event keeps its full width; timed events
  are unaffected. New Playwright test stubs one timed and one
  same-day-all-day event on the current month (a fixed future date
  would eventually scroll out of the widget's default view), asserts
  both render, clicks the all-day one, and asserts navigation to its
  detail page.
  - **Residual, not silently dropped**: the acceptance criteria also
    asked for a drag-driven `PUT` assertion covering the
    reschedule-writes-back path. `Calendar`'s `update-event` handler is
    wired only through the widget's own internal drag gesture, which
    SVAR exposes no headless test hook for and which Playwright cannot
    reliably simulate against a real drag-and-drop calendar grid — so
    that half of T-27 is not delivered here and is not tracked under a
    new task number; a future pass should pick it up if `update-event`
    coverage becomes a real need.
- [ ] T-28: The BFF auth flow itself — `/signin`, `/verify`, and the
  sign-out form action (`src/routes/+page.server.ts`) — has no direct
  test coverage. `tests/unit/session.test.ts` covers only the pure
  helpers (`verifyCsrf`, `requireSignedIn`); nothing renders `/signin`
  or `/verify`, and nothing exercises the `signout` action that clears
  `SESSION_COOKIE`/`CSRF_COOKIE` and calls `signout()`.
  *(verified: `grep -rln "signin\|verify" tests/` returns only
  `session.test.ts` and `events.spec.ts` — the latter mentions
  `/signin` solely as the redirect target in guard tests; `grep -rn
  "signout" tests/` returns nothing.)*
  - **Acceptance:** Playwright smoke tests for `/signin` (form renders,
    submits to the magic-link request) and `/verify` (renders its
    landing states); a unit or component test for the `signout` action
    asserting it clears both cookies and redirects to `/`.

- [x] **T-29: `/verify` crashed with a raw 500 when the authentication service was unreachable.** *(resolved 2026-09-06.)* `src/routes/verify/+page.server.ts` called `await verifyMagicLink(fetch, token)` with no `try`/`catch`. A network-level failure (the authentication service unreachable, timed out, connection reset) makes `fetch` throw rather than resolve — uncaught, that propagated out of `load` and SvelteKit rendered its generic 500 error page instead of this route's own friendly UI. The same bug class was found and fixed first in `place-front-end-with-svelte` (T-26) and `thing-front-end-with-svelte` (T-23); ported here.
  - **Resolved.** A `try`/`catch` around the call, a new `"serviceUnavailable"` error variant, and its message in `+page.svelte`.
  - **Acceptance:** `tests/unit/verify.test.ts` (new) unit-tests the `load` function directly — pinning `missingToken`, the new `serviceUnavailable` (fetch rejects), and `invalidToken` (non-ok response) branches — verified to fail with the `try`/`catch` reverted and pass with it restored. Three-part change: spec (here) + code + test.
