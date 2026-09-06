## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `thing-service-with-loco/agents/models.md`.
- [x] T-3: `ApiClient` + `ThingRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `ThingRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [x] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Contradicts a decision this project already made and states in `AGENTS.md`: `src/routes/+layout.ts` sets `ssr = false` and `prerender = false`, every page is client-rendered, and every entity-API call goes through the same-origin BFF proxy with the browser's session cookie — there is no server-side render in which an `event.fetch` load could run, and no `+page.ts` load exists anywhere in `src/routes/`. The "warm-cache win" would mean re-introducing SSR against the CSR-only + BFF design; if that is ever wanted it is a new decision, not this task.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Edit UI for the remaining Thing fields — `images`, `main_entity_of_page`, `subject_of`, `potential_action` (the edit form re-PUTs the whole record, so these round-trip unchanged; identifiers, alternate names, and same-as URLs are already editable).
- [x] T-16: Theming tokens in `app.css` extracted to a small theme module. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Superseded by the Lily `ThemePicker` adopted 2026-07-31 (`src/routes/+layout.svelte` imports it from `lily-design-system-svelte-theme-picker` and offers the DaisyUI theme ids), which is the theming mechanism now. The 42 `--mxi-*` custom properties in `src/app.css` `:root` are the contract components consume, and CSS is where custom properties belong; a JS "theme module" would re-export what the stylesheet already provides and add a second place for the same values to drift.
- [x] T-17: `check-duplicates` endpoint wired into create form (preview before commit). *(closed as superseded by T-6, 2026-09-03 — repo `tasks.md` WEB-6)* `POST /api/things` already runs the identical duplicate check (both handlers score the same candidates against the same `state.matcher.threshold()`) and answers `409` **with the candidate matches, creating nothing** — and `/things/new` renders them inline (T-6). The create is therefore already the preview: a pre-submit `checkDuplicates()` call would run the matcher a second time to show the same list. What the investigation did surface is different and belongs to the **service**, not this front-end: neither the API nor the form offers any way to create past a `409` — a legitimate near-duplicate can never be created through the UI, because no override exists to wire a button to. That is an authorisation question (an override is `destructive`-class work), recorded under WEB-6 for the service specs to take up if a deployment needs it.
- [x] T-18: Batch deduplicate-scan results UI — `/review` (SVAR Kanban: Pending / Confirmed / Rejected / AutoMerged), landed 2026-07-19; drag-to-decide against `POST /api/things/review-queue/{id}/decision` landed the same day. The board was never otherwise specified or tested until T-24 below backfilled the filter, keyboard path, and comparison panel.
- [x] T-19: Masked-view toggle on detail page. *(2026-09-03)* A toggle
  button in the `/things/[id]` header re-fetches through the existing
  `ThingRepository.masked(id)` (`GET /api/things/{id}/masked`) rather
  than redacting fields client-side — the server decides what counts as
  sensitive, mirroring person's, worker's, and place's identical T-19
  delivery. Shows a `role="status"` banner while the masked view is
  active, so a screenshot or a glance at the page makes the mode
  unambiguous. Toggling back re-fetches the plain record rather than
  caching the pre-toggle state, so a concurrent edit is never shown
  stale. New keys `detail.showMasked` / `detail.showFull` /
  `detail.maskedNotice`, translated across all 13 locales. The
  repository method and its unit test already existed on `main` — only
  the UI toggle was missing.
  - **Acceptance:** the pre-existing `tests/unit/things.test.ts`
    `masked()` test already pinned the endpoint; a new Playwright smoke
    test (`tests/e2e/things.spec.ts`) stubs the plain and masked
    endpoints with visibly different `owner` values and asserts the
    toggle switches between them and shows/hides the masked-view
    banner.
- [x] T-20: GDPR-export download button. *(2026-09-03, repo `tasks.md` WEB-5 — copy-adapted from person's reference)* A button on `/things/[id]` calls the existing `ThingRepository.exportGdpr(id)` (`GET /api/things/{id}/export`) and hands the service-defined payload to the browser as a downloaded `thing-<id>-export.json` — serialised verbatim, never interpreted, through a Blob object URL and a synthetic anchor (revoked once the click has fired). An `exporting` state disables the button while the request is in flight; errors go to the existing banner. New keys `detail.exportGdpr` / `detail.exportingGdpr` ×13 locales.
  - **Acceptance:** `tests/unit/things.test.ts` pins `exportGdpr()` GETs `/api/things/{id}/export` and returns the payload unchanged; a Playwright smoke test stubs the endpoint, clicks the button, awaits the browser `download` event, and asserts both the suggested filename and that the saved bytes parse back to the stubbed payload.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [x] T-22: Auth — BFF + httpOnly `__Host-mxi_session` cookie + session→PASETO exchange; the browser never holds a token (per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)). Landed 2026-07-04 (`f66ff50f`): `hooks.server.ts`, `/signin`, `/verify`, `/api/proxy/[...path]`, `src/lib/server/{config,session,auth}.ts`. **CSRF is not yet implemented** — the BFF has no `X-CSRF-Token`/synchroniser-cookie check on mutating browser→BFF calls; tracked as a follow-up rather than closed silently under this checkbox.
- [ ] T-23: E2E coverage for the BFF pages — `tests/e2e/things.spec.ts` (7 tests, `/review` covered as of the note under T-24 below) still has no Playwright smoke test for `/signin` or `/verify`.
- [x] T-24 (repo FE-4): **Duplicate review-queue screen** at `/review` — 2026-08-04. Extends T-18's board (Kanban only, drag-to-decide only, no filter, no comparison, no keyboard path) to close the gaps person's own FE-4 fan-out (T-25) established as the family pattern: a **status + page-size filter** wired to `?status=` / `?limit=` (`listReviewQueue` gained a `ReviewQueueOptions` argument; "all" is the *absence* of `status`, since the endpoint answers `422 INVALID_STATUS` for a token it does not know, and there is no `offset` so page size is the whole pagination story); a **keyboard-reachable path** — the SVAR Kanban's drag-to-decide is mouse-only, so a native queue table with a per-row `Compare` button was added alongside it, and the panel carries real `Confirm` / `Reject` buttons (drag still works, it is simply no longer the only way); and an **inline side-by-side comparison** loading both things with two parallel `GET /api/things/{id}` calls (`Promise.allSettled`, so a soft-deleted side still renders the other), rendering id / name / additional type / description / url / owner / primary identifier / primary same-as, plus the matcher's `score_breakdown` as a component / weight / score table and its two boolean flags. Scope decisions forced by the service: **confirming does not merge** (the decision endpoint is a pure status change and the service records no link to a merge), so the panel deep-links `/things/merge?main=…&duplicate=…` — in **either** survivor order, because a review item names an unordered pair — and `/things/merge` gained a one-line `?main=`/`?duplicate=` seed (FR-17). **Two verified, documented gaps versus person's reference** (checked against this service's own `src/api/rest/handlers.rs` and `src/db/review_queue.rs` rather than assumed): the wire `ReviewQueueItem` has **no `provenance` column at all** (the `review_queue` table itself has none, unlike person / worker / place / organization), so the queue surfaces `detection_method` in its place rather than fabricating a field the service does not have (FR-18); and the wire type **never serializes `score_breakdown`**, even though the stored row has the column — its one writer (`deduplicate`) always writes it `None` — so the breakdown table renders its documented "no breakdown was recorded" empty state for every live item today (FR-15). `score_breakdown` was declared `?: unknown` on the TS `ReviewQueueItem` (forward-compatible, not a claim the field is sent) so the front end needs no further type change the day the service wires the column through. New pure module `src/lib/review.ts` (status vocabulary, `canDecide`, the five weighted `MATCH_COMPONENTS` — name 0.40 / identifier 0.30 / description 0.10 / url 0.10 / same_as 0.10, matching `src/matching/scoring.rs::MatchWeights::default()` — `breakdownRows`, `breakdownFlags`, `mergeHref`), reusing the existing `results.*` i18n keys already translated for `MatchResultsList.svelte` rather than duplicating a second set of component labels. Tests: `tests/unit/review.test.ts` (19 — weights sum to 1.00, descending order, the five wire keys, the null/non-object/unknown-key breakdown paths, the boolean-flag mapper including the truthy-string trap, the decidable guard, both merge-link orders), an i18n-parity extension for 44 new keys across all 13 locales (real per-locale translations, not English stubs), and the existing five-test Playwright smoke suite re-verified green (no BFF-proxy-prefix regression found in this crate — `tests/unit/client.test.ts` already pinned the correct `/api/proxy/api/…` expectation). **Follow-up, 2026-08-05:** `tests/e2e/things.spec.ts` gained the two e2e pins that worker/place/organization already carried from their own FE-4 fan-out and this crate was missing — a merge-page `?main=&duplicate=` query-string pre-fill test, and a `/review` end-to-end test (network-stubbed `GET /api/things/review-queue` + two `GET /api/things/{id}`) exercising the queue table, the `Compare` button, the side-by-side panel, and — per the FR-15 gap above — asserting the breakdown section renders its documented `review-no-breakdown` empty state rather than a table, since this service never sends `score_breakdown` on the wire. Suite is now 7 tests, all green; no application code changed.
- [x] T-25 (repo PRO-H5): **CSRF on the BFF mutations.** Closes the gap T-22 flagged and root [`tasks.md`](../../../tasks.md) PRO-H5 tracked (this crate had no prior open task line for it). Ports the synchroniser/double-submit pattern landed as the family reference in `person-front-end-with-svelte` (2026-08-28): a second, non-httpOnly `__Host-mxi_csrf` cookie (`generateCsrfToken()`/`CSRF_COOKIE`/`CSRF_COOKIE_OPTIONS` in `src/lib/server/session.ts`) set alongside the session cookie in `/verify`'s `+page.server.ts`; `src/lib/api/client.ts`'s `ApiClient` reads it from `document.cookie` (a no-op server-side) and echoes it as `X-CSRF-Token` on every non-GET/HEAD request; `src/routes/api/proxy/[...path]/+server.ts` rejects a missing/mismatched token with `403 {"error":"csrf"}` before forwarding upstream, backstopped by an Origin/Referer check (rejects only when a value is present and disagrees); the root `+page.server.ts` `signout` action clears both cookies. `vite.config.ts` gained `environmentOptions.jsdom.url: "https://localhost:5173"` so `document.cookie = "__Host-…"` actually sticks in jsdom (the `__Host-` prefix requires a secure origin). New tests: `tests/unit/session.test.ts` (10 — `verifyCsrf` matrix + cookie config pins), `tests/unit/proxy.test.ts` (7 — the CSRF gate: safe-method bypass, missing/mismatched token, matching token with no/same-origin/cross-origin Origin, cross-site Referer), and a CSRF-header `describe` block added to `tests/unit/client.test.ts` (3 — attaches on POST when cookie present, omits on GET, omits on POST with no cookie).
- [x] T-26 (2026-08-29, PRO-H10): **Page-visit guard.** Redirect an
  unauthenticated visitor away from every page whose sole purpose is
  submitting a mutation — `/things/new`, `/things/[id]/edit`,
  `/things/merge`, `/review` each gained a `+page.server.ts` calling the
  new `requireSignedIn(locals)` (`src/lib/server/session.ts`),
  `redirect(303, "/signin")` on no session. Read/list/search/view pages
  stay public — this mirrors the backend's own default-allow-read /
  mutation-deny ABAC posture rather than a separate front-end policy.
  `locals.sessionId` is presence-only, a UX convenience in front of the
  backend's real enforcement, not a substitute for it. **No
  `/things/bulk` route exists to guard** — thing carries no bulk
  import/export capability at all (`agents/share/overview.md`'s
  capability matrix), unlike person, so this crate guards one fewer
  route than person's reference set. `/review` is guarded in full
  despite listing the stored queue on load, matching person's own
  `/review` guard: the queue exists only to be decided on. Deliberately
  does not thread a `next` param back through `/signin` in v1 (see
  `AGENTS.md`'s "Page-visit guard" section for why — the magic-link
  round trip does not carry one today). Tests: `tests/unit/session.test.ts`
  (+2 — `requireSignedIn` throws a 303-to-`/signin` redirect when signed
  out, passes through silently when signed in).
- [x] T-27: **Wire `mask_sensitive` into the list/search UI.** *(resolved 2026-09-06.)*
  `ThingRepository`'s search options already declare `mask_sensitive`
  (`src/lib/api/things.ts`) and it is forwarded to
  `GET /api/things/search`, but `/things` (the list/search route)
  already exposes a `fuzzy` checkbox and never sets `mask_sensitive` —
  there is no operator-facing masked-search toggle, unlike the detail
  page's masked-view toggle (T-19). *(verified:
  `grep -n mask_sensitive src/routes/things/+page.svelte` returns
  nothing, while `grep -n mask_sensitive src/lib/api/things.ts` shows
  the option declared and threaded through `search()`; the same route
  already wires a `fuzzy` `$state` checkbox at line 29/82.)*
  **Acceptance:** a checkbox on `/things` (alongside the existing
  `fuzzy` toggle) sets `mask_sensitive` on the search call and
  re-fetches; a Playwright test stubs masked vs. unmasked search
  responses and asserts the toggle changes the request/rendered values;
  three-part change (spec §6/§9 + code + test).
  - **Resolved.** A new `maskSensitive` checkbox sits alongside the
    existing `fuzzy`/`phonetic` toggles, wired to `search()`'s
    `mask_sensitive` option. Unlike `fuzzy`/`phonetic` (which only take
    effect on the *next* manual `SearchBox` submit — it fires on
    submit, not on keystroke or checkbox change), this toggle carries
    its own `onchange` that re-runs `runSearch(query)` immediately,
    mirroring the detail page's masked-view toggle (T-19) so switching
    the view doesn't require re-submitting the query too. New
    `things.maskSensitive` i18n key across all 13 locales. New
    Playwright test (`tests/e2e/things.spec.ts`) stubs
    `**/api/things/search**`, branching the returned `name` on the
    request's `mask_sensitive` query param, and asserts checking then
    unchecking the toggle swaps the rendered value both ways. Verified:
    `npm test` (92 passed), `npx playwright test` (14 passed, up from
    13), `npm run check` (0 errors), `npm run lint` clean.
- [x] T-28: **Pagination on the `/things` list/search route.** *(resolved 2026-09-06.)*
  `place-service`'s family-wide pagination convention
  (`agents/share/restful.md`: `?limit=&offset=` plus
  `X-Total-Count`/`X-Limit`/`X-Offset` response headers) is never
  consumed here — `/things` calls `search({ limit: 50 })` with no
  `offset` and no way to page past the first 50 results, and
  `ThingRepository`/`ApiClient` never read the `X-Total-Count` header
  at all, so an operator has no way to know the search matched more
  than 50 things. *(verified: `grep -n "offset\|X-Total-Count"
  src/routes/things/+page.svelte` returns nothing but the hard-coded
  `limit: 50`; `grep -rn "X-Total-Count" src/lib/api/*.ts` returns
  nothing.)* **Acceptance:** `/things` reads `X-Total-Count` from the
  search response and shows a "N of M" indicator plus next/previous
  controls that advance `offset`; a unit test pins
  `ThingRepository.search` surfacing the total-count header; a
  Playwright test stubs a response with `X-Total-Count` greater than
  the page size and asserts the next-page control fetches with the
  advanced `offset`; three-part change (spec §6/§9 + code + test).
  - **Resolved.** `ApiClient` gained `getWithHeaders` (and a private
    `requestWithResponse` core `request` now delegates to), so a
    caller can read response headers without disturbing the envelope
    contract every other verb relies on. `ThingRepository.search` now
    calls it and returns `{ items, total, limit, offset }`, preferring
    `X-Total-Count`/`X-Limit`/`X-Offset` over anything the body itself
    carries — the headers are the authoritative count ignoring
    `limit`/`offset`, and a bare-array body has no room for one at all
    — falling back to the pre-existing behaviour when a service
    predates the headers. `/things` gained a fixed `PAGE_SIZE` (50,
    same default as before), an `offset` `$state` (updated from the
    service's *actually-applied* `X-Offset`, which may clamp), and
    Previous/Next buttons (disabled at each end) plus a "N of M"
    indicator, shown once a search returns any total > 0. A new query
    or toggle submit always starts back at offset 0 (`runSearch`'s
    default parameter); only the pagination buttons and the
    mask-sensitive toggle's `onchange` (which stays on the current
    page — masking is a view change, not a new query) pass an explicit
    offset. New `things.previousPage`/`things.nextPage`/
    `things.pageRange` i18n keys across all 13 locales. Two new unit
    tests pin the header-preference and the header-absent fallback; a
    new Playwright test stubs `X-Total-Count: 100` and asserts clicking
    Next sends `offset=50` and Previous returns to `offset=0`, with the
    indicator and button disabled-states updating each way. Verified:
    `npm test` (94 passed, up from 92), `npx playwright test` (15
    passed, up from 14), `npm run check` (0 errors), `npm run lint`
    clean.
- [x] T-29: **401/403 handling in `ApiClient`.** *(resolved 2026-09-06.)*
  `ApiError` exposes `isNotFound()`/`isConflict()`/`isValidation()`
  helpers but nothing for `401`/`403`, and no route reacts to either
  status — a session that expires mid-visit (or a
  `THING_REQUIRE_AUTH`-gated `403`) surfaces as a raw, untranslated
  error banner rather than a redirect to `/signin` or an access-denied
  message, the same gap `place-front-end-with-svelte` carries under
  its own new T-25 (401/403). *(verified: `grep -n
  "401\|403\|isUnauthorized\|isForbidden" src/lib/api/client.ts` returns
  nothing.)* **Acceptance:** `ApiError` gains `isUnauthorized()` /
  `isForbidden()`; a `401` from the proxy (session expired/absent)
  redirects the browser to `/signin`; a `403` shows a translated
  access-denied message rather than the raw error body; unit tests pin
  both helpers and the redirect behaviour; three-part change (spec
  §8/§9 + code + test).
  - **Resolved.** `ApiError` gained `isUnauthorized` (401) and
    `isForbidden` (403) getters (`src/lib/api/client.ts`, mirroring
    `isConflict`'s existing shape). New `src/lib/api/errorHandling.ts`
    exports `describeApiError(err)`: on `isUnauthorized` it calls
    `goto("/signin")` (from `$app/navigation`) and returns the new
    translated `auth.sessionExpired` string; on `isForbidden` it
    returns the new translated `auth.accessDenied` string; otherwise
    it falls back to `err instanceof Error ? err.message :
    String(err)` — byte-identical to what every route's catch block
    already produced, so wiring it in changes nothing for any other
    status. All 8 routes with a catch block
    (`+page.svelte`, `review/+page.svelte`,
    `things/+page.svelte`, `things/merge/+page.svelte`,
    `things/match/+page.svelte`, `things/[id]/+page.svelte`,
    `things/[id]/edit/+page.svelte`, `things/[id]/audit/+page.svelte`)
    now call `describeApiError` instead of the inline ternary; the
    merge page's second catch block (which prefers the service's
    structured `CODE: message` for other `ApiError` statuses) checks
    `isUnauthorized`/`isForbidden` first and falls through to that
    existing behaviour otherwise, so 409/422/etc. are unaffected.
    `auth.sessionExpired`/`auth.accessDenied` added to all 13 locales.
    New `tests/unit/errorHandling.test.ts` (5 tests, mocking
    `$app/navigation`'s `goto`) pins the redirect-on-401,
    message-on-403 (no redirect), and every fallback case; two new
    tests in `tests/unit/client.test.ts` pin the getters themselves.
    Verified: `npm test` (92 passed, up from 85), `npx playwright
    test` (13 passed, unaffected), `npm run check` (0 errors), `npm
    run lint` clean (prettier auto-fixed two files' formatting).

