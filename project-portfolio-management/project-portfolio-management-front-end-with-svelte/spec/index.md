# Portfolio Front-End — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test. Live work queue is §13.
>
> Sibling service: [project-portfolio-management-service-with-loco](../../project-portfolio-management-service-with-loco/spec/index.md).
> Entity umbrella: [portfolio/spec](../../spec/index.md).

## 1. Purpose and vision

A SvelteKit SPA for portfolio managers, programme leads, and project
operators to register, browse, edit, and duplicate-check **plan**
identities in **one recursive collection** — **and** to run each plan as a
live project workspace: goals, a Kanban task board, issues, a timeline /
Gantt, and a burndown chart. It is a thin presentation layer over the
portfolio service's REST API (`/api/plans/...`); the Rust service is the
system of record.

The entity has two faces that share one record (entity spec §1): a
**matchable identity registry** (plan-level dedup across the whole
collection) and a **project-management tool** (operational
sub-resources). The front-end surfaces both — identity CRUD +
duplicate-check + merge on one side, the project-management views on the
other. The canonical matchable Rust type is **`Plan`** with an
**optional** `kind: PlanKind` descriptive label; a plan may contain any
other plan (it carries an optional `parent_ref` to its parent), so the
tree is recursive. Every record is a plan in **one collection / table**,
and matching runs across the whole collection — it is **not** gated by
kind.

## 2. Scope

In scope:

- The **identity routes** under the static `plans/` directory
  (`/plans`, with `/plans/new`, `/plans/[pid]`, `/plans/[pid]/edit`):
  the plan list (SVAR DataGrid), create/edit form, detail page. The nav
  has a single **Plans** destination (no collection switcher).
- The **API client** (`src/lib/api/{types,client,plans}.ts`), the
  plan form, a name-search box on the list, a duplicate-check /
  match screen with a per-component **MatchBreakdown** visual, a
  merge-duplicate action on the detail page, and a per-plan audit
  timeline.
- The **child roll-up**: a plan detail page also lists its child plans
  (those whose `parent_ref` is this plan's pid).
- The **project-management views** on the detail page (or its
  sub-routes) for any plan: a **Kanban task board** (drag = status
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

Portfolio managers and PMO analysts (dedup, roll-up across the plan
tree), programme / project leads (the workspace), team
contributors (tasks, issues, goals), and auditors (the audit timeline +
match breakdown).

## 4. Glossary

- **pid** — the plan's public id (route param).
- **Plan** — the `project_portfolio_management_matcher::Plan` payload (the
  matchable identity header; the API DTO = the matcher type, persisted as
  JSONB). Carries an **optional** `kind` (Portfolio / Project / Product /
  Program / Practice / Process / Purpose / Pathway / Proposal)
  descriptive label; every plan lives in one collection / table.
- **Collection** — the single recursive `/api/plans` collection holding
  every plan. One REST collection / list / CRUD; matching runs across the
  whole collection.
- **kind** — an **optional** descriptive label on a plan (Portfolio /
  Project / Product / Program / Practice / Process / Purpose /
  Pathway / Proposal). It is display metadata; matching is **not**
  gated by kind.
- **parent_ref** — the containing plan's `pid` carried by any plan
  (optional; any plan may contain any other). Drives the roll-up and is an
  exact-match supporting signal.
- **Sub-resource** — a `Goal` / `Task` / `Issue` owned by a plan,
  reached under `/api/plans/{pid}/…`. Not part of the matching
  surface (except goal titles).
- **check-duplicates** — POST the current record to find stored matches
  (across the whole collection).
- **MatchBreakdown** — the per-component score map returned by a match
  (name, goals, code, owner org, parent, timeframe, keywords,
  relationships, tags).
- **EntityRef** — an opaque `*_ref` into person / worker / auth-user /
  organization, stored verbatim and resolved by the front-end picker.
- **Derived view** — a read-only computed view (timeline, burndown),
  never canonical state.

## 5. Information architecture

```
/                            landing → links to /plans
/dashboard                   PPM dashboard (site tiles + RAG / stage rollups)
/plans                       list (SVAR DataGrid + FilterBar)
/plans/new                   create form
/plans/[pid]                 detail + delete + check-duplicates
/plans/[pid]/edit            edit form
/plans/[pid]/governance      governance panel (gates, risks, budget,
                             benefits + ROI, OKR mappings, milestones,
                             allocations)
/plans/[pid]/schedule        plan schedule (child timeframes)
/plans/[pid]/board           per-plan task Kanban + sprints +
                             honest burndown + standup digest
/gantt                       schedule Gantt (SVAR; dependency links +
                             critical path; read-only)
/capacity                    resource capacity rollup
/engineering                 blocked aging + MoSCoW + delivery links
/calendar                    estate milestone calendar (kind filter)
/executive                   CEO area: portfolio-health briefing,
                             decision log, benefits realization
/financials                  CFO area: budget variance + per-currency
                             exposure (server-derived; no FX)
/technology                  CTO area: technology radar + dependency
                             risk lens + tech-debt register + flow
                             metrics
/ideas                       idea board
/objectives                  OKR objectives + alignment rollups
/proposals                   work-intake proposal pipeline
/reports                     saved reports + CSV download
/scenarios                   scenario planning + side-by-side compare
/board                       board pack + investments + snapshot trends
/auditor                     audit explorer + SoD findings + evidence pack
/compliance                  compliance register + conformance findings
/risk                        CRO heatmap + posture + appetite
/security                    CISO register + late-stage heuristic
/regulator                   coarse regulator extract (mask-aware)
/signin · /verify            BFF magic-link sign-in / verification

Roadmap (per-plan project-management sub-routes, §13/§15;
the board landed 2026-07-20 and is listed above):
/plans/[pid]/issues          issues list (kind / severity / status)
/plans/[pid]/timeline        Gantt / timeline (goal milestones + task date ranges)
/plans/[pid]/burndown        burndown chart (remaining estimate over time)
/plans/[pid]/goals           goals panel
```

(The project-management views MAY be implemented as detail-page tabs
rather than discrete sub-routes. The spec fixes the *capabilities*, not
the URL shape. If sub-routes are used they share the `[pid]` layout.)

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
- The nav has a single **Plans** destination (no collection switcher). A
  chrome utility area in the top bar carries the **theme selector**
  (`lily-design-system-svelte-theme-picker`) and the **locale selector**
  (`lily-design-system-svelte-locale-picker`), plus the session affordance
  (Sign in / Sign out).

### Theming

The app uses the **full shared Lily/DaisyUI theme catalogue** for
parity with the rest of the family. Selecting a theme via
`ThemePicker` changes the whole site look: it manages exactly one
`<link rel="stylesheet" data-lily-theme-picker="theme">` in
`document.head`, mutating its `href` and the `data-theme` attribute on
`<html>`. The choice persists to `localStorage` (key `portfolio:theme`).
Theme stylesheets are served from `static/assets/themes/` (a symlink
to the shared design-system themes).

### Locale / i18n

`LocalePicker` (`lily-design-system-svelte-locale-picker`) sets `lang`
and `dir` on `<html>` and switches the active translation catalogue, so
selecting a locale changes the displayed language. Supported locales
(13): `en`, `cy`, `es`, `fr`, `de`, `ar`, `ru`, `hi`, `zh`, `bn`, `pt`,
`id`, `ur`. `ar` and `ur` are **RTL** (`dir="rtl"`); the rest are LTR.
The choice persists to `localStorage` (key `portfolio:locale`). UI
strings come from a per-locale catalogue under `$lib/i18n/`; missing
keys fall back to `en`.

## 6. Functional requirements

1. **List** active plans (`GET /api/plans`) in a **SVAR DataGrid** with
   columns: name, kind, status, owner org, lead, `parent_ref`, target
   date, tags. Sortable; client-side filter/search.
   - Search box (search-on-submit): a non-blank query calls
     `GET /api/plans/search?q=` (URL-encoded) and renders the
     filtered results; **Clear** (or an empty query) restores the full
     list. Loading and empty-result states are shown.
   - Recent activity: a "Show recent activity" toggle lazy-loads
     `GET /api/plans/events/recent` on first open and renders
     the events newest-first (highest `seq` first): the kind
     (created/updated/deleted/merged), the name (linked to the plan
     by pid), and the `seq`. Loading, empty, and error states; the panel
     does not auto-load on mount.
2. **Create** (`POST /api/plans`), redirect to the new detail page. The
   `kind` label is chosen on the form (optional).
3. **Detail**: render the stored `Plan`; offer edit, delete,
   check-duplicates, merge, the audit timeline, and entry points to the
   project-management views. A plan detail page additionally rolls up its
   **child plans** — those whose `parent_ref` equals this plan's pid — as
   linked lists.
4. **Edit** (`PUT`), redirect back to detail.
5. **Delete** (`DELETE`, soft), redirect to the plans list.
6. **Check-duplicates** posts the current record and lists matches
   (name, score, confidence) **across the whole collection**, excluding
   the record itself, each with a visual **MatchBreakdown** (per-component
   bars for name / goals / code / owner org / parent / timeframe /
   keywords / relationships / tags). Matching is **not** gated by `kind`;
   there is no `plan_type` / kind component.
7. **Merge**: each duplicate row offers "Merge into this record" (the
   detail record is the survivor/main; the row's pid is the duplicate).
   A two-step inline confirm calls `POST /api/plans/merge` with
   `{main_pid, duplicate_pid, reason?}`. On success it adopts the
   returned survivor record, re-runs check-duplicates, and shows a
   success message. Equal pids are guarded client-side (the service
   `422`s); `404`/other errors surface via the error banner.
8. **Audit timeline**: a "Show audit trail" toggle lazy-loads
   `GET /api/plans/{pid}/audit` on first open and renders the
   rows newest-first (action, actor or "—" when null, timestamp).
   Loading, empty, and error states; the panel does not auto-load on
   mount.
9. **The plan form** (create/edit) edits: `name` (required),
   `alternate_names`, `code` (owner-scoped), `owner_org_id` (**org
   picker** into the organization entity), `owner_org_name`, `lead_ref`
   (**person / worker picker**), `parent_ref` (**plan picker**; optional
   for any plan), `kind` (**optional** select — Portfolio / Project /
   Product / Program / Practice / Process / Purpose / Pathway /
   Proposal, or none), `status`, a **goals editor** (title +
   description + status + target date rows), `start_date` / `target_date`,
   `keywords`, `tags`, `identifiers` (scheme + value rows), `same_as`,
   `in_language`, and `relationships` (kind + target rows). Comma-list
   fields split on submit; blanks null; empty repeatable rows dropped.
10. **Kanban task board** (`/plans/[pid]/board`): columns **Todo /
    InProgress / InReview / Done / Blocked**; cards show task title,
    assignee, estimate, due date. **Drag a card to a column = status
    change** (`PATCH /api/plans/{pid}/tasks/{tid}`). Create / edit task
    inline.
11. **Issues list** (`/plans/[pid]/issues`): table of issues with
    kind (Bug / Risk / Blocker / Question / Improvement), severity
    (Low / Med / High / Critical), status (Open / InProgress / Resolved /
    Closed), reporter, assignee; create / edit; filter by status /
    severity.
12. **Gantt / timeline** (`/plans/[pid]/timeline`): renders
    `GET /api/plans/{pid}/timeline` — goal milestones (target dates) +
    task date ranges (start / due) as Gantt-shaped rows over the plan
    timeframe.
13. **Burndown chart** (`/plans/[pid]/burndown`): renders
    `GET /api/plans/{pid}/burndown` — remaining estimate over time as a
    series.
14. **Goals panel** (`/plans/[pid]/goals`): list / create / edit /
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
    the main content area is **full-width**, the nav has a single **Plans**
    destination (no collection switcher), and the chrome area carries
    the theme + locale selectors.

## 7. Non-functional requirements

- **Svelte 5 runes only** (`$state` / `$derived` / `$effect` / `$props`
  / `$bindable`); no `export let`, no `$:`, events are callback props.
- **SvelteKit 2**, SPA (`ssr = false` / `prerender = false`).
- **TypeScript strict** with `noUncheckedIndexedAccess`; no `any`
  without a justifying comment.
- **SVAR Svelte DataGrid** for the plans list and any tabular
  sub-view; native HTML for simple lists.
- **Lily Design System Svelte Headless** for accessibility primitives
  (focus trap, listbox, combobox, dialog) and the theme / locale
  selectors; native HTML elsewhere.
- **No global stores** for HTTP state — construct a `PlanRepository`
  (all paths under `/api/plans`) per page/component.
- Drift accepted: own copy of API client / types / form primitives; no
  shared package (repo decision 2026-06-02).

## 8. Architecture

`ApiClient` (lean, raw-JSON, get/post/put/patch/delete + `ApiError`) →
`PlanRepository` (all paths under `/api/plans`) → routes. The
service is loco.rs and returns **raw JSON** (no envelope). Under the BFF
model (§6.15) the browser carries only the `__Host-mxi_session` cookie
and the SvelteKit server attaches the short-lived PASETO server-side when
calling the service; no token is read or attached in browser JS.
`PlanForm` builds a `Plan` from the inputs (comma lists split,
blanks nulled, goals / identifiers / relationships as editable rows;
empty rows dropped on submit; a seeded `Custom` enum collapses to "—";
`kind` is an optional select, `parent_ref` an optional plan picker). The
project-management views call the sub-resource endpoints via the same
repository. Layout chrome (Plans destination, theme / locale selectors,
hamburger nav, session affordance) lives in `+layout.svelte`;
`+layout.ts` sets the SPA toggles.

`types.ts` hand-mirrors `project_portfolio_management_matcher::Plan` and the
sub-resource / DTO shapes: `Plan`, `PlanKind`, `PlanStatus`,
`Goal`, `GoalStatus`, `Task`, `TaskStatus`, `Issue`, `IssueKind`,
`IssueSeverity`, `IssueStatus`, `Relationship`, `RelationKind`,
`PlanIdentifier`, `IdentifierScheme`, `PlanRef`, `ScoredRef`,
`MatchBreakdown`, `MergeResult`, `AuditEntry`, `PlanEvent`,
`TimelineRow`, `BurndownPoint`. It MUST be updated in the same change
cycle as any matcher-type change (entity spec §18).

## 9. API consumption

Every plan lives under `/api/plans` (one recursive collection).

| Route / action | Endpoint |
|---|---|
| list | `GET /api/plans` |
| search | `GET /api/plans/search?q=` |
| recent activity | `GET /api/plans/events/recent` (→ `PlanEvent[]`) |
| create | `POST /api/plans` |
| detail load | `GET /api/plans/{pid}` |
| delete | `DELETE /api/plans/{pid}` |
| duplicates | `POST /api/plans/check-duplicates` (→ `ScoredRef[]` w/ `MatchBreakdown`) |
| merge | `POST /api/plans/merge` (`{main_pid, duplicate_pid, reason?}`) |
| audit | `GET /api/plans/{pid}/audit` (→ `AuditEntry[]`) |
| edit | `PUT /api/plans/{pid}` |
| child roll-up | `GET /api/plans?parent={pid}` |
| schedule | `GET /api/plans/{pid}/schedule` |
| board: list / move | `GET /api/plans/{pid}/tasks` · `PATCH /api/plans/{pid}/tasks/{tid}` (status) |
| board: create / edit | `POST /api/plans/{pid}/tasks` · `PUT /api/plans/{pid}/tasks/{tid}` |
| issues | `GET / POST /api/plans/{pid}/issues` · `PUT /api/plans/{pid}/issues/{iid}` |
| goals | `GET / POST /api/plans/{pid}/goals` · `PUT / DELETE /api/plans/{pid}/goals/{gid}` |
| timeline | `GET /api/plans/{pid}/timeline` (→ `TimelineRow[]`) |
| burndown | `GET /api/plans/{pid}/burndown` (→ `BurndownPoint[]`) |

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
- `PlanRepository` (`tests/unit/plans.test.ts`; every method's path +
  verb, incl. `check-duplicates`, `search()` `/search?q=` URL-encoding,
  `merge()` body `{main_pid, duplicate_pid, reason?}` with `404` / `422`
  `ApiError` propagation, `audit()`, `recentEvents()`, the `?parent=`
  child roll-up query, and the sub-resource methods — task move `PATCH`,
  issues, goals, timeline, burndown);
- the **`PlanForm`** component (`tests/unit/plan-form.test.ts`;
  required-name guard blocks `onsubmit` on blank/whitespace; `build()`
  trims scalars, nulls blanks, splits comma lists, drops empty goal /
  identifier / relationship rows, collapses a `Custom` enum seed, leaves
  `kind` optional, `parent_ref` an optional plan picker);
- the **i18n catalogue** (every key present in `en`; fallback to `en`
  for a missing key; RTL flag for `ar` / `ur`);
- a **`+layout` render test** asserting the **hamburger toggles the
  nav** (collapsed → expanded), and that the Plans destination and the
  theme + locale selectors render.

Component tests run via `@testing-library/svelte` mounted client-side
by the `svelteTesting()` vite plugin.

**Playwright** smoke tests (`tests/e2e/`) with the API stubbed via
`page.route`, run against the production build (`vite preview`) to
avoid the `vite dev` cold-start module race: load the plan identity
routes; the list search box (matching keeps the row,
non-matching shows the empty message); the detail-page merge action
(check-duplicates → confirm merge → success, asserting the merge
endpoint fired); the audit timeline; the recent-activity panel; the
child-plan roll-up; the Kanban board (drag a card →
status PATCH fired); the issues list; the timeline / burndown views
render; and the layout (hamburger toggles nav; theme + locale selectors
switch `data-theme` / `lang` + `dir`, incl. RTL for `ar`).

Run: `pnpm test` (vitest) and `pnpm test:e2e` (Playwright).

## 12. Compliance

Plans are business / portfolio artefacts; defer to the service's
controls for access / audit. Lead / assignee / author refs are opaque
`EntityRef`s into person / worker — the front-end displays them but does
not resolve PII beyond what the referenced services return. Cross-service
links are **never** a match signal (entity spec §1).

## 13. Tasks (live work queue)

> This is the build queue for the implemented app (MVP shipped); check off
> in three-part PRs (spec + code + test).

- [x] **2026-07-22 — Capability views (service spec §9.4a).**
  `CapabilityClient` + wire types in `src/lib/api/capabilities.ts`, and
  four pages: `/prioritisation` (Smart Score queue + per-component
  explanation), `/lifecycle` (the phase funnel), `/reviews`
  (delegation, verdicts, consensus), `/automations` (rules, the
  deadline queue + manual sweep, and the run log). Nav entries added;
  vitest pins every client path. English-first (locale catalogues not
  extended yet); no standalone notifications page.

- [x] **2026-07-22 — Offer the five new `kind` labels.** `PlanKind` /
  `ALL_KINDS` in `src/lib/api/types.ts` gain `Practice`, `Process`,
  `Purpose`, `Pathway`, `Proposal`, so the `PlanForm` kind `<select>`
  lists them. `COLLECTIONS` gains the matching plural segments except
  `proposals`, which already names the proposals catalogue route.

- [x] **Unify the four collections into one recursive `/api/plans`
  collection + rename the entity to `plan` (2026-07-20).** Collapsed the
  dynamic `[collection]` route directory to a static `plans/` directory
  (`/plans`, `/plans/new`, `/plans/[pid]`,
  `/plans/[pid]/{edit,board,schedule,governance}`); removed the collection
  switcher (one **Plans** destination). `WorkItemForm` → `PlanForm`
  (`kind` now an **optional** select); `src/lib/api/work-items.ts` →
  `src/lib/api/plans.ts` exporting `PlanRepository` (not collection-bound;
  all paths under `/api/plans`, `?parent=` roll-up,
  `/api/plans/{pid}/schedule`); `types.ts` renamed `WorkItem` → `Plan`,
  `kind` optional, `portfolio_ref` → `parent_ref`. Tests renamed to
  `tests/unit/plans.test.ts` + `tests/unit/plan-form.test.ts`. `kind` is
  an optional descriptive label; matching is **not** gated by kind.
  svelte-check 0/0, 45 vitest pass.
- [x] Scaffold the SvelteKit 2 / Svelte 5 (runes) SPA: `package.json`,
  `svelte.config.js`, `vite.config.ts`, `tsconfig` (strict +
  `noUncheckedIndexedAccess`), `src/app.html`, `src/app.css`,
  `.env.example` (`PUBLIC_API_BASE_URL`, `VITE_AUTH_FRONTEND_URL`).
- [x] `src/lib/api/types.ts` — mirror `project_portfolio_management_matcher::Plan` +
  `PlanKind` (optional) + sub-resource + DTO shapes (per §8).
- [x] `src/lib/api/client.ts` — lean fetch wrapper (get/post/put/patch/
  delete) + `ApiError`. Browser→BFF calls only; the SvelteKit server
  attaches the PASETO bearer server-side (no client-held token).
- [x] `src/lib/api/plans.ts` — `PlanRepository` (CRUD + search +
  checkDuplicates + merge + audit + recentEvents + `?parent=` child
  roll-up + sub-resource methods + timeline + burndown; all paths under
  `/api/plans`).
- [x] Auth — adopt BFF + httpOnly cookie + CSRF: `hooks.server.ts` /
  server routes read the `__Host-mxi_session` cookie, exchange it for a
  short-lived PASETO server-side, and attach it when calling the
  portfolio service; CSRF on mutating browser→BFF calls. No
  `mxi_access_token` / `localStorage` bearer, no fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
- [x] `src/lib/config.ts` — `PUBLIC_API_BASE_URL` + `VITE_AUTH_FRONTEND_URL`
  + `signInUrl()` (BFF sign-in redirect).
- [x] `src/lib/i18n/` — 13-locale catalogues (en, cy, es, fr, de, ar,
  ru, hi, zh, bn, pt, id, ur) + RTL flags + `en` fallback.
- [x] `+layout.svelte` / `+layout.ts` — top-bar nav with **leftmost
  hamburger**, full-width content, Plans destination + theme + locale
  selectors, session affordance (Sign in / Sign out — BFF, no token
  paste); SPA toggles.
- [x] Plan identity routes: `/plans` (SVAR
  DataGrid list + search + recent activity), `/plans/new`,
  `/plans/[pid]` (detail + delete + check-duplicates +
  MatchBreakdown + merge + audit timeline; child-plan roll-up),
  `/plans/[pid]/edit`.
- [x] `PlanForm` component (incl. org picker, person/worker lead
  picker, optional plan `parent_ref` picker, goals / identifiers /
  relationships editors; `kind` optional select).
- [x] `MatchBreakdown` visual component (per-component score bars incl.
  goals / parent / relationships / tags; no `plan_type` / kind gate).
- [x] Kanban task board (`/plans/[pid]/board`) — drag = status PATCH.
- [ ] Issues list (`/plans/[pid]/issues`).
- [ ] Gantt / timeline view (`/plans/[pid]/timeline`).
- [ ] Burndown chart (`/plans/[pid]/burndown`).
- [ ] Goals panel (`/plans/[pid]/goals`).
- [x] vitest unit suite (client, `PlanRepository`, BFF auth
  integration, signInUrl, `PlanForm`, i18n, `+layout`
  hamburger-toggle render test).
- [x] Playwright e2e smoke (plan routes, search, merge, audit,
  recent activity, child-plan roll-up, board drag, issues,
  timeline/burndown, layout hamburger + theme/locale switch incl. RTL).

## 14. Implementation status

**Implemented (MVP, v0.1.0).** The SvelteKit app is built and verified (svelte-check clean, vitest + Playwright green): the routes in §5 are live against the sibling service via the BFF proxy, with SVAR grid / Kanban / Gantt views, Lily theme + locale chrome, and 13-locale i18n. Open §13 items (the roadmap sub-list in §5) remain unchecked.

## 15. Roadmap

- **v0.1** (here): spec + docs only.
- **v0.2**: scaffold + plan identity routes (list / create / detail /
  edit) + lean client + repository + BFF auth (httpOnly cookie + CSRF, per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md))
  + SSO sign-in + layout shell (top-bar hamburger, theme + locale
  selectors) + vitest + Playwright smoke.
- **v0.3**: duplicate-check + MatchBreakdown visual + merge + audit
  timeline + child-plan roll-up.
- **v0.4**: project-management views — Kanban board, issues, timeline /
  Gantt, burndown, goals.
- **v0.5**: full 13-locale translation catalogues + RTL polish.
- **Deferred / roadmap-only**: posts feed + threaded comments, member /
  role panel (not part of the v1 portfolio sub-resource set).

## 16. Open questions

- Project-management views as detail-page tabs or discrete sub-routes?
- Real-time duplicate warning on the create form?
- Which charting primitive for the Gantt / burndown (SVAR vs. native
  SVG vs. a Lily chart helper)?
- How rich should the org / person / worker / plan pickers be
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
`src/lib/api/types.ts` in lockstep with the matcher `Plan` type — if
a field changes in the service / matcher, fix it here in the same cycle.
