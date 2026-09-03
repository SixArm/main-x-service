## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `person-service-with-loco/agents/models.md`.
- [x] T-3: `ApiClient` + `PersonRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `PersonRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [x] T-12a: Playwright **integration** suite (`tests/integration/golden-paths.spec.ts`) driving the live preview against a running `person-service-with-loco`. 9 tests covering FR-1, FR-3 (×2 — happy path + 409 duplicate), FR-5, FR-6, FR-7, FR-8, FR-9, and per-record audit. Idempotent (timestamped family names + REST `DELETE` cleanup, plus a bounded 409-retry in `apiCreatePerson` — PRO-P4, see OQ-5). Run with `bin/e2e` or `pnpm test:integration`. Harness is validated (svelte-check clean, playwright `--list` discovers all 9 tests, smoke project still 6/6, bin/e2e exits 1 with a clear message when the service is down). **The duplicate-detector test-data interaction that used to fail 3 of 9 is fixed (PRO-P4, 2026-08-29 — see OQ-5). The page-visit auth guard (PRO-H10) + CSRF check (PRO-H5) blocker is also closed (T-27, PRO-P32, 2026-08-29): the suite now signs in for real via a dedicated `setup` project — see OQ-5's final update.** **Fully green 10/10, run twice (2026-09-02, PRO-P4 resumed):** PRO-P32 proved the auth mechanism but never re-ran this suite; doing so live surfaced and closed two real front-end bugs (`createForm` couldn't clone a `$state`-wrapped `initial`, breaking `FR-6`; `match()`/`checkDuplicates()` didn't unwrap the service's real `{matches,…}`/`{potential_matches,…}` envelopes, breaking `FR-8`) plus one operational gap (the container's database was never migrated). See OQ-5's closing update for the full record.
- [x] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Contradicts a decision this project already made and states in `AGENTS.md`: `src/routes/+layout.ts` sets `ssr = false` and `prerender = false`, every page is client-rendered, and every entity-API call goes through the same-origin BFF proxy with the browser's session cookie — there is no server-side render in which an `event.fetch` load could run, and no `+page.ts` load exists anywhere in `src/routes/`. The "warm-cache win" would mean re-introducing SSR against the CSR-only + BFF design; if that is ever wanted it is a new decision, not this task.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Identifier / address / emergency-contact edit (currently read-only on detail; edit form re-PUTs whole record but no UI to add/remove sub-records).
- [x] T-16: Theming tokens in `app.css` extracted to a small theme module. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Superseded by the Lily `ThemePicker` adopted 2026-07-31 (`src/routes/+layout.svelte` imports it from `lily-design-system-svelte-theme-picker` and offers the DaisyUI theme ids), which is the theming mechanism now. The 42 `--mxi-*` custom properties in `src/app.css` `:root` are the contract components consume, and CSS is where custom properties belong; a JS "theme module" would re-export what the stylesheet already provides and add a second place for the same values to drift.
- [x] T-17: `check-duplicates` endpoint wired into create form (preview before commit). *(closed as superseded by T-6, 2026-09-03 — repo `tasks.md` WEB-6)* `POST /api/persons` already runs the identical duplicate check (`check_duplicates_internal` is the same function behind both routes) and answers `409` **with the candidate matches, creating nothing** — and `/persons/new` renders them inline (T-6). The create is therefore already the preview: a pre-submit `checkDuplicates()` call would run the matcher a second time to show the same list. What the investigation did surface is different and belongs to the **service**, not this front-end: neither the API nor the form offers any way to create past a `409` — a legitimate near-duplicate can never be created through the UI, because no override exists to wire a button to. That is an authorisation question (an override is `destructive`-class work), recorded under WEB-6 for the service specs to take up if a deployment needs it.
- [x] T-18: Batch deduplicate-scan results UI. *(closed as duplicate,
  2026-09-03 — verified directly rather than assumed)* T-25 below
  shipped this exact capability at `/review` (`src/routes/review/`
  exists and is live) under a different task number rather than under
  this one; T-18 was never itself ticked when the board landed. See
  T-25 for the full delivery record — the status/page-size filter, the
  keyboard-reachable table path, the side-by-side comparison panel,
  `provenance` on cards, and the test suite.
- [x] T-19: Masked-view toggle on detail page. *(2026-09-03)* A toggle
  button in the `/persons/[id]` header re-fetches through the existing
  `PersonRepository.masked(id)` (`GET /api/persons/{id}/masked`) rather
  than redacting fields client-side — the server decides what counts as
  sensitive, matching this crate's server-source-of-truth posture
  elsewhere (bulk export's `masking_profile`, review's server-computed
  breakdown). Shows a `role="status"` banner while the masked view is
  active, so a screenshot or a glance at the page makes the mode
  unambiguous. Toggling back re-fetches the plain record rather than
  caching the pre-toggle state, so a concurrent edit is never shown
  stale. New keys `detail.showMasked` / `detail.showFull` /
  `detail.maskedNotice`, translated across all 13 locales and added to
  the i18n-parity `CANONICAL_KEYS` list.
  - **Acceptance:** `tests/unit/persons.test.ts` pins `masked()` GETs
    `/api/persons/{id}/masked`; a new Playwright smoke test
    (`tests/e2e/persons.spec.ts`) stubs the plain and masked endpoints
    with visibly different `tax_id` values and asserts the toggle
    switches between them and shows/hides the masked-view banner.
- [x] T-20: GDPR-export download button. *(2026-09-03, repo `tasks.md` WEB-5 — person is the reference; the other five copy-adapt)* A button on `/persons/[id]` calls the existing `PersonRepository.exportGdpr(id)` (`GET /api/persons/{id}/export`) and hands the service-defined payload to the browser as a downloaded `person-<id>-export.json` — serialised verbatim, never interpreted, through a Blob object URL and a synthetic anchor (revoked once the click has fired). An `exporting` state disables the button while the request is in flight; errors go to the existing banner. New keys `detail.exportGdpr` / `detail.exportingGdpr` ×13 locales, added to the i18n-parity `CANONICAL_KEYS`. The repository method existed since T-3 with no test and no caller.
  - **Acceptance:** `tests/unit/persons.test.ts` pins `exportGdpr()` GETs `/api/persons/{id}/export` and returns the payload unchanged; a Playwright smoke test stubs the endpoint, clicks the button, awaits the browser `download` event, and asserts both the suggested filename and that the saved bytes parse back to the stubbed payload — so an empty or wrongly-named file cannot pass.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [x] T-23 (repo FE-2): Cross-service **links panel** on `/persons/[id]` — list this person's active outbound edges, assert a new one (`same_identity` → worker, `works_at` / `member_of` → organization), and withdraw one behind a confirm. `LinksPanel.svelte` + `EntityLink` / `CreateLinkRequest` types + `listLinks` / `createLink` / `deleteLink` on the repository + the pure kind↔target-type rules in `src/lib/links.ts` (mirroring the service's `validate_edge`, so a wrong target type is caught before the request). Server `422` reasons are surfaced inline. Deliberately distinct from the `Person.links` merge relationship (§9). Tests: `tests/unit/links-validation.test.ts` (12), three repository tests + a 422-surfacing test, an i18n-parity extension for 26 new keys across all 13 locales, and a route-stubbed Playwright smoke assertion.
- [x] T-24 (repo FE-3): **Bulk import/export screen** at `/persons/bulk` — upload a JSONL/CSV file with a dry-run toggle, submit a filtered export with a masking profile, poll each `202`-accepted job to a terminal state, and list recent jobs with client-side kind/status filters (the `bulk-jobs` endpoint takes only `limit` — no server-side filtering). `ApiClient` gained `FormData` pass-through (no JSON serialization, and the forced `content-type` is stripped so `fetch` sets the multipart boundary); repository gained `importPersons` / `exportPersons` / `getImportJob` / `getExportJob` / `listBulkJobs`; `src/lib/bulk.ts` holds the pure rules (terminal-state set, the dry-run token encoding matching the service's `1|true|yes|on`, progress clamping, and the import-format set excluding export-only Parquet). Each submit sends a fresh `Idempotency-Key` (SEC-B9). Scope decisions, all forced by the service rather than chosen here: `download_url` / `errors_url` are **rendered as plain text, not links** — they are opaque artifact-store references (`file://…` / `s3://…`) and the service exposes no endpoint serving their bytes (see §16 OQ-7); `include_soft_deleted` is **not offered** because the endpoint accepts it but the worker rejects it; a `404` on a status poll stops the loop and reports expired-or-gone, since the service returns `404` both for a job past its retention TTL and for another actor's job. Tests: `tests/unit/bulk.test.ts` (14 — pure rules, the `FormData`-not-JSON body with its header contract, the JSON-body regression pin, and the 404 path), an i18n-parity extension for 69 new keys across all 13 locales, and a route-stubbed Playwright smoke assertion.
- [x] T-25 (repo FE-4): **Duplicate review-queue screen** at `/review` — 2026-08-04. The board itself predates this task and was never specified (FR-14…FR-19 and this entry are the backfill, and the i18n parity assertion had never listed `nav.review` / `review.run` either — a real lapse of the three-part-change rule). What landed now closes the board's four gaps: a **status + page-size filter** wired to `?status=` / `?limit=` (`listReviewQueue` gained a `ReviewQueueOptions` argument; "all" is the *absence* of `status`, since the endpoint answers `422 INVALID_STATUS` for a token it does not know, and there is no `offset` so page size is the whole pagination story); a **keyboard-reachable path** — the SVAR Kanban's drag-to-decide is mouse-only, so a native queue table with a per-row `Compare` button was added alongside it, and the panel carries real `Confirm` / `Reject` buttons (drag still works, it is simply no longer the only way); an **inline side-by-side comparison** loading both persons with two parallel `GET /api/persons/{id}` calls (`Promise.allSettled`, so a soft-deleted side still renders the other) and rendering the matcher's `score_breakdown` as a component / weight / score table; and **`provenance`** on the cards and in the table, not only in the panel. Chosen as an inline expandable section rather than a modal: it needs no focus-trap dependency, keeps the board visible as context, and cannot strand a keyboard user behind a trap. Scope decisions forced by the service: **confirming does not merge** (the decision endpoint is a pure status change and the service records no link to a merge), so the panel deep-links `/persons/merge?main=…&duplicate=…` — in **either** survivor order, because a review item names an unordered pair — and `/persons/merge` gained a one-line `?main=`/`?duplicate=` seed (FR-20); decision buttons are disabled for a non-`pending` item rather than offering a request guaranteed to answer `422 INVALID_REVIEW_TRANSITION`; and `ReviewQueueItem` gained the `provenance` field the type had been missing since the service added the column. New pure module `src/lib/review.ts` (status vocabulary, `canDecide`, the seven weighted `MATCH_COMPONENTS`, `breakdownRows`, `mergeHref`). Tests: `tests/unit/review.test.ts` (15 — weights sum to 1.00, descending order, the null/non-object/unknown-key breakdown paths, the decidable guard, both merge-link orders), four repository tests (bare call sends no query string, both filters reach the wire, an absent status is omitted rather than sent as `"undefined"`, and the decision body's field is `status`), an i18n-parity extension for 57 new keys across all 13 locales plus the three pre-existing review keys that had never been listed, and two route-stubbed Playwright smoke assertions (the board + queue + comparison panel, and the merge page's query-param seed).
- [x] T-26 (2026-08-29, PRO-H10): **Page-visit guard.** Redirect an
  unauthenticated visitor away from every page whose sole purpose is
  submitting a mutation — `/persons/new`, `/persons/[id]/edit`,
  `/persons/merge`, `/persons/bulk`, `/review` each gained a
  `+page.server.ts` calling the new `requireSignedIn(locals)`
  (`src/lib/server/session.ts`), `redirect(303, "/signin")` on no
  session. Read/list/search/view pages stay public — this mirrors the
  backend's own default-allow-read / mutation-deny ABAC posture
  rather than a separate front-end policy. `locals.sessionId` is
  presence-only, a UX convenience in front of the backend's real
  enforcement, not a substitute for it. Deliberately does not thread a
  `next` param back through `/signin` in v1 (see `AGENTS.md`'s "Page-
  visit guard" section for why — the magic-link round trip does not
  carry one today). Tests: `tests/unit/session.test.ts` (+2 —
  `requireSignedIn` throws a 303-to-`/signin` redirect when signed
  out, passes through silently when signed in).
- [x] T-27 (2026-08-29, PRO-P32): **Real magic-link sign-in for the
  integration suite.** T-26/T-22's guard + CSRF check left
  `tests/integration/golden-paths.spec.ts` unable to complete any
  mutating flow (see §16 OQ-5's 2026-08-29 update). Decision (asked,
  not assumed): a real sign-in against a live authentication-service,
  not a bypass of the guard/CSRF check. Added a new Playwright `setup`
  project (`tests/integration/auth.setup.ts`, a project `dependencies`
  on rather than a top-level `globalSetup` — the latter would also
  gate the deliberately service-free `smoke` project): `POST
  /api/auth/signup`, poll the live authentication-service container's
  log for the issued token (its tracing output is ANSI-coloured
  regardless of pipe TTY-ness — stripped before matching), then
  `page.goto` the real `/verify?token=…` URL and save the resulting
  `__Host-mxi_session` / `__Host-mxi_csrf` cookies as a `storageState`
  the `integration` project depends on and reuses. This only works
  against a live authentication-service running in `LOCO_ENV=development`
  (SEC-A3 — a production-mode container never logs the magic link),
  which had no compose file anywhere in the repo — added
  `examples/compose/authentication-dev.yml` (family-reusable, not
  person-specific), whose one non-obvious fix is a container-start
  `sed` patch of `server.binding: localhost → 0.0.0.0` (a dev-mode
  container listening on its own loopback is unreachable through the
  compose port-forward regardless of `ports:` — found live: the
  container's own internal healthcheck passed while a host curl got a
  connected-then-empty reply). `bin/e2e` now health-checks
  authentication-service too. Also fixed a previously-latent, adjacent
  bug found in the same investigation: `playwright.config.ts`'s
  `webServer` command baked `PUBLIC_API_BASE_URL` into the client
  build but never set the BFF's own runtime `PERSON_API_URL`
  (`src/lib/server/config.ts`, read at request time, not build time),
  so a UI-submitted mutation went through the server-side proxy to
  whatever this crate's own `.env` said (`:5150`, the native `cargo
  run` port) rather than the live instance `PUBLIC_API_BASE_URL`
  actually started (`:8080`, the podman-compose container) — the two
  are now set to the same base. Verified live end-to-end (not
  inferred): signup → log line → `/verify` → both real cookies present
  in the saved `storageState`; then, reusing it, a guarded page
  (`/persons/new`) rendered its create form instead of redirecting to
  `/signin`. `svelte-check` 0 errors, `prettier --check` clean on the
  new/changed files.
- [x] T-22 (2026-08-28, PRO-H5): Auth — adopt BFF + httpOnly cookie + CSRF (per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)). **BFF + httpOnly cookie + PASETO exchange are implemented**: `/signin` (magic-link request) and `/verify` (consumes the link, sets `__Host-mxi_session`, httpOnly/Secure/`SameSite=Lax`) at `src/routes/{signin,verify}/`; `src/hooks.server.ts` reads the cookie into `locals.sessionId`; `src/routes/api/proxy/[...path]/+server.ts` is the reverse proxy that drops the browser's cookie, exchanges the session for a short-lived PASETO (`src/lib/server/auth.ts`), and forwards with `Authorization: Bearer …`. No `mxi_access_token`/`localStorage` bearer, no fragment handoff — the browser never holds a token. **CSRF closed**: `/verify` additionally sets a second, **non-httpOnly**, Secure, `SameSite=Lax` cookie `__Host-mxi_csrf` (`generateCsrfToken()`/`CSRF_COOKIE`/`CSRF_COOKIE_OPTIONS`, `src/lib/server/session.ts`); `ApiClient` (`src/lib/api/client.ts`) reads it from `document.cookie` when running in the browser and echoes it as `X-CSRF-Token` on every non-GET/HEAD request; the proxy verifies the header matches the cookie (`verifyCsrf`, constant-value equality — both sides are BFF-issued, so no timing-safe compare is needed) and additionally rejects a present-but-mismatched `Origin`/`Referer`, returning `403 {"error":"csrf"}` **without forwarding upstream** on either failure. Sign-out (root `+page.server.ts`'s `signout` action) clears both cookies. Tests: `tests/unit/session.test.ts` (10, `verifyCsrf`/`generateCsrfToken`/cookie-option pins), `tests/unit/proxy.test.ts` (7, the route handler exercised directly — GET always passes, missing/mismatched token 403s, Origin/Referer backstop), `tests/unit/client.test.ts` (+3, the browser header-attach path via jsdom's real `document.cookie`, which required pointing `vite.config.ts`'s jsdom `testURL` at `https://` since a `__Host-`-prefixed cookie only sets over a secure origin).

