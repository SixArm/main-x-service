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
- [ ] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Identifier / address / emergency-contact edit (currently read-only on detail; edit form re-PUTs whole record but no UI to add/remove sub-records).
- [ ] T-16: Theming tokens in `app.css` extracted to a small theme module.
- [ ] T-17: `check-duplicates` endpoint wired into create form (preview before commit).
- [ ] T-18: Batch deduplicate-scan results UI.
- [ ] T-19: Masked-view toggle on detail page.
- [ ] T-20: GDPR-export download button.
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

