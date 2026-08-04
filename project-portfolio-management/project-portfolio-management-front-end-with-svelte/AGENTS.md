# AGENTS.md — Portfolio Front-End

Operator UI for the [Portfolio Service](../project-portfolio-management-service-with-loco):
plan identity CRUD + matching + merge over **one recursive `/api/plans`
collection**, plus the project-management workspace (Kanban board,
governance, schedule) and cookie-session / SSO auth (BFF; the browser
holds no token).

> Read [`spec/index.md`](./spec/index.md) first — the living spec
> (§1–§18). Entity umbrella: [`../spec/index.md`](../spec/index.md).
>
> **Status: implemented (MVP, v0.1.0).** `pnpm install` works, `pnpm run
> check` is **0 errors / 0 warnings**, `pnpm test` (vitest) is green, and
> `pnpm run build` succeeds. Shipped: the unified identity surface — a
> static `plans/` route layer (`/plans`, `/plans/new`, `/plans/[pid]`,
> `/plans/[pid]/{edit,board,schedule,governance}`, `/plans/merge`) with
> list (SVAR grid + client-side filter) + create + detail + edit + delete
> + check-duplicates + record-merge (FE-1, 2026-08-03), the `PlanForm`,
> the `PlanRepository`, the lean `ApiClient`, the BFF proxy +
> cookie-session auth, the Kanban board (`/plans/[pid]/board`), a wide set
> of oversight / executive dashboard views (§5 of the spec), and the
> 13-locale i18n + top-bar hamburger + theme/locale selectors (the
> dashboard-style views added after the original MVP stay English-first).
> **Not built** (per `spec/index.md` §13, distinct from "deferred" — no
> repository method, type, or UI element exists for these yet): the
> per-plan audit timeline, the recent-activity feed, a match-score
> breakdown visual, and the detail page's child-plan roll-up. Also not
> built: the issues list, Gantt/timeline, burndown, and goals sub-routes
> (`/plans/[pid]/{issues,timeline,burndown,goals}`) — the service defers
> those sub-resources too.

## What this is

A SvelteKit 2 / Svelte 5 (runes) **SPA**. It calls the portfolio service
REST API under `/api/plans/...`, whose request/response body for the
identity record **is** the `project_portfolio_management_matcher::Plan`
shape itself (the API DTO = the matcher type, persisted as JSONB). There
is **one recursive collection** — every record is a **plan**, all under
`/api/plans`, with a single controller shape; matching runs across the
whole collection (it is **not** gated by kind). A plan carries an
**optional** `kind` descriptive label (Portfolio / Project / Product /
Program / Practice / Process / Purpose / Pathway / Proposal) and an
optional `parent_ref` to another plan (any plan may contain any other,
so the tree is recursive). The operational sub-resources (goals, tasks,
issues) hang off a plan under `/api/plans/{pid}/…` and are **not** part
of the matching surface (except goal titles).

## Single source of truth

- The service's [`spec.md`](../project-portfolio-management-service-with-loco/spec/index.md)
  and [`AGENTS/`](../AGENTS/) describe the API contract. If a field
  changes in `Plan` (the matcher type) in the service, fix
  `src/lib/api/types.ts` here in the **same** change cycle — do not let
  the front-end drift.
- This project has its own [`spec.md`](spec/index.md) (§1–§18) for
  front-end-specific decisions: routes, components, design system,
  build, layout shell, i18n.

## Ground rules

1. **Runes only** (`$state` / `$derived` / `$effect` / `$props` /
   `$bindable`). No `export let`, no `$:`, events are callback props.
2. **SPA.** `+layout.ts` sets `ssr = false` / `prerender = false`.
3. **TypeScript strict** (`noUncheckedIndexedAccess`). No `any` without
   a justifying comment.
4. **SVAR DataGrid** for tabular data (the plans list, sub-resource
   tables). Native HTML for simple lists.
5. **Lily Headless** for accessibility primitives (focus trap,
   listbox, combobox, dialog) and the theme / locale selectors. Native
   HTML elsewhere.
6. **No global stores** for HTTP state — construct a
   `PlanRepository` (all paths under `/api/plans`) per page/component.
7. **No envelope.** The service is loco.rs and returns **raw JSON**;
   `src/lib/api/client.ts` is the lean wrapper (get/post/put/patch/
   delete).
8. **Drift accepted** (repo decision 2026-06-02): own copy of API
   client / types / form primitives; do not factor a shared package
   without explicit user approval.

## Layout shell (cross-cutting family rule)

- Global navigation is a **top navigation bar** (header) spanning the
  full viewport width. There MUST NOT be a left-hand sidebar / rail.
- The navigation toggle is a **hamburger menu placed LEFTMOST** in the
  top bar; narrow viewports collapse the nav behind it.
- The main content area is **full-width**.
- The nav has a single **Plans** destination (no collection switcher).
- The chrome area carries the **theme selector**
  (`lily-design-system-svelte-theme-picker` — full shared catalogue;
  selecting a theme restyles the whole site; persists to
  `localStorage["portfolio:theme"]`), the **locale selector**
  (`lily-design-system-svelte-locale-picker` — 13 locales; selecting
  one switches the language; `ar` / `ur` are RTL; persists to
  `localStorage["portfolio:locale"]`), and the session affordance (Sign
  in / Sign out; the browser holds no token — see **Auth** below).

## Layout (actual `src/` tree, portfolio-specific parts)

The oversight / executive dashboard route directories (`auditor/`,
`automations/`, `board/`, `calendar/`, `capacity/`, `compliance/`,
`dashboard/`, `engineering/`, `executive/`, `financials/`, `gantt/`,
`ideas/`, `lifecycle/`, `objectives/`, `prioritisation/`, `proposals/`,
`regulator/`, `reports/`, `reviews/`, `risk/`, `scenarios/`,
`security/`, `technology/`) are listed in [README.md](README.md)'s
routes table, not repeated here.

```
src/
├── hooks.server.ts                BFF session handling (reads the httpOnly session cookie)
├── lib/
│   ├── config.ts                  API_BASE_URL → same-origin BFF proxy (/api/proxy); no browser-held bearer
│   ├── i18n.svelte.ts             13-locale catalogues (en default) + RTL flags + en fallback; one file, not a directory
│   ├── api/
│   │   ├── client.ts              lean fetch wrapper (get/post/put/patch/delete + ApiError); credentials: 'include' (cookie) + CSRF header
│   │   ├── types.ts               Plan + PlanKind + PlanStatus + Goal + Task + Issue (+ IssueKind/IssueSeverity/IssueStatus) + Relationship + RelationKind + PlanIdentifier + IdentifierScheme + PlanRef + ScoredRef + MergeRequest + MergeResponse + MergeRecordRow. No MatchBreakdown / AuditEntry / PlanEvent / TimelineRow / BurndownPoint type — those capabilities are not built (see the Status note above).
│   │   ├── capabilities.ts        CapabilityClient (reviews, assignees, notifications, automations, scheduled actions, Smart Score, lifecycle)
│   │   ├── ppm.ts                 PpmClient — the oversight/executive dashboard views' endpoints (board, governance, schedule, auditor, compliance, …)
│   │   └── plans.ts               PlanRepository (list + listPage + search + get + create + update + remove + checkDuplicates + merge + recentMerges; all paths under /api/plans). No audit() or recentEvents() method.
│   ├── server/                    BFF-only (never bundled to the browser): auth.ts (magic-link + session→PASETO exchange), session.ts (cookie), config.ts (PROJECT_PORTFOLIO_MANAGEMENT_API_URL / AUTH_API_URL)
│   └── components/                PlanForm.svelte, merge-validation.ts (pure guard). No MatchBreakdown / KanbanBoard / IssuesList / Timeline / Burndown / GoalsPanel / picker components — the board route uses @svar-ui/svelte-kanban directly.
└── routes/
    ├── +layout.svelte / +layout.ts / +layout.server.ts   top-bar nav (leftmost hamburger) + Plans destination + theme/locale selectors + session affordance + SPA toggle
    ├── signin/ · verify/          per-app magic-link sign-in (BFF server routes)
    ├── api/proxy/[...path]/+server.ts   BFF proxy → portfolio service (injects the PASETO bearer)
    ├── +page.svelte               landing (links to /plans)
    └── plans/
        ├── +page.svelte          list (SVAR DataGrid + client-side FilterBar over the loaded rows)
        ├── new/+page.svelte      create
        ├── [pid]/+page.svelte    detail + delete + check-duplicates (plain score/confidence, no breakdown visual)
        ├── [pid]/edit/+page.svelte    edit
        ├── [pid]/{board,schedule,governance}/+page.svelte
        └── merge/+page.svelte    merge a duplicate into a survivor + recent merge history
```

## API consumption

Every plan lives under `/api/plans` (one recursive collection).

Rows below are endpoints an actual route calls today. `PlanRepository`
also defines `search()` (`GET /api/plans/search?q=`) and `listPage()`'s
`?parent=` roll-up scope, but no route calls either — the list page's
search box is a client-side filter over the already-loaded rows instead.
There is no repository method, type, or route at all for a recent-activity
feed, a per-plan audit timeline, or a match-score breakdown visual (see
the Status note above).

| UI action | Endpoint |
|---|---|
| List | `GET /api/plans` |
| Create | `POST /api/plans` |
| Detail | `GET /api/plans/{pid}` |
| Edit | `PUT /api/plans/{pid}` |
| Delete | `DELETE /api/plans/{pid}` |
| Check duplicates | `POST /api/plans/check-duplicates` → `ScoredRef[]` (score + confidence; no breakdown) |
| Merge duplicate (`/plans/merge`) | `POST /api/plans/merge` (body `{main_pid, duplicate_pid, reason?}`) |
| Merge history (`/plans/merge`) | `GET /api/plans/merges/recent` |
| Schedule | `GET /api/plans/{pid}/schedule` |
| Tasks (board) | `GET / POST /api/plans/{pid}/tasks` · `PUT / PATCH /api/plans/{pid}/tasks/{tid}` (PATCH = status move) |
| Sprints + burndown (board) | `GET / POST /api/plans/{pid}/sprints` · `GET /api/plans/{pid}/burndown?sprint=` |
| Collaborative review (`/reviews`) | `POST / GET /api/reviews` · `/{pid}/respond` · `/{pid}/submit` · `GET /api/reviews/consensus` |
| Automations (`/automations`) | `POST / GET /api/automations` · `/{pid}/enable`·`/disable` · `GET /api/automations/runs` |
| Scheduled actions (`/automations`) | `POST / GET /api/scheduled-actions` · `POST /api/scheduled-actions/sweep` |
| Smart Score (`/prioritisation`) | `GET /api/plans/{pid}/smart-score` · `GET /api/prioritisation` |
| Lifecycle (`/lifecycle`) | `GET /api/lifecycle` · `GET /api/plans/{pid}/lifecycle` |

`src/lib/api/capabilities.ts` additionally defines a per-task assign
endpoint, an assignee-workload query, and a notifications inbox
(`GET /api/notifications`); none is called by a route yet — the CHANGELOG's
2026-07-22 entry already flags the notifications gap specifically. There
is no `/plans/[pid]/{issues,timeline,goals}` route or endpoint at all
(service spec §9.4 sub-resources not yet built on either side).

## Three-part change rule

A behavioural change is one PR with three parts:

1. **Spec edit** — `spec.md` §13 (Tasks) or the relevant numbered
   section.
2. **Code edit** — `src/`.
3. **Test edit** — `tests/unit/` (vitest) and/or `tests/e2e/`
   (Playwright).

## Commands (once scaffolded)

```bash
pnpm install
pnpm dev          # http://localhost:5173
pnpm run check    # svelte-check (strict; 0/0 expected)
pnpm run build
pnpm test         # vitest unit suite
pnpm test:e2e     # Playwright smoke (runs against `vite preview`)
```

## Auth — BFF + cookie session (no token in the browser)

**BFF model (current).** The browser holds no token: sign-in establishes
a server-side **cookie session** (`__Host-mxi_session`, httpOnly), the
browser talks only to this front-end's own SvelteKit server (BFF), and
the BFF exchanges the session for a short-lived **PASETO v4.public**
token and calls the portfolio service server-side. Mutating requests are
CSRF-protected; there is no `localStorage` and no `mxi_access_token`.
Service-side enforcement (`PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`) is
off by default.

- **Sign in** — the top bar's **Sign in** leads to this app's own
  `signin/` route (not a redirect to a central authentication front-end);
  the magic-link flow completes at `verify/`, which establishes the
  server-side session and sets the httpOnly `__Host-mxi_session` cookie.
- **BFF** — `src/hooks.server.ts` reads the session cookie;
  `src/lib/server/auth.ts` performs the magic-link + session→PASETO
  exchange; `src/routes/api/proxy/[...path]/+server.ts` is the reverse
  proxy that injects the PASETO bearer and forwards to the portfolio
  service. There is no client-held session store
  (`$lib/auth.svelte.ts` does not exist).
- **`ApiClient`** — browser→BFF calls (`API_BASE_URL` = same-origin
  `/api/proxy`) send the cookie (`credentials: 'include'`); state-changing
  requests carry a **CSRF token** (`X-CSRF-Token`). Safe `GET`/`HEAD` are
  CSRF-exempt.

Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256 JWT + JWKS and any cross-origin `#access_token` fragment handoff
are decommissioned).

Configure the BFF's upstream URLs with the server-side env vars
`PROJECT_PORTFOLIO_MANAGEMENT_API_URL` (portfolio service) and
`AUTH_API_URL` (authentication service) — see `src/lib/server/config.ts`;
both default to `http://localhost:5150` (see `.env.example`).

## What does NOT live here

- FHIR Plan UI — the service has no FHIR surface.
- Consent-management UI — out of scope for MVP.
- Finance / budgeting UI — not a finance system (entity spec §1.3).
- Posts feed / threaded comments — not part of the v1 portfolio
  sub-resource set (tasks / goals / issues only); roadmap-only.
- Members / role panel — not part of the v1 sub-resource set;
  roadmap-only.
- A password / credential form — sign-on is passwordless magic-link
  (own `/signin` + `/verify` BFF routes calling the central
  authentication-service); this app never handles or stores a password,
  and the browser never holds a token.
