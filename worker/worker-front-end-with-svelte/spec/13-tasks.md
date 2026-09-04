## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `worker-service-with-loco/agents/models.md`.
- [x] T-3: `ApiClient` + `WorkerRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `WorkerRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [x] T-23 (FE-2): Cross-service links panel on the worker detail route —
  list / assert / withdraw the worker's outbound `entity_links` edges
  (`GET`/`POST`/`DELETE /api/workers/{id}/links`). Only the two kinds
  worker may originate are offered (`same_identity` → `person`,
  `employed_by` → `organization`, where `role` is the job title), with a
  client-side mirror of the service's `validate_edge` so a wrong target
  type is caught before the `422`; the server's reason is still shown
  inline when it is the one to refuse. Deliberately distinct from the
  within-service `Worker.links`, which stays untouched (the partition
  rule, [`cross-service-linking.md`](../../../agents/share/cross-service-linking.md) §7).
- [x] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Contradicts a decision this project already made and states in `AGENTS.md`: `src/routes/+layout.ts` sets `ssr = false` and `prerender = false`, every page is client-rendered, and every entity-API call goes through the same-origin BFF proxy with the browser's session cookie — there is no server-side render in which an `event.fetch` load could run, and no `+page.ts` load exists anywhere in `src/routes/`. The "warm-cache win" would mean re-introducing SSR against the CSR-only + BFF design; if that is ever wanted it is a new decision, not this task.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Identifier / address / emergency-contact edit (currently read-only on detail; edit form re-PUTs whole record but no UI to add/remove sub-records).
- [x] T-16: Theming tokens in `app.css` extracted to a small theme module. *(closed as won't-do, 2026-09-03 — repo `tasks.md` WEB-6)* Superseded by the Lily `ThemePicker` adopted 2026-07-31 (`src/routes/+layout.svelte` imports it from `lily-design-system-svelte-theme-picker` and offers the DaisyUI theme ids), which is the theming mechanism now. The 42 `--mxi-*` custom properties in `src/app.css` `:root` are the contract components consume, and CSS is where custom properties belong; a JS "theme module" would re-export what the stylesheet already provides and add a second place for the same values to drift.
- [x] T-17: `check-duplicates` endpoint wired into create form (preview before commit). *(closed as superseded by T-6, 2026-09-03 — repo `tasks.md` WEB-6)* `POST /api/workers` already runs the identical duplicate check (`check_duplicates_internal` is the same function behind both routes) and answers `409` **with the candidate matches, creating nothing** — and `/workers/new` renders them inline (T-6). The create is therefore already the preview: a pre-submit `checkDuplicates()` call would run the matcher a second time to show the same list. What the investigation did surface is different and belongs to the **service**, not this front-end: neither the API nor the form offers any way to create past a `409` — a legitimate near-duplicate can never be created through the UI, because no override exists to wire a button to. That is an authorisation question (an override is `destructive`-class work), recorded under WEB-6 for the service specs to take up if a deployment needs it.
- [x] T-18: Batch deduplicate-scan results UI. *(closed as duplicate,
  2026-09-03 — verified directly rather than assumed)* T-25 below
  shipped this exact capability at `/review` (`src/routes/review/`
  exists and is live) under a different task number rather than under
  this one; T-18 was never itself ticked when the board landed. See
  T-25 for the full delivery record — the status/page-size filter, the
  keyboard-reachable table path, the side-by-side comparison panel,
  and the test suite.
- [x] T-19: Masked-view toggle on detail page. *(2026-09-03)* A toggle
  button in the `/workers/[id]` header re-fetches through the existing
  `WorkerRepository.masked(id)` (`GET /api/workers/{id}/masked`) rather
  than redacting fields client-side — the server decides what counts as
  sensitive, mirroring person's identical T-19 delivery. Shows a
  `role="status"` banner while the masked view is active, so a
  screenshot or a glance at the page makes the mode unambiguous.
  Toggling back re-fetches the plain record rather than caching the
  pre-toggle state, so a concurrent edit is never shown stale. New keys
  `detail.showMasked` / `detail.showFull` / `detail.maskedNotice`,
  translated across all 13 locales.
  - **Acceptance:** `tests/unit/workers.test.ts` pins `masked()` GETs
    `/api/workers/{id}/masked`; a new Playwright smoke test
    (`tests/e2e/workers.spec.ts`) stubs the plain and masked endpoints
    with visibly different `tax_id` values and asserts the toggle
    switches between them and shows/hides the masked-view banner.
- [x] T-20: GDPR-export download button. *(2026-09-03, repo `tasks.md` WEB-5 — copy-adapted from person's reference)* A button on `/workers/[id]` calls the existing `WorkerRepository.exportGdpr(id)` (`GET /api/workers/{id}/export`) and hands the service-defined payload to the browser as a downloaded `worker-<id>-export.json` — serialised verbatim, never interpreted, through a Blob object URL and a synthetic anchor (revoked once the click has fired). An `exporting` state disables the button while the request is in flight; errors go to the existing banner. New keys `detail.exportGdpr` / `detail.exportingGdpr` ×13 locales.
  - **Acceptance:** `tests/unit/workers.test.ts` pins `exportGdpr()` GETs `/api/workers/{id}/export` and returns the payload unchanged; a Playwright smoke test stubs the endpoint, clicks the button, awaits the browser `download` event, and asserts both the suggested filename and that the saved bytes parse back to the stubbed payload.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [x] T-22a: Auth — adopt the BFF + httpOnly-cookie shape: `/signin` +
  `/verify` per-app magic-link pages, `__Host-mxi_session` httpOnly
  cookie (`src/lib/server/session.ts`, `src/hooks.server.ts`), and the
  same-origin `/api/proxy` reverse proxy that exchanges the session for
  a short-lived PASETO server-side (`src/lib/server/auth.ts`) before
  calling the Worker Service. No `mxi_access_token`/`localStorage`
  bearer, no fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
  Landed 2026-06-18 (`f66ff50f`); see `CHANGELOG.md`.
- [x] T-22b (2026-08-28, PRO-H5): CSRF protection on mutating
  browser→BFF calls (`authentication-sessions.md` §4). `/verify`
  additionally sets a second, **non-httpOnly**, Secure, `SameSite=Lax`
  cookie `__Host-mxi_csrf` (`generateCsrfToken()`/`CSRF_COOKIE`/
  `CSRF_COOKIE_OPTIONS`, `src/lib/server/session.ts`); `ApiClient`
  (`src/lib/api/client.ts`) reads it from `document.cookie` when
  running in the browser and echoes it as `X-CSRF-Token` on every
  non-GET/HEAD request; the proxy verifies the header matches the
  cookie (`verifyCsrf`, constant-value equality — both sides are
  BFF-issued, so no timing-safe compare is needed) and additionally
  rejects a present-but-mismatched `Origin`/`Referer`, returning
  `403 {"error":"csrf"}` **without forwarding upstream** on either
  failure. Sign-out (root `+page.server.ts`'s `signout` action) clears
  both cookies. Tests: `tests/unit/session.test.ts` (10,
  `verifyCsrf`/`generateCsrfToken`/cookie-option pins),
  `tests/unit/proxy.test.ts` (7, the route handler exercised directly
  — GET always passes, missing/mismatched token 403s, Origin/Referer
  backstop), `tests/unit/client.test.ts` (+3, the browser
  header-attach path via jsdom's real `document.cookie`, which
  required pointing `vite.config.ts`'s jsdom `testURL` at `https://`
  since a `__Host-`-prefixed cookie only sets over a secure origin).
- [x] T-25 (repo FE-4): **Duplicate review-queue screen** at `/review` —
  2026-08-04. The board itself predates this task and was never fully
  specified. What landed now closes the board's gaps, mirroring the
  person front-end's own T-25 (the reference implementation for this
  fan-out): a **status + page-size filter** wired to `?status=` /
  `?limit=` (`listReviewQueue` gained a `ReviewQueueOptions` argument;
  "all" is the *absence* of `status`, since the endpoint answers `422
  INVALID_STATUS` for a token it does not know — confirmed against
  `worker-service-with-loco/src/api/rest/handlers.rs::get_review_queue`,
  which is byte-for-byte the same guard as person's — and there is no
  `offset` so page size is the whole pagination story); a
  **keyboard-reachable path** — the SVAR Kanban's drag-to-decide is
  mouse-only, so a native queue table with a per-row `Compare` button
  was added alongside it, and the panel carries real `Confirm` /
  `Reject` buttons (drag still works, it is simply no longer the only
  way); and an **inline side-by-side comparison** loading both workers
  with two parallel `GET /api/workers/{id}` calls (`Promise.allSettled`,
  so a soft-deleted side still renders the other) and rendering the
  matcher's `score_breakdown` as a component / weight / score table —
  confirmed against `worker-service-with-loco/src/matching/mod.rs`'s
  `MatchScoreBreakdown` that the seven components and their weights
  (name 0.30, birth date 0.25, gender 0.10, address 0.10, identifier
  0.10, tax id 0.10, document 0.05) are identical in name and value to
  the person service's own in-service matcher — a distinct, simpler
  struct from the `worker-matcher` reference crate's much larger
  ~50-component breakdown (national identifiers + document/phone/email/
  phonetic fields), which is not what powers this queue's stored
  `score_breakdown`. Chosen as an inline expandable section rather than
  a modal, matching person: no focus-trap dependency, keeps the board
  visible as context, cannot strand a keyboard user behind a trap.
  Scope decisions forced by the service: **confirming does not merge**
  (the decision endpoint is a pure status change and the service
  records no link to a merge), so the panel deep-links
  `/workers/merge?main=…&duplicate=…` — in **either** survivor order,
  because a review item names an unordered pair — and `/workers/merge`
  gained a one-line `?main=`/`?duplicate=` seed it did not have before;
  decision buttons are disabled for a non-`pending` item rather than
  offering a request guaranteed to answer `422
  INVALID_REVIEW_TRANSITION`. **Known gap vs. the person reference,
  verified rather than assumed**: `worker-service-with-loco`'s
  `review_queue` table and `ReviewQueueItem` carry **no `provenance`
  column** — person's was added by a dedicated migration
  (`m20260802_000001_review_queue_provenance.rs`) that was never ported
  to worker — so this screen cannot surface a provenance badge the
  service does not send. Backend follow-up, not a front-end omission;
  the front-end will pick it up the day the column exists (out of scope
  here — no Rust service crate was touched by this task). New pure
  module `src/lib/review.ts` (status vocabulary, `canDecide`, the seven
  weighted `MATCH_COMPONENTS`, `breakdownRows`, `mergeHref`). Tests:
  `tests/unit/review.test.ts` (15 — weights sum to 1.00, descending
  order, the null/non-object/unknown-key breakdown paths, the decidable
  guard, both merge-link orders), four repository tests in
  `tests/unit/workers.test.ts` (bare call sends no query string, both
  filters reach the wire, an absent status is omitted rather than sent
  as `"undefined"`, and the decision body's field is `status`), an
  i18n-parity extension for 57 new keys across all 13 locales, and two
  route-stubbed Playwright smoke assertions in `tests/e2e/workers.spec.ts`
  (the board + queue + comparison panel, and the merge page's
  query-param seed) — all passing (`pnpm check`: 0/0; `pnpm vitest run`:
  6 files / 56 tests; `pnpm exec playwright test`: 9/9).
- [x] T-26 (2026-08-29, PRO-H10): **Page-visit guard.** Redirect an
  unauthenticated visitor away from every page whose sole purpose is
  submitting a mutation — `/workers/new`, `/workers/[id]/edit`,
  `/workers/merge`, `/review` each gained a `+page.server.ts` calling
  the new `requireSignedIn(locals)` (`src/lib/server/session.ts`),
  `redirect(303, "/signin")` on no session. Read/list/search/view
  pages stay public — this mirrors the backend's own default-allow-
  read / mutation-deny ABAC posture rather than a separate front-end
  policy. `locals.sessionId` is presence-only, a UX convenience in
  front of the backend's real enforcement, not a substitute for it.
  No `/workers/bulk` route exists to guard — worker carries no bulk
  import/export capability, unlike person's reference implementation
  of this task. Deliberately does not thread a `next` param back
  through `/signin` in v1 (see `AGENTS.md`'s "Page-visit guard"
  section for why — the magic-link round trip does not carry one
  today). Tests: `tests/unit/session.test.ts` (+2 — `requireSignedIn`
  throws a 303-to-`/signin` redirect when signed out, passes through
  silently when signed in).

- [ ] T-27: **Assessment UI.** `worker-service-with-loco` exposes a full
  assessments surface — `create_assessment` / `list_assessments` /
  `get_assessment` / `update_assessment` / `delete_assessment` /
  `assessment_profile` (`src/api/rest/assessments.rs`) over
  `src/models/assessment.rs` / `src/db/assessments.rs` — recording
  aptitude / personality / psychometric / selection assessments per
  worker (per-scale results, score bands, validity, derived profile,
  masked under ABAC), per `agents/share/overview.md`'s explicit worker
  capability callout. This front-end has no surface for it at all: no
  assessment types in `src/lib/api/types.ts`, no repository/client
  module (`src/lib/api/workers.ts` has none), and no route anywhere
  under `src/routes/` (verified: full-repo `grep -rn
  "assessment|psychometric|aptitude|personality"` over `src/` returns
  nothing). Add an assessments panel/route on `/workers/[id]`
  (list + create + the derived profile view), gated by the same
  masking/ABAC posture the detail page already applies to
  identifiers/tax id.
  - **Acceptance:** a new FR in `spec/06-functional-requirements.md`; a
    route-stubbed Playwright smoke test asserting the panel lists
    assessments and the derived profile renders; an i18n-parity
    extension for the new keys across every locale.

- [ ] T-28: **`/expiry` Playwright smoke coverage.** The credential-expiry
  calendar is the only route in the route map with zero e2e coverage
  (verified: `tests/e2e/workers.spec.ts` navigates `/`, `/workers`,
  `/workers/new`, `/workers/match`, `/workers/[id]`, `/workers/merge`,
  `/review`, but never `/expiry` — grep for `page.goto` across the
  file). Add a smoke test stubbing `search()` with documents carrying
  `expiry_date`, asserting the calendar renders
  (`data-testid="expiry-calendar"`) and that selecting an event
  navigates to `/workers/{id}`.
  - **Acceptance:** the new Playwright test passes locally against the
    route stub and is added to the standard `e2e` run.

- [ ] T-29: **Expiry-calendar truncation indicator.** `/expiry`'s
  `onMount` calls `repo.search({ q: "*", limit: 200 })` and only reads
  `res.items`, discarding `res.total` even though
  `WorkerRepository.search()` already returns `{ items, total }`
  (verified: `src/lib/api/workers.ts` lines 82-100). When more than 200
  workers carry documents, the calendar silently shows a partial
  window with no signal to the operator — the code comment even names
  this ("a window, not a promise of completeness") but nothing in the
  UI says so. Surface `res.total` (e.g. "showing up to 200 of N
  workers") when it exceeds the fetched count.
  - **Acceptance:** a component/unit test pins that the truncation
    notice appears when `total > items.length` and is absent otherwise.

- [ ] T-30: **Test coverage for the phonetic search toggle.** `/workers`
  offers a `phonetic` checkbox alongside `fuzzy`
  (`src/routes/workers/+page.svelte`), and `SearchOptions.phonetic` is
  wired all the way to the query string
  (`src/lib/api/workers.ts:93`) — but unlike `fuzzy`, which is pinned
  in `tests/unit/client.test.ts` (asserts `fuzzy=true` on the wire),
  `phonetic` has zero coverage anywhere (verified: `grep -n
  "phonetic" tests/unit/*.test.ts` returns nothing). Add a unit test
  pinning `phonetic=true` reaches the API exactly as `fuzzy` does.
  - **Acceptance:** the new assertion is added to `tests/unit/client.test.ts`
    (or the equivalent repository test file) and passes.

