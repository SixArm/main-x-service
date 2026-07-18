# Portfolio Front-End — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test. Live work queue is §13.
>
> Sibling service: [project-portfolio-management-service-with-loco](../../project-portfolio-management-service-with-loco/spec/index.md).
> Entity umbrella: [portfolio/spec](../../spec/index.md).

## 1. Purpose and vision

A SvelteKit SPA for portfolio managers, programme leads, and project
operators to register, browse, edit, and duplicate-check **work-item**
identities across **four matchable collections — portfolios, projects,
products, programs** — **and** to run each work item as a live project
workspace: goals, a Kanban task board, issues, a timeline / Gantt, and a
burndown chart. It is a thin presentation layer over the portfolio
service's REST API (`/api/{portfolios,projects,products,programs}/...`);
the Rust service is the system of record.

The entity has two faces that share one record (entity spec §1): a
**matchable identity registry** (portfolio-level dedup across the four
collections) and a **project-management tool** (operational
sub-resources). The front-end surfaces both — identity CRUD +
duplicate-check + merge on one side, the project-management views on the
other. The canonical matchable Rust type is **`WorkItem`** with a required
`kind: WorkItemKind` discriminator; a Portfolio is the umbrella kind of
work item, and Project / Product / Program sit under a portfolio (they
carry a `portfolio_ref` to their parent). The four kinds are **distinct
collections / tables**, not types of one collection, so a project never
matches a product (the matcher's `kind` gate enforces this).

## 2. Scope

In scope:

- The **identity routes** for **each of the four collections**
  (`/portfolios`, `/projects`, `/products`, `/programs`, with
  `/{collection}/new`, `/{collection}/[pid]`,
  `/{collection}/[pid]/edit`): the work-item list (SVAR DataGrid),
  create/edit form, detail page. A **collection switcher** in the chrome
  moves between the four list views.
- The **API client** (`src/lib/api/{types,client,work-items}.ts`), the
  work-item form, a name-search box on each list, a duplicate-check /
  match screen with a per-component **MatchBreakdown** visual, a
  merge-duplicate action on the detail page, and a per-work-item audit
  timeline.
- The **portfolio roll-up**: a Portfolio detail page also lists its child
  projects / products / programs (those whose `portfolio_ref` is this
  portfolio's pid).
- The **project-management views** on the detail page (or its
  sub-routes) for any work item: a **Kanban task board** (drag = status
  change), an **Issues** list, a **Gantt / timeline** view, a
  **burndown** chart, and a **Goals** panel.
- The **layout shell**: a top navigation bar with a leftmost hamburger
  menu, full-width content, a full theme catalogue selector, and a
  13-locale i18n selector (RTL for `ar` / `ur`).

Out of scope (stated): no FHIR surface (the service exposes none); no
consent-management UI; no finance / budgeting UI; **no posts / comments
feed, no member-role panel** (the portfolio entity carries only
tasks / goals / issues as sub-resources — see entity spec §5; posts /
comments / members are roadmap-only). Authentication is **not**
implemented here as a login screen — sign-on is delegated to the central
[authentication-service](../../../authentication/authentication-service-with-loco/)
magic-link SSO. Per the BFF + httpOnly-cookie model
([`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md))
the browser holds only the `__Host-mxi_session` cookie and the SvelteKit
server attaches a short-lived PASETO server-side — no token in JS, no
`localStorage`.

## 3. Stakeholders and users

Portfolio managers and PMO analysts (dedup, roll-up across the four
collections), programme / project leads (the workspace), team
contributors (tasks, issues, goals), and auditors (the audit timeline +
match breakdown).

## 4. Glossary

- **pid** — the work item's public id (route param).
- **WorkItem** — the `project_portfolio_management_matcher::WorkItem` payload (the matchable
  identity header; the API DTO = the matcher type, persisted as JSONB).
  Required `kind` (Portfolio / Project / Product / Program) selects the
  collection / table it lives in.
- **Collection** — one of the four matchable record types: `portfolios`,
  `projects`, `products`, `programs`. Each is its own REST collection /
  list / CRUD; matching is **within a collection only**.
- **portfolio_ref** — the parent portfolio `pid` carried by a Project /
  Product / Program; absent for the Portfolio kind. Drives the roll-up
  and is an exact-match supporting signal for child kinds.
- **Sub-resource** — a `Goal` / `Task` / `Issue` owned by a work item,
  reached under `/api/{collection}/{pid}/…`. Not part of the matching
  surface (except goal titles).
- **check-duplicates** — POST the current record to find stored matches
  (within its collection).
- **MatchBreakdown** — the per-component score map returned by a match
  (name, goals, code, owner org, portfolio, timeframe, keywords,
  relationships, tags).
- **EntityRef** — an opaque `*_ref` into person / worker / auth-user /
  organization, stored verbatim and resolved by the front-end picker.
- **Derived view** — a read-only computed view (timeline, burndown),
  never canonical state.

## 5. Information architecture

```
/                            collection switcher → defaults to /portfolios
/{collection}                list (SVAR DataGrid) + search + recent activity
/{collection}/new            create form
/{collection}/[pid]          detail + delete + check-duplicates + merge + audit
                             (Portfolio detail also rolls up child work items)
/{collection}/[pid]/edit     edit form
/{collection}/[pid]/board    Kanban task board (Todo/InProgress/InReview/Done/Blocked)
/{collection}/[pid]/issues   issues list (kind / severity / status)
/{collection}/[pid]/timeline Gantt / timeline (goal milestones + task date ranges)
/{collection}/[pid]/burndown burndown chart (remaining estimate over time)
/{collection}/[pid]/goals    goals panel
```

where `{collection} ∈ { portfolios, projects, products, programs }`.

(The collection switcher MAY be a top-bar control rendering four list
views, or four sibling routes; the project-management views MAY be
implemented as detail-page tabs rather than discrete sub-routes. The
spec fixes the *capabilities*, not the URL shape. If sub-routes are used
they share the `[pid]` layout.)

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app, applied
here:

- Global navigation MUST be a **top navigation bar** (header) spanning
  the full viewport width. There MUST NOT be a left-hand navigation
  sidebar / rail.
- The navigation toggle is a **hamburger menu** placed **leftmost** in
  the top bar; on narrow viewports the top-bar navigation collapses
  behind it. The hamburger is present (leftmost) regardless of viewport.
- The main content area MUST be **full-width** — never inset by a
  persistent side-navigation column.
- A chrome utility area in the top bar carries the **collection
  switcher** (portfolios / projects / products / programs), the **theme
  selector** (`lily-design-system-svelte-theme-select`) and the **locale
  selector** (`lily-design-system-svelte-locale-select`), plus the
  session affordance (Sign in / Sign out).

### Theming

The app uses the **full shared Lily/DaisyUI theme catalogue** for
parity with the rest of the family. Selecting a theme via
`ThemeSelect` changes the whole site look: it manages exactly one
`<link rel="stylesheet" data-lily-theme-select="theme">` in
`document.head`, mutating its `href` and the `data-theme` attribute on
`<html>`. The choice persists to `localStorage` (key `portfolio:theme`).
Theme stylesheets are served from `static/assets/themes/` (a symlink
to the shared design-system themes).

### Locale / i18n

`LocaleSelect` (`lily-design-system-svelte-locale-select`) sets `lang`
and `dir` on `<html>` and switches the active translation catalogue, so
selecting a locale changes the displayed language. Supported locales
(13): `en`, `cy`, `es`, `fr`, `de`, `ar`, `ru`, `hi`, `zh`, `bn`, `pt`,
`id`, `ur`. `ar` and `ur` are **RTL** (`dir="rtl"`); the rest are LTR.
The choice persists to `localStorage` (key `portfolio:locale`). UI
strings come from a per-locale catalogue under `$lib/i18n/`; missing
keys fall back to `en`.

## 6. Functional requirements

1. **List** active work items for the selected collection
   (`GET /api/{collection}`) in a **SVAR DataGrid** with columns:
   name, status, owner org, lead, `portfolio_ref` (child kinds only),
   target date, tags. Sortable; client-side filter/search.
   - Search box (search-on-submit): a non-blank query calls
     `GET /api/{collection}/search?q=` (URL-encoded) and renders the
     filtered results; **Clear** (or an empty query) restores the full
     list. Loading and empty-result states are shown.
   - Recent activity: a "Show recent activity" toggle lazy-loads
     `GET /api/{collection}/events/recent` on first open and renders
     the events newest-first (highest `seq` first): the kind
     (created/updated/deleted/merged), the name (linked to the work item
     by pid), and the `seq`. Loading, empty, and error states; the panel
     does not auto-load on mount.
2. **Create** (`POST /api/{collection}`), redirect to the new detail
   page. The collection (kind) is fixed by the route.
3. **Detail**: render the stored `WorkItem`; offer edit, delete,
   check-duplicates, merge, the audit timeline, and entry points to the
   project-management views. A **Portfolio** detail page additionally
   rolls up its **child work items** — the projects / products / programs
   whose `portfolio_ref` equals this portfolio's pid — as linked lists.
4. **Edit** (`PUT`), redirect back to detail.
5. **Delete** (`DELETE`, soft), redirect to the collection list.
6. **Check-duplicates** posts the current record and lists matches
   (name, score, confidence) **within the same collection**, excluding
   the record itself, each with a visual **MatchBreakdown** (per-component
   bars for name / goals / code / owner org / portfolio / timeframe /
   keywords / relationships / tags). There is no `plan_type` component —
   `kind` is a hard match gate, not a scored component.
7. **Merge**: each duplicate row offers "Merge into this record" (the
   detail record is the survivor/main; the row's pid is the duplicate).
   A two-step inline confirm calls `POST /api/{collection}/merge` with
   `{main_pid, duplicate_pid, reason?}`. On success it adopts the
   returned survivor record, re-runs check-duplicates, and shows a
   success message. Equal pids are guarded client-side (the service
   `422`s); `404`/other errors surface via the error banner.
8. **Audit timeline**: a "Show audit trail" toggle lazy-loads
   `GET /api/{collection}/{pid}/audit` on first open and renders the
   rows newest-first (action, actor or "—" when null, timestamp).
   Loading, empty, and error states; the panel does not auto-load on
   mount.
9. **The work-item form** (create/edit) edits: `name` (required),
   `alternate_names`, `code` (owner-scoped), `owner_org_id` (**org
   picker** into the organization entity), `owner_org_name`, `lead_ref`
   (**person / worker picker**), `portfolio_ref` (**portfolio picker**;
   shown only for child kinds — Project / Product / Program), `status`, a
   **goals editor** (title + description + status + target date rows),
   `start_date` / `target_date`, `keywords`, `tags`, `identifiers`
   (scheme + value rows), `same_as`, `in_language`, and `relationships`
   (kind + target rows). The `kind` is fixed by the collection route (not
   an editable field). Comma-list fields split on submit; blanks null;
   empty repeatable rows dropped.
10. **Kanban task board** (`/{collection}/[pid]/board`): columns **Todo /
    InProgress / InReview / Done / Blocked**; cards show task title,
    assignee, estimate, due date. **Drag a card to a column = status
    change** (`PATCH …/{pid}/tasks/{tid}`). Create / edit task inline.
11. **Issues list** (`/{collection}/[pid]/issues`): table of issues with
    kind (Bug / Risk / Blocker / Question / Improvement), severity
    (Low / Med / High / Critical), status (Open / InProgress / Resolved /
    Closed), reporter, assignee; create / edit; filter by status /
    severity.
12. **Gantt / timeline** (`/{collection}/[pid]/timeline`): renders
    `GET …/{pid}/timeline` — goal milestones (target dates) + task date
    ranges (start / due) as Gantt-shaped rows over the work-item
    timeframe.
13. **Burndown chart** (`/{collection}/[pid]/burndown`): renders
    `GET …/{pid}/burndown` — remaining estimate over time as a series.
14. **Goals panel** (`/{collection}/[pid]/goals`): list / create / edit /
    delete goals (title, description, status, target date). Goal titles
    feed the match `Goals` component (display-only note) via the
    `data.goals[]` bridge.
15. **Session / auth (BFF + httpOnly cookie)**: the top bar carries a
    session affordance. The primary path is **Sign in**, routed through
    the BFF to the central authentication front-end for the passwordless
    magic-link; on success the authentication-service sets the
    `__Host-mxi_session` httpOnly cookie. The browser holds only that
    cookie — **no token in JS, no `localStorage`, no URL-fragment
    handoff**. The SvelteKit **server** (BFF) holds the session and
    attaches a short-lived PASETO server-side when calling the portfolio
    service; the browser never calls the service directly. Mutating
    browser→BFF calls carry a CSRF token; **Sign out** revokes the
    session. This lets operator traffic through once the service turns on
    blanket enforcement (`PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`, off by default). Per
    [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
16. **Layout shell**: global navigation is a full-width **top bar**
    (header) with a **leftmost hamburger** toggle — NOT a left sidebar —
    the main content area is **full-width**, and the chrome area carries
    the collection switcher and the theme + locale selectors.

## 7. Non-functional requirements

- **Svelte 5 runes only** (`$state` / `$derived` / `$effect` / `$props`
  / `$bindable`); no `export let`, no `$:`, events are callback props.
- **SvelteKit 2**, SPA (`ssr = false` / `prerender = false`).
- **TypeScript strict** with `noUncheckedIndexedAccess`; no `any`
  without a justifying comment.
- **SVAR Svelte DataGrid** for each collection list and any tabular
  sub-view; native HTML for simple lists.
- **Lily Design System Svelte Headless** for accessibility primitives
  (focus trap, listbox, combobox, dialog) and the theme / locale
  selectors; native HTML elsewhere.
- **No global stores** for HTTP state — construct a `WorkItemRepository`
  (bound to a collection) per page/component.
- Drift accepted: own copy of API client / types / form primitives; no
  shared package (repo decision 2026-06-02).

## 8. Architecture

`ApiClient` (lean, raw-JSON, get/post/put/patch/delete + `ApiError`) →
`WorkItemRepository` (constructed for a given collection) → routes. The
service is loco.rs and returns **raw JSON** (no envelope). Under the BFF
model (§6.15) the browser carries only the `__Host-mxi_session` cookie
and the SvelteKit server attaches the short-lived PASETO server-side when
calling the service; no token is read or attached in browser JS.
`WorkItemForm` builds a `WorkItem` from the inputs (comma lists split,
blanks nulled, goals / identifiers / relationships as editable rows;
empty rows dropped on submit; a seeded `Custom` enum collapses to "—";
`kind` injected from the route, `portfolio_ref` shown only for child
kinds). The project-management views call the sub-resource endpoints via
the same repository. Layout chrome (collection switcher, theme / locale
selectors, hamburger nav, session affordance) lives in `+layout.svelte`;
`+layout.ts` sets the SPA toggles.

`types.ts` hand-mirrors `project_portfolio_management_matcher::WorkItem` and the
sub-resource / DTO shapes: `WorkItem`, `WorkItemKind`, `WorkItemStatus`,
`Goal`, `GoalStatus`, `Task`, `TaskStatus`, `Issue`, `IssueKind`,
`IssueSeverity`, `IssueStatus`, `Relationship`, `RelationKind`,
`WorkItemIdentifier`, `IdentifierScheme`, `WorkItemRef`, `ScoredRef`,
`MatchBreakdown`, `MergeResult`, `AuditEntry`, `WorkItemEvent`,
`TimelineRow`, `BurndownPoint`. It MUST be updated in the same change
cycle as any matcher-type change (entity spec §18).

## 9. API consumption

`{collection} ∈ { portfolios, projects, products, programs }`.

| Route / action | Endpoint |
|---|---|
| list | `GET /api/{collection}` |
| search | `GET /api/{collection}/search?q=` |
| recent activity | `GET /api/{collection}/events/recent` (→ `WorkItemEvent[]`) |
| create | `POST /api/{collection}` |
| detail load | `GET /api/{collection}/{pid}` |
| delete | `DELETE /api/{collection}/{pid}` |
| duplicates | `POST /api/{collection}/check-duplicates` (→ `ScoredRef[]` w/ `MatchBreakdown`) |
| merge | `POST /api/{collection}/merge` (`{main_pid, duplicate_pid, reason?}`) |
| audit | `GET /api/{collection}/{pid}/audit` (→ `AuditEntry[]`) |
| edit | `PUT /api/{collection}/{pid}` |
| roll-up (portfolio) | `GET /api/{projects,products,programs}?portfolio_ref={pid}` |
| board: list / move | `GET …/{pid}/tasks` · `PATCH …/{pid}/tasks/{tid}` (status) |
| board: create / edit | `POST …/{pid}/tasks` · `PUT …/{pid}/tasks/{tid}` |
| issues | `GET / POST …/{pid}/issues` · `PUT …/{pid}/issues/{iid}` |
| goals | `GET / POST …/{pid}/goals` · `PUT / DELETE …/{pid}/goals/{gid}` |
| timeline | `GET …/{pid}/timeline` (→ `TimelineRow[]`) |
| burndown | `GET …/{pid}/burndown` (→ `BurndownPoint[]`) |

(Sub-resource verbs/paths track the service spec §9; pin them in tests
when the controllers land.)

## 10. Persistence

None client-side beyond in-memory route state and two app-local
`localStorage` keys for chrome preferences (`portfolio:theme` /
`portfolio:locale`). The session lives only in the `__Host-mxi_session`
httpOnly cookie — no auth token in `localStorage` (BFF model, §6.15).

## 11. Testing strategy

`pnpm run check` (svelte-check strict, 0/0). **vitest** unit tests
(`tests/unit/`) cover:

- the `ApiClient` (verb / body / headers / cookie-credentials / CSRF /
  error-classification / empty-body, incl. `PATCH`);
- the **BFF auth integration** (server-side session→PASETO exchange and
  CSRF on mutating browser→BFF calls per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md);
  no client-held token to test);
- `WorkItemRepository` **per collection** (every method's path + verb,
  incl. `check-duplicates`, `search()` `/search?q=` URL-encoding,
  `merge()` body `{main_pid, duplicate_pid, reason?}` with `404` / `422`
  `ApiError` propagation, `audit()`, `recentEvents()`, the portfolio
  roll-up query, and the sub-resource methods — task move `PATCH`,
  issues, goals, timeline, burndown);
- the **`WorkItemForm`** component (required-name guard blocks `onsubmit`
  on blank/whitespace; `build()` trims scalars, nulls blanks, splits
  comma lists, drops empty goal / identifier / relationship rows,
  collapses a `Custom` enum seed, injects `kind` from the route, shows
  `portfolio_ref` only for child kinds);
- the **i18n catalogue** (every key present in `en`; fallback to `en`
  for a missing key; RTL flag for `ar` / `ur`);
- a **`+layout` render test** asserting the **hamburger toggles the
  nav** (collapsed → expanded), and that the collection switcher and the
  theme + locale selectors render.

Component tests run via `@testing-library/svelte` mounted client-side
by the `svelteTesting()` vite plugin.

**Playwright** smoke tests (`tests/e2e/`) with the API stubbed via
`page.route`, run against the production build (`vite preview`) to
avoid the `vite dev` cold-start module race: load the identity routes
for a collection; the list search box (matching keeps the row,
non-matching shows the empty message); the detail-page merge action
(check-duplicates → confirm merge → success, asserting the merge
endpoint fired); the audit timeline; the recent-activity panel; the
portfolio roll-up of child work items; the Kanban board (drag a card →
status PATCH fired); the issues list; the timeline / burndown views
render; and the layout (hamburger toggles nav; collection switcher
changes the list; theme + locale selectors switch `data-theme` / `lang`
+ `dir`, incl. RTL for `ar`).

Run: `pnpm test` (vitest) and `pnpm test:e2e` (Playwright).

## 12. Compliance

Work items are business / portfolio artefacts; defer to the service's
controls for access / audit. Lead / assignee / author refs are opaque
`EntityRef`s into person / worker — the front-end displays them but does
not resolve PII beyond what the referenced services return. Cross-service
links are **never** a match signal (entity spec §1).

## 13. Tasks (live work queue)

> Spec-only at v0.1.0 — no code yet. This is the build queue; check off
> in three-part PRs (spec + code + test).

- [ ] Scaffold the SvelteKit 2 / Svelte 5 (runes) SPA: `package.json`,
  `svelte.config.js`, `vite.config.ts`, `tsconfig` (strict +
  `noUncheckedIndexedAccess`), `src/app.html`, `src/app.css`,
  `.env.example` (`PUBLIC_API_BASE_URL`, `VITE_AUTH_FRONTEND_URL`).
- [ ] `src/lib/api/types.ts` — mirror `project_portfolio_management_matcher::WorkItem` +
  `WorkItemKind` + sub-resource + DTO shapes (per §8).
- [ ] `src/lib/api/client.ts` — lean fetch wrapper (get/post/put/patch/
  delete) + `ApiError`. Browser→BFF calls only; the SvelteKit server
  attaches the PASETO bearer server-side (no client-held token).
- [ ] `src/lib/api/work-items.ts` — `WorkItemRepository` (collection-bound:
  CRUD + search + checkDuplicates + merge + audit + recentEvents +
  portfolio roll-up + sub-resource methods + timeline + burndown).
- [ ] Auth — adopt BFF + httpOnly cookie + CSRF: `hooks.server.ts` /
  server routes read the `__Host-mxi_session` cookie, exchange it for a
  short-lived PASETO server-side, and attach it when calling the
  portfolio service; CSRF on mutating browser→BFF calls. No
  `mxi_access_token` / `localStorage` bearer, no fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
- [ ] `src/lib/config.ts` — `PUBLIC_API_BASE_URL` + `VITE_AUTH_FRONTEND_URL`
  + `signInUrl()` (BFF sign-in redirect).
- [ ] `src/lib/i18n/` — 13-locale catalogues (en, cy, es, fr, de, ar,
  ru, hi, zh, bn, pt, id, ur) + RTL flags + `en` fallback.
- [ ] `+layout.svelte` / `+layout.ts` — top-bar nav with **leftmost
  hamburger**, full-width content, collection switcher + theme + locale
  selectors, session affordance (Sign in / Sign out — BFF, no token
  paste); SPA toggles.
- [ ] Identity routes for the four collections: `/{collection}` (SVAR
  DataGrid list + search + recent activity), `/{collection}/new`,
  `/{collection}/[pid]` (detail + delete + check-duplicates +
  MatchBreakdown + merge + audit timeline; portfolio roll-up of child
  work items), `/{collection}/[pid]/edit`.
- [ ] `WorkItemForm` component (incl. org picker, person/worker lead
  picker, portfolio picker for child kinds, goals / identifiers /
  relationships editors; `kind` from route).
- [ ] `MatchBreakdown` visual component (per-component score bars incl.
  goals / portfolio / relationships / tags; no `plan_type`).
- [ ] Kanban task board (`/{collection}/[pid]/board`) — drag = status
  PATCH.
- [ ] Issues list (`/{collection}/[pid]/issues`).
- [ ] Gantt / timeline view (`/{collection}/[pid]/timeline`).
- [ ] Burndown chart (`/{collection}/[pid]/burndown`).
- [ ] Goals panel (`/{collection}/[pid]/goals`).
- [ ] vitest unit suite (client, repository per collection, BFF auth
  integration, signInUrl, WorkItemForm, i18n, `+layout`
  hamburger-toggle render test).
- [ ] Playwright e2e smoke (identity routes, search, merge, audit,
  recent activity, portfolio roll-up, board drag, issues,
  timeline/burndown, layout hamburger + collection switcher +
  theme/locale switch incl. RTL).

## 14. Implementation status

**Spec-only; no code yet.** This document and the doc-set
(`README.md`, `CLAUDE.md`, `AGENTS.md`, `CHANGELOG.md`, `index.md`)
are the inaugural v0.1.0 deliverable. The §13 queue is the build plan;
nothing under `src/` exists. Scaffolding (copy-adapt from a sibling
`*-front-end-with-svelte`) is the first task.

## 15. Roadmap

- **v0.1** (here): spec + docs only.
- **v0.2**: scaffold + identity routes for the four collections (list /
  create / detail / edit) + collection switcher + lean client +
  repository + BFF auth (httpOnly cookie + CSRF, per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md))
  + SSO sign-in + layout shell (top-bar hamburger, theme + locale
  selectors) + vitest + Playwright smoke.
- **v0.3**: duplicate-check + MatchBreakdown visual + merge + audit
  timeline + portfolio roll-up of child work items.
- **v0.4**: project-management views — Kanban board, issues, timeline /
  Gantt, burndown, goals.
- **v0.5**: full 13-locale translation catalogues + RTL polish.
- **Deferred / roadmap-only**: posts feed + threaded comments, member /
  role panel (not part of the v1 portfolio sub-resource set).

## 16. Open questions

- Collection switcher as a top-bar control over four list views, or four
  discrete sibling routes?
- Project-management views as detail-page tabs or discrete sub-routes?
- Real-time duplicate warning on the create form?
- Which charting primitive for the Gantt / burndown (SVAR vs. native
  SVG vs. a Lily chart helper)?
- How rich should the org / person / worker / portfolio pickers be
  (typeahead against the sibling services vs. raw `pid` entry) for MVP?

## 17. References

- Sibling service spec: [project-portfolio-management-service-with-loco/spec](../../project-portfolio-management-service-with-loco/spec/index.md).
- Entity umbrella: [portfolio/spec](../../spec/index.md);
  [models](../../AGENTS/models.md); [matching](../../AGENTS/matching.md).
- Family layout shell + theming + locale conventions; SvelteKit 2 +
  Svelte 5 runes docs; SVAR Svelte DataGrid; Lily Design System Svelte
  Headless.
- Auth / sessions (source of truth): [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md);
  blanket enforcement: [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md);
  locales: [`agents/share/locales.md`](../../../agents/share/locales.md).

## 18. Change control

Update this spec with any behavioural change; a behavioural change is
one PR with three parts (spec + code + test). Bump `CHANGELOG.md`. Keep
`src/lib/api/types.ts` in lockstep with the matcher `WorkItem` type — if
a field changes in the service / matcher, fix it here in the same cycle.
