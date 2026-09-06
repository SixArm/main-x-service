# Case Front-End — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test. Live work queue is §13.
>
> Sibling service: [case-service](../../case-service-with-loco/spec/index.md).

## 1. Purpose and vision

A small SvelteKit SPA for caseworkers to create, browse, edit, and
duplicate-check governmental case-management records via the case
service.

## 2. Scope

In scope: the routes (`/`, `/cases`, `/board`, `/new`, `/merge`, `/audit`,
`/[pid]`, `/[pid]/edit`, `/[pid]/audit`, `/signin`, `/verify`), the
API client, the case form, and a BFF + httpOnly-cookie session (§6.7/§6.8,
per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
Full-text search (`/`) and audit views (`/[pid]/audit`, `/audit`) landed
2026-08-29 (PRO-P15), once the service shipped Tantivy search
(2026-08-02). Out of scope: consent/GDPR export UI (the service exposes
no such endpoints for case).

## 3. Stakeholders and users

Caseworkers and case administrators across governmental agencies.

## 4. Glossary

- **pid** — the case's public id (route param).
- **Case** — the `case_matcher::Case` payload.
- **check-duplicates** — POST the current record to find stored matches.

## 5. Information architecture

```
/               list of cases + full-text search box (fuzzy/phonetic toggles)
/cases          SVAR DataGrid + FilterBar index (client-side filtering)
/board          SVAR Kanban board, one column per status; drag-to-change-status
/new            create form
/[pid]          detail + delete + check-duplicates + "subject of this case" links panel + link to audit trail
/[pid]/edit     edit form
/[pid]/audit    one case's audit trail, newest first
/merge          merge a duplicate into a survivor + recent merge history
/audit          system-wide recent activity: recent audit entries + recent events
```

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.

## 6. Functional requirements

1. List active cases (`GET /api/cases`).
2. Create (`POST`), redirect to the new detail page.
3. Detail: render the stored `Case`; offer edit, delete, and
   check-duplicates.
4. Edit (`PUT`), redirect back to detail.
5. Delete (`DELETE`, soft), redirect to the list.
6. Check-duplicates posts the current record and lists matches (title,
   score, confidence), excluding the record itself.
7. Session affordance (BFF + httpOnly cookie): the top navigation bar
   offers a primary **Sign in** link and, once signed in, **Sign out**.
   The browser holds only the `__Host-mxi_session` httpOnly cookie — **no
   token in JS, no `localStorage`**. The SvelteKit **server** (BFF) holds
   the session and attaches a short-lived PASETO server-side when calling
   the case service, so operator traffic passes the service's blanket
   enforcement (`CASE_REQUIRE_AUTH`) once activated. Mutating browser→BFF
   calls carry a CSRF token. Per
   [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
8. Sign-in (BFF): **Sign in** is routed through the BFF to the central
   **authentication-service** front-end for the passwordless magic-link;
   on success the authentication-service sets the `__Host-mxi_session`
   cookie. There is no client-held access token and **no URL-fragment
   handoff**. Per
   [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
9. Merge (`/merge`): take a surviving `main_pid` and a `duplicate_pid`
   plus an optional reason, optionally preview both records, and merge
   after a confirmation (`POST /api/cases/merge`). The pre-flight guard
   (both ids present, and distinct — the service answers `422` on a
   self-merge) is a pure helper so it is unit-testable. The response is
   `{main_pid, duplicate_pid, main}` — the service returns no
   `merge_record` wrapper, so the page shows the survivor's pid and links
   to it, and reads merge timestamps from the history below. That history
   is `GET /api/cases/merges/recent` (newest first, service-capped at
   100), rendered as merged-at / main / duplicate / reason / actor.
10. Cross-service links — "subject of this case" (detail route): list,
   assert, and withdraw the `subject_of` (case → person) edges this case
   originates (`GET`/`POST`/`DELETE /api/cases/{pid}/links`). Case
   originates exactly **one** edge kind
   ([`../../../agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md)
   §9), so the panel offers **no kind picker** — `kind` is fixed to
   `subject_of` and any other value is a service-side `422`. The edge is
   **high-sensitivity** (§10 of that doc): it asserts that a named person
   is the subject of a governmental case, is authorised at the same level
   as reading the case, and is audited on every write. The UI reflects
   that — a plainly-labelled section with an explanatory note rather than
   a casual inline control, and an explicit `confirm()` naming the person
   reference before a withdrawal. A pure `validateLink` guard mirrors the
   service's `validate_edge` (person `EntityRef` URN shape; confidence in
   `[0,1]`) so an obviously-doomed request states its reason locally, in
   the operator's own locale. Server rejections are surfaced inline from
   loco's `ErrorDetail.description` (the `error` field carries the machine
   code `validation`, not the reason).
11. Layout shell: global navigation is a full-width **top bar** (header)
   with a **hamburger** toggle on narrow viewports — NOT a left sidebar —
   and the main content area is **full-width**.
12. Full-text search (`/`, PRO-P15): a search box (`SearchBox.svelte`,
   copy-adapted from the course front-end's dependency-light pattern —
   the reference for a lightweight search box in this family) above the
   list, with **fuzzy** and **phonetic** checkboxes. A non-blank query
   runs `GET /api/cases/search?q=&fuzzy=&phonetic=&limit=&offset=`
   (Tantivy, service spec T-6); a blank query falls back to the plain
   list (`GET /api/cases`) — the service rejects a blank `q` with `400`,
   so the client never sends one. The result count reads `{shown} /
   {total}` from the paginated response when the page is partial (same
   convention as `/cases`' FilterBar count).
13. Audit trail (`/[pid]/audit`, PRO-P15): one case's audit-log rows,
   newest first (`GET /api/cases/{pid}/audit`) — action, timestamp,
   actor (or none), and an expandable JSON snapshot where present.
   Linked from the detail route's action row. Modelled on the
   `[id]/audit` dedicated-route pattern the majority of sibling
   front-ends use (course/event/person/place/thing/worker), rather than
   care-pathway's inline toggle, since it matches this app's own
   `/[pid]/edit` routing convention.
14. Recent activity (`/audit`, PRO-P15): system-wide, across every case
   — recent audit-log entries (`GET /api/cases/audit/recent`, cap 100)
   and the recent CRUD/merge event stream (`GET
   /api/cases/events/recent`, cap 100), each panel loading and failing
   independently. Reachable from the top nav.

## 7. Non-functional requirements

Svelte 5 runes only; TS strict (`noUncheckedIndexedAccess`); SPA. Started
dependency-light (v0.1, no data grid / design system); SVAR DataGrid +
FilterBar (`/cases`), SVAR Kanban (`/board`), and the Lily
`ThemePicker`/`LocalePicker` were added 2026-07-19 (see `CHANGELOG.md`) —
each is a real, used dependency, not a speculative install, though
`@svar-ui/svelte-calendar`, `@svar-ui/svelte-gantt`, and
`@svar-ui/svelte-filemanager` are installed with no route using them yet
(candidate features, catalogued in the roadmap). The core CRUD/merge/links
forms remain plain inputs + `app.css` utilities.

## 8. Architecture

`ApiClient` (lean, raw-JSON, get/post/put/delete) → `CaseRepository`
→ routes. `CaseForm` builds a `Case` from the inputs (comma lists
split, blanks nulled, case type / status / priority / identifier
schemes from `ALL_*` dropdowns, identifiers as editable rows). Under the
BFF model (§6.7) the browser carries only the `__Host-mxi_session` cookie
and the SvelteKit server attaches the short-lived PASETO server-side when
calling the service; no token is read or attached in browser JS.

## 9. API consumption

| Route / action | Endpoint |
|---|---|
| `/` | `GET /api/cases` |
| `/new` | `POST /api/cases` |
| `/[pid]` load | `GET /api/cases/{pid}` |
| `/[pid]` delete | `DELETE /api/cases/{pid}` |
| `/[pid]` duplicates | `POST /api/cases/check-duplicates` |
| `/[pid]/edit` | `PUT /api/cases/{pid}` |
| `/merge` submit | `POST /api/cases/merge` |
| `/merge` history | `GET /api/cases/merges/recent` |
| `/[pid]` links list | `GET /api/cases/{pid}/links` |
| `/[pid]` link assert | `POST /api/cases/{pid}/links` |
| `/[pid]` link withdraw | `DELETE /api/cases/{pid}/links/{id}` |
| `/` search | `GET /api/cases/search?q=&fuzzy=&phonetic=&limit=&offset=` |
| `/[pid]/audit` | `GET /api/cases/{pid}/audit` |
| `/audit` recent audit | `GET /api/cases/audit/recent` |
| `/audit` recent events | `GET /api/cases/events/recent` |

The privileged cross-case dump `GET /api/cases/links` (the aggregator's
reconciliation pull, gated as a destructive governed read) is deliberately
**not** consumed here — an operator UI has no use for every `subject_of`
edge in the service at once.

## 10. Persistence

None client-side beyond in-memory route state.

## 11. Testing strategy

`pnpm run check` (svelte-check strict, 0/0). **vitest** unit tests
(`tests/unit/`, 70 tests across 7 files) cover: `client.test.ts` (the
`ApiClient` — verb/body/headers, per-call `token` override, `getPage()`
pagination-header parsing, error-classification/empty-body);
`cases.test.ts` (`CaseRepository` — every method's path + verb, incl. a
regression pinning `check-duplicates`, the merge/links methods, and
(PRO-P15) `search()`'s query-string shape — `q` alone, `fuzzy`/`phonetic`
included only when `true`, `limit`/`offset` via the shared pager, and
URL-encoding — plus `audit()`/`recentAudit()`/`recentEvents()`);
`case-form.test.ts` (form-to-`Case` assembly); `i18n.test.ts` (13-locale
parity — every locale covers every key with no extras, RTL detection,
English fallback, plus dedicated coverage assertions for the `merge.*`
and `links.*` blocks including their `{dup}`/`{main}`/`{ref}`
placeholders); `layout.test.ts` (nav); `merge-validation.test.ts` (the
pure `validateMerge` guard, incl. local self-merge rejection); and
`link-validation.test.ts` (the pure `validateLink` guard — `EntityRef`
URN shape, confidence bounds).
**Playwright** smoke tests (`tests/e2e/smoke.spec.ts`, 8 tests) load
`/`, `/new`, `/[pid]` (incl. the links panel), `/[pid]/edit`, `/merge`,
plus the nav-exposes-merge-link and check-duplicates-self-exclusion
cases, with the API stubbed via `page.route`; they run against the
production build (`vite preview`) to avoid the `vite dev` cold-start
module race. The stub dispatches on `url.pathname` with the `/api/proxy`
BFF-proxy prefix stripped first — the client's requests land on
`/api/proxy/api/cases`, not `/api/cases`, so a stub matching only the
bare service path silently 404s every request (found and fixed
2026-08-04: all 8 were failing under the bare-path comparison,
undetected because `svelte-check`/vitest/build all stay green
regardless).
Run: `pnpm test` (vitest) and `pnpm test:e2e` (Playwright).

## 12. Compliance

Cases are governmental records; defer to the service's controls for any
access/audit requirements.

## 13. Tasks (live work queue)

- [ ] **FE-3 (M) Masked view + GDPR export UI.** The service has
  carried `GET /api/cases/{pid}/masked` and `GET /api/cases/{pid}/export`
  since 2026-08-02 (this crate's own `AGENTS.md` "API consumption"
  table lists both), but `CaseRepository` has no `masked()`/`export()`
  method and no route calls either path — *(verified:
  `grep -n "  async \|(pid" src/lib/api/cases.ts` lists `get`/`update`/
  `remove`/`audit` and no masked/export method; a repo-wide grep for
  `masked` and `/export` under `src/lib/api/cases.ts` and
  `src/routes/` returns nothing)*. Add `CaseRepository.masked(pid)` /
  `.export(pid)`, and a "View masked" / "Export (GDPR)" affordance on
  `/[pid]` — even a plain link/button that renders the JSON is enough
  to make the capability reachable; a dedicated view is a stretch goal.
  Three-part change: spec §6/§9/§13 + `src/lib/api/cases.ts` +
  `src/routes/[pid]/+page.svelte` + vitest coverage.
  **Acceptance:** new vitest cases pin the two new repository methods'
  URLs; a Playwright smoke test exercises the masked view (or export)
  affordance from `/[pid]`.

- [ ] **FE-4 (M) Decide and, if adopted, implement the `requireSignedIn`
  page-visit guard (PRO-H10).** Repo `tasks.md` WEB-1 found the guard
  rolled to 5 of 16 front-ends (person, worker, thing, course, event)
  and explicitly left "the 5/11 roll-out question... still open" for
  the rest, case included — *(verified:
  `grep -rn "requireSignedIn" src/` in this project returns zero
  matches, matching WEB-1's finding)*. This project's own auth model
  (BFF + httpOnly session cookie, §8) is otherwise identical to the
  five that do have the guard, so there is no structural reason it
  couldn't apply the same page-visit check to `/new`, `/merge`,
  `/[pid]/edit` and similar mutating routes. Either add it (following
  person's `requireSignedIn(locals)` pattern in `+page.server.ts`,
  plus the WEB-1 fix's `SMOKE_STORAGE_STATE` Playwright stub-cookie
  approach so the smoke suite still renders guarded pages) or record
  in this spec (§8/§13) why case deliberately opts out — either answer
  closes the open question for this one front-end.
  **Acceptance:** either `requireSignedIn` gates the mutating routes
  with a pinned anonymous-303 Playwright test (mirroring person's), or
  spec §8 states the explicit reason case stays unguarded.

- [x] **FE-5 (S) `Custom(label)` editing for case type / status / schemes.**
  *(resolved 2026-09-06.)* Already an open §16/roadmap item ("Still
  open: ... `Custom(label)` editing") but never promoted to a §13
  checkbox with a concrete scope — *(verified: `grep -n "Custom"
  src/lib/components/CaseForm.svelte` showed no `Custom` variant
  handling in the form; the `case_matcher` wire shape carries a
  `Custom(String)` case-type/status/identifier-scheme variant per the
  sibling matcher's serde docs)*.
  - **Resolved.** Each of the three dropdowns (case type, status, and
    every identifier row's scheme) gained a "Custom" option; selecting
    it reveals a text input (`aria-label="Custom label"`, distinct
    from the wrapping `<label>` so it doesn't collide with the
    select's own accessible name) bound to a new `*Custom` state
    field, reassembled in `build()` into the `{ Custom: "<label>" }`
    wire shape. Client-side validation blocks submit with a new
    `form.customLabelRequired` message ("A custom label is required.")
    rather than ever sending `{ Custom: "" }`. Identifier rows changed
    shape internally (`IdentifierRow { schemeKind, customLabel, value
    }` instead of a bare `CaseIdentifier`) to let a native `<select>`
    bind the sentinel while keeping a separate label field per row —
    a fix that incidentally corrects a real (if minor) prior bug: a
    seeded Custom-scheme identifier used to be silently dropped on
    load (the scheme `<select>` only offered unit schemes); it now
    round-trips instead. New i18n keys `form.customLabel` /
    `form.customLabelRequired` across all 13 locales.
  - `tests/unit/case-form.test.ts` gained 4 new cases (one per
    dropdown proving the `{ Custom: "<label>" }` round-trip, plus the
    blank-label validation block) and one existing case was rewritten
    from "drops seeded Custom-scheme identifier rows" to "preserves a
    seeded Custom-scheme identifier row", reflecting the corrected
    behaviour rather than pinning the bug. All 5 verified to fail with
    `CaseForm.svelte`'s changes reverted and pass with them restored.
  - **Acceptance met:** selecting `Custom` in any of the three
    dropdowns reveals a label input; a round-trip vitest case pins the
    payload shape (`{Custom: "<label>"}`) sent on save, for all three.
    `pnpm test` 77/77 (was 73); `pnpm run check` clean; `pnpm exec
    playwright test` 8/8 (unchanged); `pnpm run lint` clean save for
    two pre-existing, untouched files (`src/lib/api/client.ts`,
    `src/lib/svar-filter-augment.d.ts`).

- [x] vitest unit tests for `ApiClient` + `CaseRepository` + `CaseForm`
  assembly + i18n parity + layout + the merge/link validators
  (`tests/unit/`, 61 tests across 7 files — see §11).
- [x] playwright smoke for the routes + check-duplicates self-exclusion
  (`tests/e2e/smoke.spec.ts`, 8 tests, API stubbed, runs against `vite
  preview`; see §11 on the 2026-08-04 BFF-proxy-path stub fix).
- [x] ~~Cross-origin SSO token handoff (consumer side): capture token
  from the URL fragment + strip it; `signInUrl` builder + top-bar **Sign
  in** redirect~~ — **superseded** (see auth-migration task below).
- [x] Merge UI (`/merge`): merge form + preview + recent merge history,
  `CaseRepository.merge()` / `recentMerges()`, the pure `validateMerge`
  guard, 13-locale strings, unit + smoke tests.
- [x] Cross-service links panel on `/[pid]` (FE-2): list / assert /
  withdraw `subject_of` edges, `CaseRepository.listLinks()` /
  `createLink()` / `deleteLink()`, the pure `validateLink` guard,
  13-locale strings, unit + smoke tests.
- [x] `Custom(label)` editing for case type / status / schemes —
  promoted to FE-5 above and resolved there.
- [x] **PRO-P15** *(done 2026-08-29)* Search box + audit/event views —
  the service's Tantivy search (T-6, landed 2026-08-02) had unblocked
  this weeks earlier. Added: a `SearchBox.svelte` (copy-adapted from the
  course front-end's dependency-light pattern) on `/` with fuzzy/phonetic
  toggles, wired to `CaseRepository.search()` (`GET
  /api/cases/search?q=&fuzzy=&phonetic=&limit=&offset=`; a blank query
  falls back to `listPage()` since the service 400s a blank `q`); a
  dedicated `/[pid]/audit` route (`CaseRepository.audit()`, `GET
  /api/cases/{pid}/audit`), linked from the detail page's action row;
  and a system-wide `/audit` "recent activity" route combining
  `recentAudit()` (`GET /api/cases/audit/recent`) and `recentEvents()`
  (`GET /api/cases/events/recent`), reachable from the top nav. New
  `AuditEntry`/`CaseEvent`/`SearchParams` types mirror the service's
  `audit_logs` entity and `EventView` (`src/lib/api/types.ts`). 9 new
  vitest cases pin `search()`'s query-string shape and the three new GET
  paths (`tests/unit/cases.test.ts`, 61→70); 21 new i18n keys
  (`search.*`, `audit.*`, `activity.*`, `nav.audit`,
  `detail.viewAudit`) added across all 13 locales, `i18n.test.ts`'s
  full-coverage assertion still passes. `pnpm run check` clean (0/0).
- [x] Auth — adopt BFF + httpOnly cookie + CSRF; remove
  `mxi_access_token`/`localStorage` bearer + fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).

- [x] **T-7: `/verify` crashed with a raw 500 when the authentication service was unreachable.** *(resolved 2026-09-06.)* `src/routes/verify/+page.server.ts` called `await verifyMagicLink(fetch, token)` with no `try`/`catch`. A network-level failure (the authentication service unreachable, timed out, connection reset) makes `fetch` throw rather than resolve — uncaught, that propagated out of `load` and SvelteKit rendered its generic 500 error page instead of this route's own friendly UI. The same bug class was found and fixed first in `place-front-end-with-svelte` (T-26) and `thing-front-end-with-svelte` (T-23); ported here.
  - **Resolved.** A `try`/`catch` around the call, a new `"serviceUnavailable"` error variant, and its message in `+page.svelte`.
  - **Acceptance:** `tests/unit/verify.test.ts` (new) unit-tests the `load` function directly — pinning `missingToken`, the new `serviceUnavailable` (fetch rejects), and `invalidToken` (non-ok response) branches — verified to fail with the `try`/`catch` reverted and pass with it restored. Three-part change: spec (here) + code + test.

## 14. Implementation status

Done: the eleven routes in §5 (list + search, `/cases` SVAR grid,
`/board` SVAR Kanban, create, detail + delete + check-duplicates + links
panel + audit link, edit, `/[pid]/audit`, merge + recent-merges,
`/audit` recent activity); lean client + `CaseRepository` (CRUD,
check-duplicates, merge/recentMerges, listLinks/createLink/deleteLink,
search/audit/recentAudit/recentEvents — PRO-P15, 2026-08-29); form (case
type / status / priority dropdowns + identifiers editor); SPA config;
full BFF auth (session cookie → PASETO exchange → server-side proxy,
CSRF on mutating calls, per-app `/signin`+`/verify` magic-link — no
client-held token); 13-locale i18n throughout, including the merge,
links, search, and audit/activity blocks. `pnpm run check` clean (0/0);
vitest 70/70; Playwright 8/8 (unchanged — PRO-P15 added vitest coverage
only, no new e2e smoke); production build succeeds.

## 15. Roadmap

v0.1: CRUD + duplicate-check UI. v0.2 (done): tests. v0.3 (done): auth
(BFF + httpOnly cookie + CSRF, per
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
Since v0.3: merge UI, cross-service links panel, SVAR grid/board routes,
Lily pickers, `Custom(label)` editing (FE-5). Still open: a search box
once the service ships search; audit views; the catalogued-but-unrouted
SVAR calendar/gantt/filemanager seams (§7).

## 16. Open questions

- Real-time duplicate warning on the create form?
- Inline validation of agency / docket identifier formats?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
