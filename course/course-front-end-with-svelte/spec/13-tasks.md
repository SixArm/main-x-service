## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `course-service-with-loco/agents/models.md`.
- [x] T-3: `ApiClient` + `CourseRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `CourseRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [x] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Contradicts a decision this project already made and states in `AGENTS.md`: `src/routes/+layout.ts` sets `ssr = false` and `prerender = false`, every page is client-rendered, and every entity-API call goes through the same-origin BFF proxy with the browser's session cookie — there is no server-side render in which an `event.fetch` load could run, and no `+page.ts` load exists anywhere in `src/routes/`. The "warm-cache win" would mean re-introducing SSR against the CSR-only + BFF design; if that is ever wanted it is a new decision, not this task.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Instance / syllabus-section edit UI (currently the detail page renders instances read-only; the service ships full CRUD on `/api/courses/{id}/instances/{instance_id}` so it is purely a UI gap). Identifier add/remove already covered by `CourseIdentifierInput`.
- [x] T-16: Theming tokens in `app.css` extracted to a small theme module. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Superseded by the Lily `ThemePicker` adopted 2026-07-31 (`src/routes/+layout.svelte` imports it from `lily-design-system-svelte-theme-picker` and offers the DaisyUI theme ids), which is the theming mechanism now. The 42 `--mxi-*` custom properties in `src/app.css` `:root` are the contract components consume, and CSS is where custom properties belong; a JS "theme module" would re-export what the stylesheet already provides and add a second place for the same values to drift.
- [x] T-17: `check-duplicates` endpoint wired into create form (preview before commit). *(closed as superseded by T-6, 2026-09-03 — repo `tasks.md` WEB-6)* `POST /api/courses` already runs the identical duplicate check (`find_probable_duplicates` is the same function behind both routes) and answers `409` **with the candidate matches, creating nothing** — and `/courses/new` renders them inline (T-6). The create is therefore already the preview: a pre-submit `checkDuplicates()` call would run the matcher a second time to show the same list. What the investigation did surface is different and belongs to the **service**, not this front-end: neither the API nor the form offers any way to create past a `409` — a legitimate near-duplicate can never be created through the UI, because no override exists to wire a button to. That is an authorisation question (an override is `destructive`-class work), recorded under WEB-6 for the service specs to take up if a deployment needs it.
- [ ] T-18: Batch deduplicate-scan results UI.
- [ ] T-19: Masked-view toggle on detail page.
- [x] T-20: GDPR-export download button. *(2026-09-03, repo `tasks.md` WEB-5 — copy-adapted from person's reference)* A button on `/courses/[id]` calls the existing `CourseRepository.exportGdpr(id)` (`GET /api/courses/{id}/export`) and hands the service-defined payload to the browser as a downloaded `course-<id>-export.json` — serialised verbatim, never interpreted, through a Blob object URL and a synthetic anchor (revoked once the click has fired). An `exporting` state disables the button while the request is in flight; errors go to the existing banner. New keys `detail.exportGdpr` / `detail.exportingGdpr` ×13 locales.
  - **Acceptance:** `tests/unit/courses.test.ts` pins `exportGdpr()` GETs `/api/courses/{id}/export` and returns the payload unchanged; a Playwright smoke test stubs the endpoint, clicks the button, awaits the browser `download` event, and asserts both the suggested filename and that the saved bytes parse back to the stubbed payload.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [ ] T-22: Re-add the list-page `phonetic` toggle once the Course Service grows a real Soundex search path (currently a documented no-op; checkbox removed in CHANGELOG v0.2.0; FR-2 + §2 record the omission).
- [ ] T-23: Decide FR-9 preview policy. Today preview is optional ("Load preview" button) and `doMerge()` can run without a preview GET; FR-9 now matches that. If preview should be a hard precondition, gate the Merge button on a successful preview load (code + test change).
- [x] T-24: Auth — adopt BFF + httpOnly cookie; the browser holds only `__Host-mxi_session`, the SvelteKit server attaches a short-lived PASETO server-side; no `mxi_access_token`/`localStorage` bearer, no fragment handoff (per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)). **Landed 2026-06-18** in the family-wide auth-migration commit (`src/hooks.server.ts`, `src/lib/server/{auth,config,session}.ts`, `/signin` + `/verify` routes, the `/api/proxy` reverse proxy injecting a server-exchanged bearer) — this front-end had no §13/§14/§15/CHANGELOG record of it until this DOC-4 pass, and `spec/01`–`spec/03`, `spec/12`, `AGENTS.md`, `.env.example`, and `index.md` all still described auth as unshipped or documented the wrong env vars (`PUBLIC_API_BASE_URL` instead of the real `COURSE_API_URL`/`AUTH_API_URL`); all corrected. **Not done**: CSRF on browser→BFF mutating calls, and no route-level guard redirects an unauthenticated visitor (tracked as T-26).
- [x] T-25 (retroactive record): i18n (13 locales, `src/lib/i18n.svelte.ts`) + Lily `ThemePicker`/`LocalePicker` wired live in the layout shell. Landed in the same 2026-06-18 commit as T-24 but never recorded in this crate's spec/CHANGELOG; `spec/02-scope.md` §2.2 flatly listed both as **out of scope** until this pass. `tests/unit/i18n.test.ts` pins full-key parity across all 13 locales (green). `/signin` and `/verify` are the one exception — their copy is plain English by design (see the code comment in `src/routes/signin/+page.svelte`); T-27 tracks giving them real translations.
- [x] T-26: CSRF protection for browser→BFF mutating requests (synchroniser token per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md) §4) plus a route-level guard so an unauthenticated visitor is redirected rather than silently served every page (today `locals.sessionId`/`signedIn` is exposed to the layout for chrome display only — nothing blocks a request). **Landed 2026-08-28** (PRO-H5), rolling the PROVEN reference pattern from `person-front-end-with-svelte` (commit e73931b2) verbatim: `src/lib/server/session.ts` gains `CSRF_COOKIE`/`CSRF_COOKIE_OPTIONS`/`generateCsrfToken()`/`verifyCsrf()`; `/verify` sets the new non-httpOnly `__Host-mxi_csrf` cookie alongside the session cookie; the root `+page.server.ts` signout action clears both; `src/lib/api/client.ts` echoes the cookie as `X-CSRF-Token` on every non-GET/HEAD request; the proxy (`src/routes/api/proxy/[...path]/+server.ts`) rejects a missing/mismatched token with `403 {"error":"csrf"}`, backstopped by an Origin/Referer check. Tests ported: `tests/unit/session.test.ts` (new), `tests/unit/proxy.test.ts` (new), a CSRF describe block added to `tests/unit/client.test.ts`; `vite.config.ts` gains the jsdom `https://` `environmentOptions` fix the `__Host-` prefix needs to stick under test. **The second half of this task — a route-level guard that redirects an unauthenticated *visitor* away from a page — is deliberately left undone here**, on the same basis it is absent in the proven reference: `person-front-end-with-svelte` (audited, landed) has no such guard either — its `+layout.server.ts` exposes `signedIn` for chrome display only, exactly as course's does today. A related but distinct question was also investigated per this task's brief: whether the BFF proxy itself should reject a non-GET/HEAD request when `locals.sessionId` is absent, rather than forwarding it upstream with no `Authorization` header. Concluded **no** — the family's activation-gate design (`agents/share/security.md` §4) deliberately makes the entity service's own `<ENTITY>_REQUIRE_AUTH` the enforcement point; the reference proxy in person forwards unconditionally too (it only conditionally *attaches* a bearer). Adding a course-only BFF-side 401 would diverge from that shared pattern without closing a gap the family's own design treats as intentional. Both open points (the page-redirect guard and, if reconsidered, a BFF-side session check) are better tracked as a fresh family-wide task alongside `PRO-H5` than solved once, differently, in course alone.
- [ ] T-27: Translate `/signin` and `/verify` copy into the other 12 locales (currently English-only by design, per the code comment).
- [x] T-28 (2026-08-29, PRO-P8): **Bug, fixed.** `src/lib/server/config.ts`'s `COURSE_API_URL` fallback (used only when the env var is unset) was `http://localhost:5150`, the generic loco dev port, but `course-service-with-loco` is the one service in the family whose own dev config overrides that to `8084` (`course-service-with-loco/config/development.yaml`; confirmed live 2026-08-04). A developer who skips `cp .env.example .env` (the quick-start's first step) was silently routed to the wrong port — worse, 5150 is the *shared* family default, so the FE would silently talk to whatever other service happened to be listening there rather than failing loudly. **Decision: fixed the constant to `8084`** rather than adding a boot-time warning — no spec or design doc anywhere in the family documents 5150 as a deliberate choice for course (every reference to it here called it "wrong for this service"), so this reads as a copy-paste default inherited from a sibling front-end's config, not an intentional convention worth merely warning about. `src/lib/server/config.ts`, `.env.example`, `README.md`, `index.md` updated; no test asserted the old fallback value.
- [x] T-29 (2026-08-29, PRO-H10): **Page-visit guard.** Redirect an
  unauthenticated visitor away from every page whose sole purpose is
  submitting a mutation — `/courses/new`, `/courses/[id]/edit`,
  `/courses/merge` each gained a `+page.server.ts` calling the new
  `requireSignedIn(locals)` (`src/lib/server/session.ts`),
  `redirect(303, "/signin")` on no session. Read/list/search/view
  pages stay public — this mirrors the backend's own default-allow-read
  / mutation-deny ABAC posture rather than a separate front-end policy.
  `/courses/[id]`'s embedded soft-delete and `/board`'s drag-to-move
  status change both stay public too, same call as the reference's
  `/persons/[id]`: an otherwise-view page's incidental mutation control
  does not make the page itself mutation-only. Course has no
  bulk-import/export UI and no review-queue route (nothing to guard
  there), and no standalone `CourseInstance` create/edit route yet
  (T-15). `locals.sessionId` is presence-only, a UX convenience in
  front of the backend's real enforcement, not a substitute for it.
  Deliberately does not thread a `next` param back through `/signin` in
  v1 (see `AGENTS.md`'s "Page-visit guard" section for why — the
  magic-link round trip does not carry one today). Ported from the
  `person-front-end-with-svelte` reference implementation. Tests:
  `tests/unit/session.test.ts` (+2 — `requireSignedIn` throws a
  303-to-`/signin` redirect when signed out, passes through silently
  when signed in).
- [x] T-30: Playwright smoke coverage for `/board` and `/calendar`. *(resolved 2026-09-06.)*
  Both routes landed 2026-07-19 (CHANGELOG "SVAR moderate fit") but
  `tests/e2e/courses.spec.ts` (the only e2e suite) never gained a case
  for either. `agents/testing.md` states the smoke suite's goal as
  "every MVP route renders, primary action is clickable"; these two
  are MVP routes (both nav-linked from `+layout.svelte`) that don't
  meet it. *(verified: `grep -rn "board\|calendar"
  tests/e2e/courses.spec.ts` matches nothing —
  `tests/e2e/courses.spec.ts` has exactly 5 route tests — dashboard,
  courses list, new, match, merge — plus the T-20 GDPR test and the
  anonymous-guard describe; none visits `/board` or `/calendar`.)*
  - **Acceptance:** two new Playwright tests — `/board` renders the
    `data-testid="course-board"` Kanban wrap with at least the
    lifecycle column headings; `/calendar` renders
    `data-testid="instance-calendar"` — both stubbing
    `search()`/`listInstances()` via `page.route(...)` per the suite's
    existing convention.
  - **Resolved.** Added `"board renders the Kanban with lifecycle
    column headings"` and `"calendar renders the instance calendar"`
    to `tests/e2e/courses.spec.ts`, stubbing `**/api/courses/search**`
    with an empty `{items:[],total:0}` result — the board's columns
    come from the fixed `COURSE_STATUSES` list independent of course
    data, and an empty result means `/calendar`'s `listInstances()`
    fan-out never fires, so no further stub is needed for either
    shell check. Verified: `npx playwright test` (13 passed, up from
    9), `npm test` (58 passed), `npm run check` (0 errors), `npm run
    lint` clean.
- [x] T-31: Playwright smoke coverage for `/courses/[id]` and
  `/courses/[id]/audit` route shells. Neither is visited as a general
  render check today: `/courses/[id]` is only touched incidentally by
  the T-20 GDPR-export test (which asserts the export button, not the
  page's other landmarks — Edit link, Audit link, identity fields),
  and `/courses/[id]/audit` is never visited by any test at all.
  *(verified: `grep -n "audit" tests/e2e/courses.spec.ts` — no
  match.)*
  - **Acceptance:** a new Playwright test visits a stubbed
    `/courses/{id}` and asserts the heading, the Edit link, and the
    Audit link are visible; a second visits
    `/courses/{id}/audit` (stubbing the audit endpoint) and asserts
    the heading and at least one rendered audit entry.
  - **Resolved.** Added `"course detail renders heading, edit link,
    and audit link"` (stubs `**/api/courses/{id}`, asserts the `<h1>`
    heading equals the stubbed course name plus the "Edit"/"Audit"
    links) and `"course audit page renders heading and an audit
    entry"` (stubs `**/api/courses/{id}/audit**` with one entry,
    asserts the "Audit log" heading and the entry's action text) to
    `tests/e2e/courses.spec.ts`.
- [ ] T-32: `/courses` list has no pagination beyond a hardcoded
  `limit: 50` and no total-count indicator. `src/lib/api/courses.ts`'s
  `search()` and `src/lib/api/client.ts`'s `ApiClient` never read the
  family-wide `X-Total-Count`/`X-Limit`/`X-Offset` response headers
  ([`restful.md`](../../../agents/share/restful.md));
  `src/routes/courses/+page.svelte` calls `repo.search({ q: q.trim(),
  limit: 50, fuzzy })` with no offset control. A deployment with more
  than 50 courses gets a silently truncated grid with no way to see
  the rest and no on-screen indication that more exist. *(verified:
  `grep -n "X-Total-Count\|X-Limit\|X-Offset\|totalCount"
  src/lib/api/*.ts src/routes/courses/+page.svelte` — no matches;
  `SearchOptions.offset` is declared in `src/lib/api/courses.ts` but
  never surfaced in the UI.)*
  - **Acceptance:** `ApiClient` exposes response headers (or a parsed
    `{items, total, limit, offset}`) on `search()`;
    `/courses/+page.svelte` gains Prev/Next controls and a "showing X
    of Y" label driven by `X-Total-Count`; `tests/unit/courses.test.ts`
    pins the header parse, a Playwright test pins the Next button
    advancing `offset`.
- [ ] T-33: `/board` and `/calendar` silently truncate at 100 / 50
  courses respectively, with the same root cause as T-32 but a worse
  symptom — a Kanban column or a calendar month can look complete
  while quietly missing records past the cap, rather than a list
  simply lacking a "next page" control.
  `src/routes/board/+page.svelte` calls `repo.search({ q: "*", limit:
  100 })`; `src/routes/calendar/+page.svelte` calls `repo.search({ q:
  "*", limit: 50 })`. Neither reads `X-Total-Count` to detect or
  surface truncation. *(verified: `grep -n "limit"
  src/routes/board/+page.svelte src/routes/calendar/+page.svelte` →
  `limit: 100` and `limit: 50`.)*
  - **Acceptance:** once T-32 exposes the total count, both pages
    compare it against the fetched item count and render a visible
    "showing N of M — refine search to see more" notice (or raise the
    cap and paginate the underlying fetch) rather than presenting a
    silently partial board/calendar as complete; a unit or e2e test
    pins the notice appears when `total > items.length`.

