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

In scope: the routes (`/`, `/cases`, `/board`, `/new`, `/merge`,
`/[pid]`, `/[pid]/edit`, `/signin`, `/verify`), the
API client, the case form, and a BFF + httpOnly-cookie session (§6.7/§6.8,
per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
Out of scope: full-text search UI, audit views.

## 3. Stakeholders and users

Caseworkers and case administrators across governmental agencies.

## 4. Glossary

- **pid** — the case's public id (route param).
- **Case** — the `case_matcher::Case` payload.
- **check-duplicates** — POST the current record to find stored matches.

## 5. Information architecture

```
/            list of cases
/cases       SVAR DataGrid + FilterBar index (client-side filtering)
/board       SVAR Kanban board, one column per status; drag-to-change-status
/new         create form
/[pid]       detail + delete + check-duplicates + "subject of this case" links panel
/[pid]/edit  edit form
/merge       merge a duplicate into a survivor + recent merge history
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

The privileged cross-case dump `GET /api/cases/links` (the aggregator's
reconciliation pull, gated as a destructive governed read) is deliberately
**not** consumed here — an operator UI has no use for every `subject_of`
edge in the service at once.

## 10. Persistence

None client-side beyond in-memory route state.

## 11. Testing strategy

`pnpm run check` (svelte-check strict, 0/0). **vitest** unit tests
(`tests/unit/`, 61 tests across 7 files) cover: `client.test.ts` (the
`ApiClient` — verb/body/headers, per-call `token` override, `getPage()`
pagination-header parsing, error-classification/empty-body);
`cases.test.ts` (`CaseRepository` — every method's path + verb, incl. a
regression pinning `check-duplicates`, and the merge/links methods);
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
- [ ] `Custom(label)` editing for case type / status / schemes.
- [ ] Search box once the service ships search.
- [x] Auth — adopt BFF + httpOnly cookie + CSRF; remove
  `mxi_access_token`/`localStorage` bearer + fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).

## 14. Implementation status

Done: the eight routes in §5 (list, `/cases` SVAR grid, `/board` SVAR
Kanban, create, detail + delete + check-duplicates + links panel, edit,
merge + recent-merges); lean client + `CaseRepository` (CRUD,
check-duplicates, merge/recentMerges, listLinks/createLink/deleteLink);
form (case type / status / priority dropdowns + identifiers editor); SPA
config; full BFF auth (session cookie → PASETO exchange → server-side
proxy, CSRF on mutating calls, per-app `/signin`+`/verify` magic-link —
no client-held token); 13-locale i18n throughout, including the merge
and links blocks. `pnpm run check` clean (0/0); vitest 61/61; Playwright
8/8; production build succeeds.

## 15. Roadmap

v0.1: CRUD + duplicate-check UI. v0.2 (done): tests. v0.3 (done): auth
(BFF + httpOnly cookie + CSRF, per
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
Since v0.3: merge UI, cross-service links panel, SVAR grid/board routes,
Lily pickers. Still open: a search box once the service ships search;
audit views; `Custom(label)` editing; the catalogued-but-unrouted SVAR
calendar/gantt/filemanager seams (§7).

## 16. Open questions

- Real-time duplicate warning on the create form?
- Inline validation of agency / docket identifier formats?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
