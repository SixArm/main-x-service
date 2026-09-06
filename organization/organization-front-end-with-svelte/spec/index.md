# Organization Front-End — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test. Live work queue is §13.
>
> Sibling service: [organization-service](../../organization-service-with-loco/spec/index.md).

## 1. Purpose and vision

A small SvelteKit SPA for operators to create, browse, edit, and
duplicate-check organization records via the organization service.

## 2. Scope

In scope: the four routes (`/`, `/new`, `/[pid]`, `/[pid]/edit`), the
API client, the organization form, and the session/auth integration
(§6.7/§6.8 — BFF + httpOnly-cookie + PASETO per
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md);
service-side enforcement off by default). Out of scope: full-text search
UI, audit views.

## 3. Stakeholders and users

Operators curating the organization registry.

## 4. Glossary

- **pid** — the organization's public id (route param).
- **Organization** — the `organization_matcher::Organization` payload.
- **check-duplicates** — POST the current record to find stored matches.

## 5. Information architecture

```
/               list of organizations
/organizations  SVAR grid + filter index
/new            create form
/[pid]          detail + delete + check-duplicates
/[pid]/edit     edit form
/review         drag-to-decide duplicate review board (SVAR Kanban)
/merge          record merge (main + duplicate pid, preview, history)
/signin         magic-link sign-in (BFF)
/verify         magic-link verification (BFF)
```

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.

## 6. Functional requirements

1. List active organizations (`GET /api/organizations`).
2. Create (`POST`), redirect to the new detail page.
3. Detail: render the stored `Organization`; offer edit, delete, and
   check-duplicates.
4. Edit (`PUT`), redirect back to detail.
5. Delete (`DELETE`, soft), redirect to the list.
6. Check-duplicates posts the current record and lists matches (name,
   score, confidence), excluding the record itself.
7. **Session (BFF + httpOnly cookie).** The browser holds only the
   `__Host-mxi_session` httpOnly cookie — no token in JS, no
   `localStorage`. The SvelteKit **server** (BFF) holds the session and,
   when calling the organization service, attaches a short-lived PASETO
   server-side; the browser never calls the service directly. Mutating
   browser→BFF calls carry a CSRF token. Service-side enforcement
   (`ORGANIZATION_REQUIRE_AUTH`) is off by default. Per
   [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
8. **Sign-in.** When signed out the layout shows a primary **Sign in**
   action routed through the BFF to the central authentication front-end
   for the passwordless magic-link; on success the authentication-service
   sets the `__Host-mxi_session` cookie. There is no client-held access
   token and no URL-fragment handoff. Per
   [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
9. **Layout shell.** Global navigation is a full-width **top bar**
   (header) with a **hamburger** toggle on narrow viewports — NOT a left
   sidebar — and the main content area is **full-width**.

## 7. Non-functional requirements

Svelte 5 runes only; TS strict (`noUncheckedIndexedAccess`); SPA;
dependency-light (no data grid / design system).

## 8. Architecture

`ApiClient` (lean, raw-JSON, get/post/put/delete) → `OrganizationRepository`
→ routes. The `Organization` payload is the matcher shape serialized
snake_case: `name`, `legal_name`, `alternate_names`, `identifiers`,
`url`, `same_as`, `address`, `jurisdiction`, `founding_date`,
`telephone`, `email` (both contact fields; personal data — see §12),
`keywords`. `OrganizationForm` edits these and assembles the payload via
the shared, unit-tested `src/lib/api/build.ts` (`buildOrganization` +
`splitList`/`blankToUndef`): comma lists split, blanks → `null`, address
assembled all-or-nothing only if any part is set, empty identifier rows
dropped. The detail route's `excludeSelf` (same module) drops the
record's own pid from check-duplicates results (§6.6).

## 9. API consumption

| Route / action | Endpoint |
|---|---|
| `/` | `GET /api/organizations` |
| `/new` | `POST /api/organizations` |
| `/[pid]` load | `GET /api/organizations/{pid}` |
| `/[pid]` delete | `DELETE /api/organizations/{pid}` |
| `/[pid]` duplicates | `POST /api/organizations/check-duplicates` |
| `/[pid]/edit` | `PUT /api/organizations/{pid}` |
| `/organizations` | `GET /api/organizations` (SVAR grid + client-side filter) |
| `/review` scan | `POST /api/organizations/deduplicate` |
| `/review` load | `GET /api/organizations/review-queue` |
| `/review` decide | `POST /api/organizations/review-queue/{id}/decision` |
| `/merge` preview | `GET /api/organizations/{pid}` ×2 |
| `/merge` submit | `POST /api/organizations/merge` |
| `/merge` history | `GET /api/organizations/merges/recent` |

## 10. Persistence

None client-side beyond in-memory route state.

## 11. Testing strategy

`pnpm run check` (svelte-check strict, 0/0). **vitest** unit tests
(`tests/unit/`, 54 across 6 files) cover:
- `client.test.ts` — the `ApiClient` (verb/body/headers, error
  classification, empty-body handling). The BFF proxy injects the
  PASETO server-side (§6.7), so this client carries no browser-held
  bearer-token path.
- `organizations.test.ts` — `OrganizationRepository` (every method's
  path + verb, incl. regressions pinning `check-duplicates`, `merge`,
  and `recentMerges`).
- `build.test.ts` — the spec §8 core in `src/lib/api/build.ts`:
  `buildOrganization` (blank → `null`, comma-list split, contact fields,
  all-or-nothing address, dropping empty identifier rows),
  `splitList`/`blankToUndef`, and `excludeSelf` (§6.6 self-match drop).
- `merge-validation.test.ts` — the `/merge` guard (both ids required,
  must differ), returning i18n keys rather than English strings.
- `i18n.test.ts` — the 13-locale catalog: exact locale set, a label per
  locale, **full key coverage in every locale** (no missing/extra
  keys), default locale, RTL detection (`ar`/`ur`), and fallback to
  English then to the key itself.
- `layout.test.ts` — the layout's session-panel behaviour.

**Playwright** smoke tests (`tests/e2e/smoke.spec.ts`, 7) load `/`,
`/new`, `/[pid]`, `/[pid]/edit`, `/review`, and `/merge` with the API
stubbed via `page.route`, asserting each renders, plus a nav-reachability
check for Merge; they run against the production build (`vite preview`)
to avoid the `vite dev` cold-start module race.
Run: `pnpm test` (vitest) and `pnpm test:e2e` (Playwright).

## 12. Compliance

Contact fields may be personal data; defer to the service's privacy
controls when they land.

## 13. Tasks (live work queue)

- [x] vitest unit tests (`tests/unit/`, 49 across 5 files) for the
  `ApiClient`, `OrganizationRepository`, `auth` store + SSO handoff
  (`captureTokenFromHash` / `captureFromLocation`), `signInUrl`, and the
  form/payload core in `build.ts` (`buildOrganization`, `splitList`,
  `blankToUndef`, `excludeSelf`).
- [x] playwright smoke for the four routes (`tests/e2e/smoke.spec.ts`,
  4 tests, API stubbed, runs against `vite preview`).
- [ ] Identifier `Custom(label)` editing in the form.
- [ ] Search box — the service's Tantivy full-text search shipped
  2026-07-31 (`GET /api/organizations/search?q=[&fuzzy][&phonetic]`,
  see the service `AGENTS.md`/spec API surface table), so this is no
  longer blocked on the service; it is a real open UI task now.
- [x] ~~Bearer token wiring — `auth.svelte.ts` token store
  (`localStorage["mxi_access_token"]`) + `ApiClient` auto-attach +
  layout session affordance~~ — **superseded** (see auth-migration task below).
- [x] ~~Cross-origin SSO handoff — `signInUrl()` redirect +
  `captureFromLocation()` / `captureTokenFromHash` fragment capture~~ —
  **superseded** (see auth-migration task below).
- [x] Auth — adopt BFF + httpOnly cookie + CSRF; remove
  `mxi_access_token`/`localStorage` bearer + fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
  **Done:** `src/lib/server/` (session cookie + magic-link +
  session→PASETO exchange), `/signin` + `/verify` routes, and the
  same-origin `/api/proxy` BFF route that injects the PASETO bearer
  server-side; the browser holds no token.

- [x] **2026-07-19 — `/review` drag-to-decide board.** Stored review
  queue as a SVAR Kanban (loads on mount via `GET /review-queue`; the
  destructive scan stays button-only; drag pending → confirmed/rejected
  posts the decision). Verified: svelte-check 0 errors, vitest 43
  (repo path pins for the three new methods), Playwright 5 (stubbed
  review-board smoke incl. a no-scan-on-load pin).

- [x] **FE-1 — `/merge` record-merge UI.** Main + duplicate pid, optional
  reason, side-by-side preview (`GET /{pid}` ×2), `confirm()`-gated
  `POST /api/organizations/merge`, and a merge-history table from
  `GET /api/organizations/merges/recent` (loads on mount — a safe GET —
  and refreshes after a merge). The wire shape is this service's own:
  request `{main_pid, duplicate_pid, reason}`, response
  `{main_pid, duplicate_pid, main}` with **no** `merge_record` wrapper,
  so the completion panel links to the survivor rather than quoting a
  record id. The guard (`src/lib/components/merge-validation.ts`) returns
  an i18n **key**, not an English sentence, so the message follows the
  chosen locale. Verified: svelte-check 0 errors, vitest 53, `pnpm build`
  clean; 27 new keys × 13 locales.

- [x] **FE-4 — `/review` upgraded to the person T-25 standard
  (2026-08-04).** Brought the 2026-07-19 Kanban-only board (above) to
  parity with `person-front-end-with-svelte`'s `/review` completion
  (root `tasks.md` FE-4, person-service's own `spec/13-tasks.md` T-25):
  `?status=`/`?limit=` filters (no `offset`; `"all"` is the *absence* of
  `status`, verified against this crate's own
  `controllers/organizations.rs::get_review_queue` — a literal `"all"`
  is `422`, not a request that silently means "every status"), a
  keyboard-reachable `<table>` with real `Compare`/`Confirm`/`Reject`
  buttons alongside the existing drag-to-decide board, `provenance` on
  both surfaces, and an inline (non-modal) side-by-side comparison panel.
  One deviation from the person pattern, forced by this service's own
  wire shape rather than chosen freely: `GET /review-queue`'s
  `ReviewQueueItem` never serializes `score_breakdown` (the column
  exists in `review_queue`; the controller's response struct omits it),
  so a stored breakdown can never reach the browser. Rather than render
  a permanently-empty table, the comparison panel calls the already-shipped
  `POST /api/organizations/match` against the loaded pair for a **live**
  breakdown — reusing an existing no-persistence endpoint, not adding one.
  `ReviewQueueItem`'s TypeScript type was also missing `provenance`
  outright (a pre-existing gap since BLK-5 added the column
  server-side); fixed. `/merge` gained `?main=&duplicate=` prefill via
  `$app/state` so `$lib/review`'s `mergeHref` deep-link (shown once an
  item is `confirmed`) actually lands filled in. New `src/lib/review.ts`
  (pure; `MATCH_COMPONENTS` mirrors this matcher's six weights —
  name 0.35 / address 0.20 / url 0.15 / jurisdiction 0.10 /
  founding-date 0.10 / keywords 0.10 — summing to 1.00). 57 new i18n
  keys × 13 locales. Verified: svelte-check 0/0, vitest 72 (was 54;
  +15 `review.test.ts`, +3 repository pins), `pnpm build` clean,
  Playwright 10/10 (was 7; +3: keyboard table, live-breakdown compare,
  merge query-string prefill).

- [ ] **ORGFE-T1 (M) Page-visit auth guard (`requireSignedIn`).** No
  route in this front-end redirects an unauthenticated visitor — every
  page (`/`, `/new`, `/[pid]`, `/[pid]/edit`, `/review`, `/merge`) loads
  and calls the BFF proxy regardless of session state; only the layout
  chrome's "signed in" panel reflects `locals.sessionId`.
  *(Verified: `grep -rn requireSignedIn src/` returns no hits, and
  `src/hooks.server.ts` / `src/routes/+layout.server.ts` only *read*
  `locals.sessionId` into `signedIn`, never redirect on its absence —
  matches the repo `tasks.md` WEB-1 finding that only 5 of 16
  front-ends carry this guard.)* Add a shared `requireSignedIn` helper
  (or per-route `+page.server.ts`/`+layout.server.ts` load guard)
  redirecting to `/signin` when `locals.sessionId` is null, on every
  mutating/record-bearing route. **Acceptance:** a vitest/Playwright
  test hits a protected route with no session cookie and asserts a
  redirect to `/signin`; the existing signed-out layout affordance is
  unchanged.

- [x] **ORGFE-T2 (S) Masked view + GDPR export UI.** *(resolved
  2026-09-06.)* The service exposes `GET /{pid}/masked` and `GET
  /{pid}/export`, but nothing in this front-end called either.
  *(Verified: `grep -rn "masked\|/export" src/lib/api/*.ts
  src/routes/` returned no hits before this change.)*
  - **Resolved.** `OrganizationRepository.masked(pid)` /
    `.exportGdpr(pid)` added, plus a detail-page ("Show masked"/"Show
    full", `aria-pressed`) toggle that re-fetches through the masked
    endpoint on click rather than redacting client-side (copy-adapted
    from place/event-front-end's equivalent masked-view toggle), and
    a "Export data (GDPR)" button that downloads the export envelope
    as `organization-<pid>-export.json` via a Blob object URL +
    synthetic anchor (the payload is service-defined and never
    interpreted, only serialized). A banner appears while the masked
    view is shown. New i18n keys `detail.showMasked` /
    `detail.showFull` / `detail.maskedNotice` / `detail.exportGdpr` /
    `detail.exportingGdpr` across all 13 locales.
  - Two new `tests/unit/organizations.test.ts` cases pin `masked()`
    and `exportGdpr()` hit their dedicated endpoints. Two new
    `tests/e2e/smoke.spec.ts` cases: one stubs plain vs. masked
    responses (distinguished by `/masked` in the URL, different
    `name` values) and toggles both ways; one stubs the export
    endpoint, clicks the button, awaits the browser `download` event,
    and asserts both the suggested filename and that the saved bytes
    parse back to the stubbed payload. Both verified to fail (30s
    timeout) with the detail-page change reverted and pass with it
    restored.
  - **Acceptance met:** `pnpm test` 77/77 (was 75); `pnpm run check`
    clean; `pnpm exec playwright test` 12/12 (was 10); `pnpm run
    lint` clean.

- [ ] **ORGFE-T3 (S) Audit-trail view.** The service exposes `GET
  /api/organizations/audit/recent` and `GET /{pid}/audit`
  (`agents/share/auditability.md`), and this front-end's own roadmap
  names "audit views" as next work, but no route or repository method
  calls either endpoint. *(Verified: `find src/routes -maxdepth 1
  -type d` lists no audit route, and `grep -rn audit
  src/lib/api/*.ts` returns no hits.)* Add an `audit(pid)` repository
  method and a detail-page "Show audit trail" toggle, matching the
  pattern already shipped in the sibling
  `care-pathway-front-end-with-svelte`. **Acceptance:** vitest pins the
  new repository method; a Playwright smoke test covers the toggle
  (loading/empty/error states).

- [ ] **ORGFE-T4 (S) Inline identifier-format validation (LEI/DUNS
  length).** Named as an open question in §16 since v0.1; the service
  now validates LEI/GLN/DUNS/VAT check digits server-side (SEC-M5), but
  the create/edit form gives no client-side hint before the round-trip
  `422`. *(Verified: `agents/share/security.md` §2 SEC-M5 confirms the
  server-side check; no client-side format check exists in
  `src/lib/components/OrganizationForm.svelte`.)* Add pure length/format
  hints (not full check-digit re-implementation — that stays
  server-authoritative) surfaced inline per identifier row.
  **Acceptance:** a vitest unit test on the new pure helper; the form
  still submits and relies on the server's `422` as the authority.

- [ ] **ORGFE-T5 (S) Real-time duplicate warning on the create form.**
  Also named as an open question in §16 since v0.1: the create form has
  no live duplicate hint, only the post-submit `check-duplicates`/`409`
  path (once ORG-T3 lands service-side) or the separate `/review` flow.
  *(Verified: `src/routes/new/+page.svelte` calls only
  `repository.create()`, no debounced `checkDuplicates()` call exists
  on the form.)* Add a debounced `checkDuplicates()` call as the
  operator fills in name/address, surfacing candidate matches inline
  before submit. **Acceptance:** a vitest test on the debounce/candidate
  logic; a Playwright smoke test (API stubbed) shows the warning
  appearing for a stubbed duplicate response.

- [x] **ORGFE-T6: `/verify` crashed with a raw 500 when the authentication service was unreachable.** *(resolved 2026-09-06.)* `src/routes/verify/+page.server.ts` called `await verifyMagicLink(fetch, token)` with no `try`/`catch`. A network-level failure (the authentication service unreachable, timed out, connection reset) makes `fetch` throw rather than resolve — uncaught, that propagated out of `load` and SvelteKit rendered its generic 500 error page instead of this route's own friendly UI. The same bug class was found and fixed first in `place-front-end-with-svelte` (T-26) and `thing-front-end-with-svelte` (T-23); ported here.
  - **Resolved.** A `try`/`catch` around the call, a new `"serviceUnavailable"` error variant, and its message in `+page.svelte`.
  - **Acceptance:** `tests/unit/verify.test.ts` (new) unit-tests the `load` function directly — pinning `missingToken`, the new `serviceUnavailable` (fetch rejects), and `invalidToken` (non-ok response) branches — verified to fail with the `try`/`catch` reverted and pass with it restored. Three-part change: spec (here) + code + test.

## 14. Implementation status

Done: the eight routes in §5 (list, `/organizations` grid, create,
detail, edit, `/review` duplicate board, `/merge` record merge, plus
`/signin`/`/verify`); lean client (+put/delete); repository covering
CRUD, check-duplicates, batch deduplicate, the stored review queue
(status/limit filters + live match-based score breakdown), and merge +
merge history (with query-string pre-fill); form; the BFF
(§6.7/§6.8 — session cookie, `/api/proxy`, magic-link sign-in), SPA
config. `pnpm run check` clean; production build succeeds; 72 vitest +
10 Playwright, all 13 locales at full key coverage.

## 15. Roadmap

v0.1 (here): CRUD + duplicate-check UI, vitest + Playwright suites. The
v0.1 session shipped as a client-held bearer + cross-origin SSO handoff;
that is now superseded by the BFF + httpOnly-cookie model (§6.7/§6.8,
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)),
which has since shipped (§13). Next: the search box (§13 — no longer
blocked, the service's Tantivy search shipped 2026-07-31); audit views.

## 16. Open questions

- Real-time duplicate warning on the create form (vs the detail page)?
- Inline validation of identifier formats (LEI/DUNS length)?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
