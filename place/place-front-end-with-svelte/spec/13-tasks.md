## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `place-service-with-loco/agents/models.md`.
- [x] T-3: `ApiClient` + `PlaceRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `PlaceRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [x] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Contradicts a decision this project already made and states in `AGENTS.md`: `src/routes/+layout.ts` sets `ssr = false` and `prerender = false`, every page is client-rendered, and every entity-API call goes through the same-origin BFF proxy with the browser's session cookie — there is no server-side render in which an `event.fetch` load could run, and no `+page.ts` load exists anywhere in `src/routes/`. The "warm-cache win" would mean re-introducing SSR against the CSR-only + BFF design; if that is ever wanted it is a new decision, not this task.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Identifier / opening-hours / amenity edit (these sub-record lists are read-only on detail; the edit form re-PUTs the whole record but has no UI to add/remove them. Address and geo are already editable via `PlaceForm`. Rewritten 2026-06-13: the original wording said "emergency-contact edit" — a person-entity copy artifact; places have no emergency contacts).
- [x] T-16: Theming tokens in `app.css` extracted to a small theme module. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Superseded by the Lily `ThemePicker` adopted 2026-07-31 (`src/routes/+layout.svelte` imports it from `lily-design-system-svelte-theme-picker` and offers the DaisyUI theme ids), which is the theming mechanism now. The 42 `--mxi-*` custom properties in `src/app.css` `:root` are the contract components consume, and CSS is where custom properties belong; a JS "theme module" would re-export what the stylesheet already provides and add a second place for the same values to drift.
- [x] T-17: `check-duplicates` endpoint wired into create form (preview before commit). *(closed as superseded by T-6, 2026-09-03 — repo `tasks.md` WEB-6)* `POST /api/places` already runs the identical duplicate check (both handlers score the same candidates against the same `state.matcher.threshold()`) and answers `409` **with the candidate matches, creating nothing** — and `/places/new` renders them inline (T-6). The create is therefore already the preview: a pre-submit `checkDuplicates()` call would run the matcher a second time to show the same list. What the investigation did surface is different and belongs to the **service**, not this front-end: neither the API nor the form offers any way to create past a `409` — a legitimate near-duplicate can never be created through the UI, because no override exists to wire a button to. That is an authorisation question (an override is `destructive`-class work), recorded under WEB-6 for the service specs to take up if a deployment needs it.
- [x] T-18: Batch deduplicate-scan results UI. Landed 2026-07-19 as `/review` — SVAR Kanban board (Pending / Confirmed / Rejected / AutoMerged) that loads the stored `GET /api/places/review-queue` on mount and drives decisions through `POST /api/places/review-queue/{id}/decision`; the scan button (`POST /api/places/deduplicate`, destructive-classed) is explicit, never a page-load side effect.
- [x] T-19: Masked-view toggle on detail page. *(2026-09-03)* A toggle
  button in the `/places/[id]` header re-fetches through the existing
  `PlaceRepository.masked(id)` (`GET /api/places/{id}/masked`) rather
  than redacting fields client-side — the server decides what counts as
  sensitive, mirroring person's and worker's identical T-19 delivery.
  Shows a `role="status"` banner while the masked view is active, so a
  screenshot or a glance at the page makes the mode unambiguous.
  Toggling back re-fetches the plain record rather than caching the
  pre-toggle state, so a concurrent edit is never shown stale. New keys
  `detail.showMasked` / `detail.showFull` / `detail.maskedNotice`,
  translated across all 13 locales. The repository method and its unit
  test (`masked() GETs /api/places/:id/masked`) already existed on
  `main` — this task closes the gap the repo memory identified: the
  method was wired but never surfaced in any route's UI.
  - **Acceptance:** the pre-existing `tests/unit/places.test.ts`
    `masked()` test already pinned the endpoint; a new Playwright smoke
    test (`tests/e2e/places.spec.ts`) stubs the plain and masked
    endpoints with visibly different `telephone` values and asserts the
    toggle switches between them and shows/hides the masked-view banner.
- [x] T-20: GDPR-export download button. *(2026-09-03, repo `tasks.md` WEB-5 — copy-adapted from person's reference)* A button on `/places/[id]` calls the existing `PlaceRepository.exportGdpr(id)` (`GET /api/places/{id}/export`) and hands the service-defined payload to the browser as a downloaded `place-<id>-export.json` — serialised verbatim, never interpreted, through a Blob object URL and a synthetic anchor (revoked once the click has fired). An `exporting` state disables the button while the request is in flight; errors go to the existing banner. New keys `detail.exportGdpr` / `detail.exportingGdpr` ×13 locales.
  - **Acceptance:** `tests/unit/places.test.ts` pins `exportGdpr()` GETs `/api/places/{id}/export` and returns the payload unchanged; a Playwright smoke test stubs the endpoint, clicks the button, awaits the browser `download` event, and asserts both the suggested filename and that the saved bytes parse back to the stubbed payload.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [x] T-22: Auth — adopt BFF + httpOnly cookie + CSRF; the browser holds only `__Host-mxi_session`, the SvelteKit server attaches a short-lived PASETO server-side; no `mxi_access_token`/`localStorage` bearer, no fragment handoff (per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)). Landed: `/signin` + `/verify` routes, `src/lib/server/{session,auth,config}.ts`, and the `/api/proxy/[...path]` reverse proxy that injects the server-exchanged PASETO. CSRF on mutating browser→BFF calls is not yet separately verified — worth a follow-up task if it isn't covered elsewhere.
- [x] T-23 (repo FE-4): **Duplicate review-queue screen upgrade** at `/review` — 2026-08-04, following the same pattern person-front-end-with-svelte's own T-25 established for its sibling screen. T-18 landed the board itself (2026-07-19); this task closes the gaps it left: a **status + page-size filter** wired to `?status=`/`?limit=` (`PlaceRepository.listReviewQueue` gained a `ReviewQueueOptions` argument; "all" is the *absence* of `status`, confirmed against `place-service-with-loco`'s actual `get_review_queue` handler, which answers `422 INVALID_STATUS` for a token it does not recognise; there is no `offset`, so page size is the whole pagination story); a **keyboard-reachable path** — the SVAR Kanban's drag-to-decide is mouse-only, so a native queue table with a per-row `Compare` button was added alongside it, and the panel carries real `Confirm`/`Reject` buttons (drag still works; it is simply no longer the only way); and an **inline side-by-side comparison** loading both places with two parallel `GET /api/places/{id}` calls (`Promise.allSettled`, so a soft-deleted side still renders the other), showing name / type / address / geo / telephone / GLN.
  - **Known, documented gap vs. person's screen** (verified against the Rust source, not assumed): place-service's `ReviewQueueItem` (`src/api/rest/handlers.rs`) carries **neither** `provenance` (the `review_queue` table has no such column — confirmed against `migration/src/m20260719_000001_create_review_queue.rs`) **nor** a wire-serialized `score_breakdown` (the column exists on `db::review_queue::ReviewQueueRow` but the batch-scan handler always inserts `NULL` and the wire struct never exposes it at all). Rather than fake either, the queue surfaces `detection_method` (which *is* on the wire) in place of a provenance badge, and the breakdown section always renders its "not recorded" note today. `src/lib/review.ts`'s `breakdownRows` is still fully generic/tested (`MATCH_COMPONENTS` uses place-matcher's real default weights — name 0.35, geo 0.25, address 0.20, place_type 0.10, identifier 0.10, per `place-service-with-loco/agents/matching.md`) so the panel activates automatically the day the service starts sending the field — no front-end change required. Fixing the service side (adding `provenance` + serializing `score_breakdown`) is out of this task's scope (front-end only) and is a candidate follow-up for `place-service-with-loco`'s own `spec/13-tasks.md`.
  - Scope decisions matching the family pattern: **confirming does not merge** (the decision endpoint is a pure status change and the service records no link to a merge), so the panel deep-links `/places/merge?main=…&duplicate=…` — in **either** survivor order, since a review item names an unordered pair — and `/places/merge` gained the same one-line `?main=`/`?duplicate=` seed person's merge page has; decision buttons are disabled for a non-`pending` item rather than offering a request guaranteed to answer `422 INVALID_REVIEW_TRANSITION`.
  - New pure module `src/lib/review.ts` (status vocabulary, `canDecide`, the five weighted `MATCH_COMPONENTS`, `breakdownRows`, `mergeHref`) — mirrors person's `$lib/review` shape exactly, adapted to place's own matcher weights and entity fields.
  - Tests: `tests/unit/review.test.ts` (15 — weights sum to 1.00, descending order, the null/non-object/unknown-key breakdown paths, the decidable guard, both merge-link orders), `PlaceRepository.listReviewQueue`'s new options plumbing exercised indirectly via `pnpm check`'s type coverage, an i18n-parity extension for 50 new keys across all 13 locales (`review.intro`/`loading`/`empty`/`filter.*`/`status.*`/`board.title`/`list.title`/`col.*`/`compare.*`/`field.*`/`breakdown.*`/`component.*`/`decide.*`/`merge.*`), and two new route-stubbed Playwright smoke assertions (the board + queue + comparison panel showing the "not recorded" breakdown note, and the merge page's query-param seed).
- [ ] T-24: **CSRF protection on the BFF mutating proxy calls.** T-22
  flagged this and left it open ("CSRF on mutating browser→BFF calls is
  not yet separately verified"); it is not merely unverified, it does
  not exist. `src/routes/api/proxy/[...path]/+server.ts` forwards
  `GET`/`POST`/`PUT`/`PATCH`/`DELETE` alike with no synchroniser-token or
  `Origin`/`Referer` check, and neither `src/` nor `tests/` contains the
  string `csrf` anywhere. Per
  [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
  §4, CSRF protection is "mandatory" for cookie-authenticated mutating
  requests — this is the same gap `thing-front-end-with-svelte` closed
  as its own T-25 (repo PRO-H5), copy-adaptable directly: a second,
  non-httpOnly `__Host-mxi_csrf` cookie set alongside the session cookie
  in `/verify`, echoed as `X-CSRF-Token` on non-GET/HEAD requests by
  `ApiClient`, and checked by the proxy before forwarding upstream,
  backstopped by an `Origin`/`Referer` check. *(verified:
  `grep -rln "csrf" src/ tests/` returns nothing in this project;
  `sed -n '1,58p' src/routes/api/proxy/\[...path\]/+server.ts` shows no
  such check; `thing/thing-front-end-with-svelte/src/lib/server/session.ts`
  carries the working reference implementation.)* **Acceptance:** a
  mutating proxy request with a missing/mismatched CSRF token is
  rejected `403` before it reaches `PLACE_API_URL`; a matching token
  passes; `tests/unit/` pins the token-generation, header-attachment,
  and proxy-rejection matrix (mirroring `thing-front-end-with-svelte`'s
  `tests/unit/session.test.ts` + `tests/unit/proxy.test.ts`); three-part
  change (spec §8/§9 + code + test).
- [x] T-25: **Page-visit guard on mutation-only pages.** *(resolved
  2026-09-06.)* `/places/new`, `/places/[id]/edit`, `/places/merge`,
  and `/review` rendered their forms to an unauthenticated visitor and
  let the submit fail server-side, rather than redirecting to `/signin`
  first.
  - **Resolved.** Ported `thing-front-end-with-svelte`'s T-26 pattern
    verbatim: `requireSignedIn(locals)` added to
    `src/lib/server/session.ts` (redirects `303` to `/signin` when
    `locals.sessionId` is absent — `App.Locals`/`hooks.server.ts`
    already set it here, no change needed there), called from a new
    `+page.server.ts` on each of the four mutation-only routes.
    `/places`, `/places/[id]`, and `/places/[id]/audit` stay public.
  - **A pre-existing gap this surfaced**: this project's
    `playwright.config.ts` carried no stub session cookie at all
    (unlike `thing`/`worker`'s `SMOKE_STORAGE_STATE`), so three
    existing smoke tests (`/places/new`, `/places/merge`, `/review`)
    would have started 303ing to `/signin` the moment the guard
    landed. Added the identical `SMOKE_STORAGE_STATE` stub cookie
    (presence-only, never validated, never sent to a real service) as
    the chromium project's default `storageState`, and a new "page-visit
    guard" describe block in `tests/e2e/places.spec.ts` that drops the
    cookie (`test.use({ storageState: { cookies: [], origins: [] } })`)
    to pin the anonymous-redirect path itself.
  - **Acceptance:** a unit test (`tests/unit/session.test.ts`, new)
    pins `requireSignedIn`'s pass/redirect behaviour; four new
    Playwright tests assert an anonymous visit to each of the four
    guarded routes redirects `303` to `/signin` — verified to **fail**
    with the four `+page.server.ts` files removed and **pass** with
    them restored. Three-part change: spec (here) + code + test.
- [x] T-26: **E2E coverage for `/signin` and `/verify`.** *(resolved
  2026-09-05.)* `tests/e2e/` contained exactly one spec file,
  `places.spec.ts`; there was no Playwright smoke test for the
  magic-link sign-in or verify pages at all — the same gap
  `thing-front-end-with-svelte` tracks as its still-open T-23.
  *(verified: `ls tests/e2e/` listed only `places.spec.ts`;
  `grep -rln "signin\|verify" tests/e2e/` returned nothing.)*
  **Resolution:** the outbound calls `/signin` and `/verify` make
  happen server-side (`src/lib/server/auth.ts`), not in the browser, so
  `page.route` cannot stub them. New `tests/e2e/auth-stub-server.ts` is
  a real, minimal HTTP server standing in for the authentication
  service's magic-link endpoints, started in a new
  `tests/e2e/global-setup.ts` (Playwright `globalSetup`) **before** the
  preview server boots — `AUTH_API_URL` is read once at that server's
  startup (`$env/dynamic/private`), so the stub must already be
  listening on the fixed port `playwright.config.ts`'s new
  `webServer.env` points it at (`tests/e2e/auth-stub-port.ts` shares
  that port between the two files). New `tests/e2e/auth.spec.ts` (5
  tests): `/signin` renders the form and shows the confirmation banner
  after submitting; `/verify` with no token shows the missing-token
  message (needs no stub); with an expired/unknown token (a real `401`
  from the stub) shows the invalid-token message; with a valid token
  the stub returns a `Set-Cookie` and the page redirects home with the
  session cookie set; and — the scenario that actually surfaced a real
  defect — with the stub simulating the service being **unreachable**
  (`req.socket.destroy()`, no response at all).
  **Real defect found and fixed, as anticipated by this task's own
  acceptance note:** `verifyMagicLink`'s `fetch` call in
  `src/routes/verify/+page.server.ts` had no error handling, so a
  network-level failure (unlike a reachable service answering `401`)
  threw uncaught out of `load` and SvelteKit rendered its generic `500
  Internal Error` page — confirmed directly by reproducing the
  scenario (`curl` against the built preview server's `__data.json`
  endpoint) before writing any fix. Now wrapped in a `try`/`catch`
  yielding a new, honest `error: "serviceUnavailable"` state with its
  own message ("We could not reach the sign-in service…") — deliberately
  distinct from "this link is invalid or has expired", which would
  misattribute an unreachable service to the token being bad.
  Verified: `npm run check` (svelte-check) clean; `npm test` (vitest,
  55/55) clean; `npx playwright test` 14/14 passing (5 new + 9
  pre-existing), run three times to rule out flake.
- [x] T-27: **Wire `mask_sensitive` into the list/search UI.**
  *(resolved 2026-09-06.)* `GET /api/places/search` accepts
  `mask_sensitive` and `PlaceRepository`'s search method already
  threaded it through, but `/places` (the list/search route) never set
  it — there was no operator-facing toggle, unlike the detail page's
  masked-view toggle (T-19). An operator who wants a masked search
  result set (e.g. to demo or screenshot without exposing
  telephone/coordinates) had no way to ask for one. *(verified:
  `grep -rn "mask_sensitive" src/routes/` in this project returned
  nothing, while `src/lib/api/places.ts` declared the parameter on the
  search options type.)*
  - **Resolved.** A "Mask sensitive fields" checkbox on `/places`
    (alongside fuzzy/phonetic) sets `mask_sensitive` on the search
    call; unlike fuzzy/phonetic (which wait for the next explicit
    search submit), it re-fetches immediately on toggle — the same
    "flip and re-fetch now" behaviour T-19's detail-page masked-view
    toggle already uses, since it's a view choice rather than a search
    strategy the operator is still composing. New i18n key
    `places.maskSensitive` across all 13 locales. Two new
    `tests/unit/places.test.ts` cases pin `mask_sensitive` is forwarded
    when set and omitted when unset; a new
    `tests/e2e/places.spec.ts` case stubs masked vs. unmasked search
    responses (different `total`s) and asserts both the outgoing
    request (`mask_sensitive=true`/`=false`) and the rendered count
    change with the toggle — verified to fail (timeout waiting for the
    checkbox) with the UI change reverted and pass with it restored.
  - **Acceptance met:** `npm test` 59/59 (was 57); `npm run check`
    clean; `npx playwright test tests/e2e/places.spec.ts` 14/14 (was
    13); `npm run lint` clean.

