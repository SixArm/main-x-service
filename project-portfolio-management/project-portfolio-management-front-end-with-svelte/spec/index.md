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
  plan form, a client-side name filter on the list, and a
  duplicate-check screen (score + confidence per candidate). A
  standalone record-merge page (`/plans/merge`, landed 2026-08-03) folds
  a confirmed duplicate into a survivor, with a recent-merge history
  table. **Not built**: a per-component match-score breakdown visual and
  a per-plan audit timeline (§13).
- The **child roll-up** (not yet built, §13): a plan detail page is meant
  to also list its child plans (those whose `parent_ref` is this plan's
  pid); the service supports it (`GET /api/plans?parent={pid}`, and
  `PlanRepository` carries the query param), but no route calls it yet.
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
comments / members are roadmap-only). There is **no password /
credential form** — sign-on is passwordless magic-link, run through this
app's own `/signin` + `/verify` BFF routes against the central
[authentication-service](../../../authentication/authentication-service-with-loco/).
Per the BFF + httpOnly-cookie model
([`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md))
the browser holds only the `__Host-mxi_session` cookie and the SvelteKit
server attaches a short-lived PASETO server-side (calling the portfolio
service through the same-origin `/api/proxy` route) — no token in JS, no
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

1. **List** active plans (`GET /api/plans`) in a **SVAR DataGrid** (name,
   pid columns) with a **SVAR FilterBar** doing a client-side, contains-
   match filter over the already-loaded rows by name.
   - **Not built** (§13): a search-on-submit box hitting
     `GET /api/plans/search?q=` — `PlanRepository.search()` wraps the
     endpoint and is unit-tested, but no route calls it, so name search
     stays client-side over the current page. There is also no
     recent-activity feed (`GET /api/plans/events/recent`) — no
     repository method, type, or UI exists for it.
2. **Create** (`POST /api/plans`), redirect to the new detail page. The
   `kind` label is chosen on the form (optional).
3. **Detail**: render the stored `Plan`; offer edit, delete,
   check-duplicates, and entry points to the project-management views
   (governance, board, schedule). **Not built** (§13): an audit timeline
   and a child-plan roll-up (those whose `parent_ref` equals this plan's
   pid) on this page. Merge is a **separate, standalone page**
   (`/plans/merge`, item 7 below), not a detail-page action.
4. **Edit** (`PUT`), redirect back to detail.
5. **Delete** (`DELETE`, soft), redirect to the plans list.
6. **Check-duplicates** posts the current record and lists matches
   (name, score, confidence) **across the whole collection**, excluding
   the record itself. Matching is **not** gated by `kind`; there is no
   `plan_type` / kind component. **Not built** (§13): a visual
   per-component **MatchBreakdown** (name / goals / code / owner org /
   parent / timeframe / keywords / relationships / tags) — there is no
   such type or component; the UI shows the raw score and confidence
   only.
7. **Merge** (`/plans/merge`, standalone page, landed 2026-08-03 as
   FE-1): survivor pid + duplicate pid + optional reason, an optional
   side-by-side preview (`GET /api/plans/{pid}` for each), a native
   `confirm()` before the destructive `POST /api/plans/merge` call
   (`{main_pid, duplicate_pid, reason?}`), and a recent-merge history
   table (`GET /api/plans/merges/recent`). Equal pids are guarded
   client-side (the service `422`s on a self-merge); errors render as
   `"<status>: <message>"`. Reachable from a **Merge** entry in the
   top-bar nav.
8. **Audit timeline** — **not built** (§13). No `AuditEntry` type, no
   `PlanRepository` method, no UI. `GET /api/plans/{pid}/audit` exists on
   the service but nothing in this front-end calls it yet.
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
    session affordance. **Sign in** leads to this app's own `/signin` +
    `/verify` pages (not a redirect to a central authentication
    front-end) for the passwordless magic-link; on success the
    authentication-service sets the `__Host-mxi_session` httpOnly cookie.
    The browser holds only that cookie — **no token in JS, no
    `localStorage`, no URL-fragment handoff**. The SvelteKit **server**
    (BFF: `hooks.server.ts` + `src/lib/server/{auth,session,config}.ts`)
    holds the session and attaches a short-lived PASETO server-side when
    calling the portfolio service through the same-origin
    `/api/proxy/[...path]` route; the browser never calls the service
    directly. Mutating browser→BFF calls carry a CSRF token; **Sign out**
    revokes the session. This lets operator traffic through once the
    service turns on blanket enforcement
    (`PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`, off by default). Per
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
model (§6.15) the browser carries only the `__Host-mxi_session` cookie;
`ApiClient`'s base URL is the same-origin `/api/proxy`, and the
SvelteKit server (`src/routes/api/proxy/[...path]`) attaches the
short-lived PASETO server-side when forwarding to the service — no token
is read or attached in browser JS. `PlanForm` builds a `Plan` from the
inputs (comma lists split, blanks nulled, goals / identifiers /
relationships as editable rows; empty rows dropped on submit; a seeded
`Custom` enum collapses to "—"; `kind` is an optional select,
`owner_org_id` / `lead_ref` / `parent_ref` are raw pid text inputs, not
typeahead pickers — §16 open question). The project-management views
(board, and the oversight/executive dashboard routes in §5) call their
sub-resource / dashboard endpoints via a separate `PpmClient`
(`src/lib/api/ppm.ts`), not `PlanRepository`. Layout chrome (Plans
destination, theme / locale selectors, hamburger nav, session affordance)
lives in `+layout.svelte`; `+layout.ts` sets the SPA toggles.

`types.ts` hand-mirrors `project_portfolio_management_matcher::Plan` and
the identity-surface DTO shapes actually in use: `Plan`, `PlanKind`,
`PlanStatus`, `Goal`, `GoalStatus`, `RelationKind`, `PlanRelationship`,
`PlanIdentifier`, `IdentifierScheme`, `PlanRef`, `ScoredRef`,
`MergeRequest`, `MergeResponse`, `MergeRecordRow`. It does **not** carry
`MatchBreakdown`, `AuditEntry`, `PlanEvent`, `TimelineRow`, or
`BurndownPoint` — those capabilities are not built (§13); `Task` /
`TaskStatus` / `Issue*` types live in `ppm.ts` instead, since the board
route uses `PpmClient`, not `PlanRepository`. Keep `types.ts` in lockstep
with any matcher-type change (entity spec §18).

## 9. API consumption

Every plan lives under `/api/plans` (one recursive collection). Rows
below are endpoints an actual route calls today.

| Route / action | Endpoint |
|---|---|
| list | `GET /api/plans` |
| create | `POST /api/plans` |
| detail load | `GET /api/plans/{pid}` |
| delete | `DELETE /api/plans/{pid}` |
| duplicates | `POST /api/plans/check-duplicates` (→ `ScoredRef[]`, score + confidence only) |
| merge (`/plans/merge`) | `POST /api/plans/merge` (`{main_pid, duplicate_pid, reason?}`) |
| merge history (`/plans/merge`) | `GET /api/plans/merges/recent` |
| edit | `PUT /api/plans/{pid}` |
| schedule | `GET /api/plans/{pid}/schedule` |
| board: list / move | `GET /api/plans/{pid}/tasks` · `PATCH /api/plans/{pid}/tasks/{tid}` (status) |
| board: create / edit | `POST /api/plans/{pid}/tasks` · `PUT /api/plans/{pid}/tasks/{tid}` |
| board: sprints / burndown | `GET / POST /api/plans/{pid}/sprints` · `GET /api/plans/{pid}/burndown?sprint=` |

**Not wired to any route** (no repository method, or a method exists but
nothing calls it — §13): search (`GET /api/plans/search?q=` — the
`PlanRepository.search()` method exists and is unit-tested; the list
page filters client-side instead), recent activity
(`GET /api/plans/events/recent`), a per-plan audit timeline
(`GET /api/plans/{pid}/audit`), and the child roll-up
(`GET /api/plans?parent={pid}`, exposed as `listPage()`'s `parent`
scope). There is no `issues` / `goals` / `timeline` sub-resource route
or endpoint at all yet (service spec §9.4 sub-resources deferred on both
sides).

(Sub-resource verbs/paths track the service spec §9; pin them in tests
when the controllers land.)

## 10. Persistence

None client-side beyond in-memory route state and two app-local
`localStorage` keys for chrome preferences (`portfolio:theme` /
`portfolio:locale`). The session lives only in the `__Host-mxi_session`
httpOnly cookie — no auth token in `localStorage` (BFF model, §6.15).

## 11. Testing strategy

`pnpm run check` (svelte-check strict, 0/0). **vitest** unit tests
(`tests/unit/`, 8 files / 62 tests as of 2026-08-03) cover:

- `client.test.ts` — the `ApiClient` (verb / body / headers /
  cookie-credentials / CSRF / error-classification / empty-body, incl.
  `PATCH`);
- `plans.test.ts` (12 tests) — every `PlanRepository` method's path +
  verb: `list()` (incl. the `?parent=` roll-up query, though no route
  passes it today), `search()` `/search?q=` URL-encoding,
  `checkDuplicates()`, `merge()` body `{main_pid, duplicate_pid,
  reason?}`, `recentMerges()`. **No** `audit()` / `recentEvents()` /
  issues / goals / timeline / burndown methods exist to test — those
  capabilities are not built (§13);
- `merge-validation.test.ts` (4 tests) — the pure pre-merge guard (both
  pids required, must differ);
- `capabilities.test.ts` / `ppm.test.ts` — the `CapabilityClient` /
  `PpmClient` endpoint paths used by the oversight/executive dashboard
  routes and the board;
- `plan-form.test.ts` (14 tests) — required-name guard blocks
  `onsubmit` on blank/whitespace; `build()` trims scalars, nulls blanks,
  splits comma lists, drops empty goal / identifier / relationship rows,
  collapses a `Custom` enum seed, leaves `kind` optional;
- `i18n.test.ts` (10 tests) — every key present in `en`; fallback to
  `en` for a missing key; RTL flag for `ar` / `ur`; merge-key coverage;
- `layout.test.ts` (1 test) — the hamburger toggles the nav.

Component tests run via `@testing-library/svelte` mounted client-side
by the `svelteTesting()` vite plugin.

**Playwright** smoke tests, 23 tests across two files, with the API
stubbed via `page.route`, run against the production build
(`vite preview`) to avoid the `vite dev` cold-start module race:

- `tests/e2e/smoke.spec.ts` (7 tests) — the identity surface: the
  list / new / detail / edit pages render; the merge page renders its
  form and the recent-merges table; the nav exposes the merge link; and
  check-duplicates self-exclusion (the record does not list itself as a
  candidate).
- `tests/e2e/ppm.spec.ts` (16 tests) — the oversight/executive dashboard
  views (dashboard, proposals, ideas, executive, financials, technology,
  scenarios, board, auditor, compliance, risk, security, regulator,
  engineering, calendar) and the per-plan Kanban task board (columns,
  burndown, standup digest, sprint notes, velocity).

There is **no** Playwright coverage yet for an audit timeline,
recent-activity panel, child-plan roll-up, issues, timeline, or a
plan-level (non-sprint) burndown — those views are not built.

Both stub files match request paths against the service's bare
`/api/...` contract; since the request actually lands on the BFF proxy
at `/api/proxy/api/...` (`ad95088e`), both strip the `/api/proxy` prefix
before dispatching. (Fixed 2026-08-04, DOC-4 audit: both files also
still navigated to the pre-2026-07-20 `/projects*` routes, which do not
exist — neither `pnpm run check` nor `pnpm test` nor `pnpm run build`
runs Playwright, so 21 of 23 tests had been silently broken since the
`/plans` unification; see CHANGELOG.)

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

- [ ] **FE-2 (M) Masked view + GDPR export UI for `/plans/[pid]`.** The
  service has carried `GET /api/plans/{pid}/masked` and
  `GET /api/plans/{pid}/export` since 2026-08-02 (this crate's own
  `AGENTS.md`/§9 "API consumption" table lists both under "Plan CRUD"),
  but `PlanRepository` has no `masked()`/`export()` method and no route
  calls either path — *(verified: `grep -n "masked\|export(" src/lib/api/plans.ts`
  returns nothing, and `AGENTS.md`'s own repository-method list —
  "list + listPage + search + get + create + update + remove +
  checkDuplicates + merge + recentMerges" — has no masked/export
  entry)*. Add `PlanRepository.masked(pid)` / `.export(pid)`, and a
  "View masked" / "Export (GDPR)" affordance on `/plans/[pid]` (a plain
  link/button rendering the JSON is enough to make the capability
  reachable; a dedicated view is a stretch goal).
  Three-part change: spec §9/§13 + `src/lib/api/plans.ts` +
  `src/routes/plans/[pid]/+page.svelte` + vitest coverage.
  **Acceptance:** new vitest cases pin the two new repository methods'
  URLs; a Playwright smoke test exercises the affordance from
  `/plans/[pid]`.

- [ ] **FE-3 (M) Decide and, if adopted, implement the `requireSignedIn`
  page-visit guard (PRO-H10).** Repo `tasks.md` WEB-1 found the guard
  rolled to 5 of 16 front-ends and explicitly left "the 5/11 roll-out
  question... still open" for the rest — *(verified:
  `grep -rn "requireSignedIn" src/` in this project returns zero
  matches, matching WEB-1's finding, which names "place, organization,
  care-pathway, case, portfolio and all five consumer apps" as
  unguarded)*. This project's auth model (BFF + httpOnly session
  cookie, §8) is structurally identical to the five front-ends that do
  have the guard on `/new`/`/merge`-style mutating routes. Either add
  it (following person's `requireSignedIn(locals)` pattern plus WEB-1's
  `SMOKE_STORAGE_STATE` Playwright stub-cookie fix so the smoke suite
  still renders guarded pages) to `/plans/new`, `/plans/merge`, and
  `/plans/[pid]/edit`, or record in this spec (§8/§13) why portfolio
  deliberately opts out.
  **Acceptance:** either `requireSignedIn` gates the mutating routes
  with a pinned anonymous-303 Playwright test (mirroring person's), or
  spec §8 states the explicit reason portfolio stays unguarded.

- [ ] **FE-4 (M) Consolidate ETag/conditional-GET handling for the
  oversight and executive dashboard views.** The service documents
  every `at-a-glance`/`executive/*`/`board/*`/`auditor/*`/
  `compliance/*`/`financials/*`/`technology/*`/`risk/*`/`security/*`/
  `regulator/*`/`scenarios/compare` read as "ETag-conditional" with an
  `as_of` freshness watermark (service `AGENTS.md`), but the front-end
  handles this on only two of the ~20 route directories that consume
  such views — *(verified: `grep -rln "ETag\|If-None-Match\|304"
  src/routes/ src/lib/` matches only `src/routes/dashboard/+page.svelte`
  and `src/routes/executive/+page.svelte`; `src/lib/api/client.ts` and
  `src/lib/api/ppm.ts` carry no shared `ETag`/`If-None-Match` handling
  at all, so `auditor/`, `board/`, `compliance/`, `financials/`,
  `technology/`, `risk/`, `security/`, `regulator/`, `scenarios/` each
  either reinvent it or, more likely given the grep, never send
  `If-None-Match` and always pay the full payload)*. Add one shared
  helper in `client.ts` (store the last `ETag` per URL, send
  `If-None-Match`, treat `304` as "reuse the cached body") and wire the
  remaining oversight/executive routes to it.
  Three-part change: spec §9/§13 + `src/lib/api/client.ts` + the
  consuming routes + vitest coverage of the 304-reuse path.
  **Acceptance:** a vitest case proves the shared helper reuses the
  cached body on a stubbed `304`; at least the `board`/`auditor`/
  `financials`/`technology` routes are wired to it.

- [x] **2026-08-05 — Wire `/plans` itself to `listPage()`.** The sibling
  gap the entry directly below flagged and deliberately left open:
  `PlanRepository.listPage()` existed and was tested at the repository
  level, but `src/routes/plans/+page.svelte` still called the
  unpaginated `list()`. Now it calls `listPage()` and shows the same
  `shown / total` count (hidden when the list is empty) as
  `/organizations`, `/automations`, and `/reviews`. No `?parent=`
  roll-up wiring added here — the list route never took a parent scope
  before this change either, so that stays out of scope (only
  `/plans/[pid]`'s detail page passes a `parentRef`, and not to this
  route). Tests: 3 new cases in `tests/unit/plans.test.ts` pinning
  `listPage()`'s URL, header parsing, and the header-absent fallback.
  No new Playwright coverage — the existing `list page renders the
  seeded plan` smoke test already exercises the paginated path (its
  stub has no pagination headers, so `total` falls back to the page
  length, and the test's assertions were unaffected). svelte-check 0/0,
  68 vitest pass (65 → 68), build green, 23/23 Playwright pass.

- [x] **2026-08-05 — Pagination on the four consuming capability-view
  lists (repo tasks.md PG-1's last sub-bullet).** The service's
  `automations` / `automations/runs` / `scheduled-actions` / `reviews`
  endpoints gained `?limit=&offset=` + `X-Total-Count`/`X-Limit`/
  `X-Offset` (service spec §9.1/§9.4a); this app wires the two routes
  that consume them:
  - `src/lib/api/capabilities.ts` — five new `*Page` methods
    (`listAutomationsPage`, `runsPage`, `listScheduledPage`,
    `listReviewsPage`, `inboxPage`), each a thin `ApiClient.getPage()`
    wrapper alongside its existing unpaginated sibling (same pattern as
    `PlanRepository.listPage()` next to `list()`).
  - `/automations` (`src/routes/automations/+page.svelte`) — all three
    of its lists (rules, the deadline queue, the run log) now call the
    `*Page` methods and show a `shown / total` count above each table
    (hidden when the list is empty; matches the organization service's
    front-end `/organizations` list-route convention).
  - `/reviews` (`src/routes/reviews/+page.svelte`) — the delegation
    list gets the same treatment.
  - **`/api/notifications` stays unconsumed by any route** (documented
    gap since the 2026-07-22 capability-views entry below); `inboxPage()`
    exists for API-contract parity with the other four but nothing
    calls it. No notifications inbox page was built here — that would
    be new UI scope, not a mechanical pagination follow-up.
  - **No pager controls (Prev/Next) were added.** The family convention
    (`agents/share/restful.md`) is deliberately headers-plus-count, not
    a cursor UI yet; `/organizations` and `/plans` show the same
    restraint. A caller past the default page still reaches deeper rows
    via `?limit=&offset=` directly.
  - Tests: 3 new cases in `tests/unit/capabilities.test.ts` pinning the
    five new methods' URLs and `Page<T>` parsing (reusing
    `tests/unit/client.test.ts`'s `getPage` header-stub pattern). No
    Playwright coverage added — `/plans`'s own `listPage()` had none
    either at the time (it was not yet wired into
    `/plans/+page.svelte`, a pre-existing gap this task did not expand
    its scope to close), so there was no established e2e depth to
    match. **Closed by the entry directly above** (same day).
  - *Verified:* svelte-check 0 errors/0 warnings; 65 vitest pass
    (62 → 65); `pnpm run build` green.

- [x] **2026-08-03 — FE-1: the record-merge page (`/plans/merge`).**
  A standalone operator page for folding a confirmed-duplicate plan into
  a survivor: survivor pid + duplicate pid + optional reason, an optional
  side-by-side preview (`GET /api/plans/{pid}` for each), a native
  `confirm()` before the destructive call, and a recent-merge history
  table read from `GET /api/plans/merges/recent`. New: `MergeRequest` /
  `MergeResponse` / `MergeRecordRow` in `src/lib/api/types.ts`;
  `PlanRepository.merge(request)` (signature changed from positional args
  to the request object, matching the wire body) and
  `PlanRepository.recentMerges()`; the pure guard
  `src/lib/components/merge-validation.ts` (both pids required, must
  differ — the service answers `422` on a self-merge); a `nav.merge` entry
  in the top-bar nav. **Wire shape note:** this service's merge response
  is `{main_pid, duplicate_pid, main}` with **no** `merge_record` wrapper
  (unlike the person service), so the page shows the survivor's pid and
  reads the history row back separately. Errors are rendered as
  `"<status>: <message>"` — this crate's `ApiError` carries no error
  code. 24 i18n keys added across all 13 locales. Tests:
  `tests/unit/merge-validation.test.ts` (4 cases), repository
  merge/recentMerges path pins in `tests/unit/plans.test.ts`, a merge-key
  coverage + placeholder test in `tests/unit/i18n.test.ts`, and two
  Playwright smoke pins. svelte-check 0/0, 62 vitest pass, build green.

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
  `noUncheckedIndexedAccess`), `src/app.html`, `src/app.css`. The
  original scaffold's `.env.example` (`PUBLIC_API_BASE_URL`,
  `VITE_AUTH_FRONTEND_URL`) named the client-held-token model's vars;
  it was superseded when the BFF (own `/signin`+`/verify`+`/api/proxy`)
  landed and now documents `PROJECT_PORTFOLIO_MANAGEMENT_API_URL` /
  `AUTH_API_URL`, read server-side only (fixed 2026-08-04, DOC-4 audit —
  the vars had drifted out of every doc, not just this one).
- [x] `src/lib/api/types.ts` — mirror `project_portfolio_management_matcher::Plan` +
  `PlanKind` (optional) + the identity-surface DTO shapes (per §8; not
  the sub-resource `Task`/`Issue` shapes, which live in `ppm.ts`).
- [x] `src/lib/api/client.ts` — lean fetch wrapper (get/post/put/patch/
  delete) + `ApiError`. Browser→BFF calls only; the SvelteKit server
  attaches the PASETO bearer server-side (no client-held token).
- [x] `src/lib/api/plans.ts` — `PlanRepository` (list + listPage +
  search + get + create + update + remove + checkDuplicates + merge +
  recentMerges; all paths under `/api/plans`). **Not built**: `audit()`,
  `recentEvents()`, or any issues/goals/timeline/burndown method — see
  §9.
- [x] Auth — adopt BFF + httpOnly cookie + CSRF: `hooks.server.ts` +
  `src/lib/server/{auth,session,config}.ts` read the
  `__Host-mxi_session` cookie, exchange it for a short-lived PASETO
  server-side, and attach it when calling the portfolio service via
  `src/routes/api/proxy/[...path]`; CSRF on mutating browser→BFF calls.
  Sign-in is this app's own `signin/` + `verify/` routes, not a redirect
  to a central authentication front-end. No `mxi_access_token` /
  `localStorage` bearer, no fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
- [x] `src/lib/config.ts` — `API_BASE_URL` (same-origin `/api/proxy`,
  hardcoded, not env-driven). There is no client-side `signInUrl()`; the
  Sign in link is a plain `href="/signin"`.
- [x] `src/lib/i18n.svelte.ts` — 13-locale catalogues (en, cy, es, fr, de, ar,
  ru, hi, zh, bn, pt, id, ur) + RTL flags + `en` fallback (one file, not
  a directory).
- [x] `+layout.svelte` / `+layout.ts` — top-bar nav with **leftmost
  hamburger**, full-width content, Plans destination + theme + locale
  selectors, session affordance (Sign in / Sign out — BFF, no token
  paste); SPA toggles.
- [x] Plan identity routes: `/plans` (SVAR DataGrid list + client-side
  name filter), `/plans/new`, `/plans/[pid]` (detail + delete +
  check-duplicates, plain score/confidence), `/plans/[pid]/edit`,
  `/plans/merge` (FE-1, above). **Not built** on any of these: a
  server-round-trip search box, recent activity, a MatchBreakdown
  visual, a per-plan audit timeline, and the child-plan roll-up — see §9.
- [x] `PlanForm` component (goals / identifiers / relationships editors;
  `kind` optional select). `owner_org_id` / `lead_ref` / `parent_ref`
  are plain pid text inputs, not typeahead pickers (§16 open question,
  still open).
- [ ] `MatchBreakdown` visual component (per-component score bars incl.
  goals / parent / relationships / tags; no `plan_type` / kind gate).
  **Not built** — no type, no component; `check-duplicates` renders raw
  score + confidence.
- [x] Kanban task board (`/plans/[pid]/board`) — drag = status PATCH.
  Uses `PpmClient`/`Task` (`ppm.ts`), not `PlanRepository`.
- [ ] Issues list (`/plans/[pid]/issues`).
- [ ] Gantt / timeline view (`/plans/[pid]/timeline`).
- [ ] Burndown chart (`/plans/[pid]/burndown`) — the board's per-sprint
  burndown (`GET /api/plans/{pid}/burndown?sprint=`) is not this: it is
  scoped to one sprint, not the plan-level "remaining estimate over
  time" series this item describes.
- [ ] Goals panel (`/plans/[pid]/goals`).
- [x] vitest unit suite, 8 files / 62 tests: `client`, `plans`
  (`PlanRepository`), `merge-validation`, `capabilities`, `ppm`,
  `plan-form`, `i18n`, `layout` (hamburger-toggle render test). See §11.
- [x] Playwright e2e smoke, 25 tests across `tests/e2e/{smoke,ppm}.spec.ts`:
  the plan identity routes render, the merge page + recent-merges table,
  the nav's merge link, check-duplicates self-exclusion, every
  oversight/executive dashboard view, and the Kanban board (columns,
  burndown, standup, sprint notes, velocity). **Not covered**: recent
  activity, audit timeline, child-plan roll-up, issues, plan-level
  timeline/burndown, or a layout hamburger/theme/locale/RTL switch test
  — see §11. Fixed 2026-08-04 (DOC-4 audit): both spec files still
  navigated to pre-unification `/projects*` routes and neither stripped
  the BFF proxy prefix, so 21/23 tests had been failing since
  2026-07-20/`ad95088e` respectively, undetected because no other
  command in this project's test pyramid runs Playwright.
- [x] **Fix: `/calendar` never actually rendered a milestone event
  (2026-09-06).** `@svar-ui/calendar-store` requires an all-day event's
  `end` to be strictly *after* `start`; `+page.svelte` passed
  `end: day` — the same `Date` object as `start` — so the SVAR Calendar
  widget silently dropped every milestone it was ever asked to show.
  The existing `"delivery calendar lists milestones with kinds"` test
  never caught this because it asserted only the fallback `<ul>`
  (`milestone-list`), a separate render path from the calendar widget
  itself. The identical bug and fix already landed in worker-front-end
  and person-front-end's `/expiry` calendars (repo `tasks.md`). Fixed
  by computing the following calendar day as `end`; the test now also
  asserts the calendar widget (`milestone-calendar`) itself renders the
  event text, and the stubbed milestone's due date is computed relative
  to the actual test-run date (a fixed date would eventually scroll
  outside the widget's default "today's month" view). Verified the
  strengthened assertion actually fails without the fix before
  confirming it passes with it.

## 14. Implementation status

**Implemented (MVP, v0.1.0).** The SvelteKit app is built and verified (svelte-check clean, vitest + Playwright green): the routes in §5 are live against the sibling service via the BFF proxy, with SVAR grid / Kanban / Gantt views, Lily theme + locale chrome, and 13-locale i18n covering the original identity + merge surface (the later oversight/executive dashboard views are English-first — CHANGELOG 2026-07-22). Open §13 items — the plan-detail audit timeline, recent activity, MatchBreakdown, child roll-up, and the issues/timeline/goals sub-routes (the roadmap sub-list in §5) — remain unchecked.

## 15. Roadmap

- **v0.1** (here): spec + docs only.
- **v0.2**: scaffold + plan identity routes (list / create / detail /
  edit) + lean client + repository + BFF auth (own `/signin`+`/verify`,
  httpOnly cookie + CSRF, per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md))
  + layout shell (top-bar hamburger, theme + locale
  selectors) + vitest + Playwright smoke.
- **v0.3**: duplicate-check (done) + merge (done, as the standalone
  `/plans/merge` page, FE-1 2026-08-03) + MatchBreakdown visual (not
  built) + audit timeline (not built) + child-plan roll-up (not built).
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
  [models](../../agents/models.md); [matching](../../agents/matching.md).
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
