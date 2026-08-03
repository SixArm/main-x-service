# AGENTS.md — Portfolio Front-End

Operator UI for the [Portfolio Service](../project-portfolio-management-service-with-loco):
plan identity CRUD + matching + name search + merge + audit timeline
over **one recursive `/api/plans` collection**, plus the
project-management workspace (Kanban board, issues, timeline / Gantt,
burndown, goals) and cookie-session / SSO auth (BFF; the browser holds no
token).

> Read [`spec/index.md`](./spec/index.md) first — the living spec
> (§1–§18). Entity umbrella: [`../spec/index.md`](../spec/index.md).
>
> **Status: implemented (MVP, v0.1.0).** `pnpm install` works, `pnpm run
> check` is **0 errors / 0 warnings**, `pnpm test` (vitest) is green, and
> `pnpm run build` succeeds. Shipped: the unified identity surface — a
> static `plans/` route layer (`/plans`, `/plans/new`, `/plans/[pid]`,
> `/plans/[pid]/{edit,board,schedule,governance}`) with list + name-search
> + create + detail + edit + delete + check-duplicates, the `PlanForm`,
> the `PlanRepository`, the lean `ApiClient`, the BFF proxy +
> cookie-session auth, and the 13-locale i18n + top-bar hamburger +
> theme/locale selectors. **Deferred** (roadmap): the rich project-
> management views (Kanban board, issues, Gantt/timeline, burndown, goals
> panels), the MatchBreakdown visual, and the merge/audit-timeline UI — the
> service defers those sub-resources too.

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

## Layout (planned `src/` tree)

```
src/
├── lib/
│   ├── config.ts                 PUBLIC_API_BASE_URL (:5150) + VITE_AUTH_FRONTEND_URL (:5173) + signInUrl()
│   ├── auth.svelte.ts            reactive session-state store (signed-in flag from the httpOnly cookie); no token in JS
│   ├── i18n/                     13-locale catalogues (en default) + RTL flags + en fallback
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (get/post/put/patch/delete + ApiError); credentials: 'include' (cookie) + CSRF header
│   │   ├── types.ts              Plan + PlanKind + PlanStatus + Goal + Task + Issue (+ IssueKind/IssueSeverity/IssueStatus) + Relationship + RelationKind + PlanIdentifier + IdentifierScheme + PlanRef + ScoredRef + MatchBreakdown + MergeResult + AuditEntry + PlanEvent + TimelineRow + BurndownPoint
│   │   ├── capabilities.ts       CapabilityClient (reviews, assignees, notifications, automations, scheduled actions, Smart Score, lifecycle)
│   │   └── plans.ts              PlanRepository (CRUD + search + checkDuplicates + merge + audit + recentEvents + parent roll-up + sub-resources + timeline + burndown; all paths under /api/plans)
│   └── components/               PlanForm, MatchBreakdown, KanbanBoard, IssuesList, Timeline, Burndown, GoalsPanel, pickers
└── routes/
    ├── +layout.svelte / +layout.ts   top-bar nav (leftmost hamburger) + Plans destination + theme/locale selectors + session affordance + SPA toggle
    ├── +page.svelte              landing (links to /plans)
    └── plans/
        ├── +page.svelte         list (SVAR DataGrid) + name-search + recent-activity toggle
        ├── new/+page.svelte     create
        ├── [pid]/+page.svelte   detail + delete + check-duplicates + MatchBreakdown + audit timeline (+ child roll-up)
        ├── [pid]/edit/+page.svelte   edit
        ├── [pid]/{board,schedule,governance}/+page.svelte
        └── merge/+page.svelte   merge a duplicate into a survivor + recent merge history
```

## API consumption

Every plan lives under `/api/plans` (one recursive collection).

| UI action | Endpoint |
|---|---|
| List | `GET /api/plans` |
| Search | `GET /api/plans/search?q=` |
| Recent activity | `GET /api/plans/events/recent` → `PlanEvent[]` |
| Create | `POST /api/plans` |
| Detail | `GET /api/plans/{pid}` |
| Edit | `PUT /api/plans/{pid}` |
| Delete | `DELETE /api/plans/{pid}` |
| Check duplicates | `POST /api/plans/check-duplicates` → `ScoredRef[]` w/ `MatchBreakdown` |
| Merge duplicate | `POST /api/plans/merge` (body `{main_pid, duplicate_pid, reason?}`) |
| Audit timeline | `GET /api/plans/{pid}/audit` → `AuditEntry[]` |
| Child roll-up | `GET /api/plans?parent={pid}` |
| Schedule | `GET /api/plans/{pid}/schedule` |
| Tasks (board) | `GET / POST /api/plans/{pid}/tasks` · `PUT / PATCH /api/plans/{pid}/tasks/{tid}` (PATCH = status move) |
| Issues | `GET / POST /api/plans/{pid}/issues` · `PUT /api/plans/{pid}/issues/{iid}` |
| Goals | `GET / POST /api/plans/{pid}/goals` · `PUT / DELETE /api/plans/{pid}/goals/{gid}` |
| Timeline | `GET /api/plans/{pid}/timeline` → `TimelineRow[]` |
| Burndown | `GET /api/plans/{pid}/burndown` → `BurndownPoint[]` |
| Collaborative review | `POST / GET /api/reviews` · `/{pid}/respond` · `/{pid}/submit` · `GET /api/reviews/consensus` |
| Assign a task | `POST /api/plans/{pid}/tasks/{t_pid}/assign` (`null` unassigns) |
| Assignee workload | `GET /api/assignees/workload` |
| Notifications | `GET /api/notifications` · `POST /api/notifications/{pid}/read` |
| Automations | `POST / GET /api/automations` · `/{pid}/enable`·`/disable` · `GET /api/automations/runs` |
| Scheduled actions | `POST / GET /api/scheduled-actions` · `POST /api/scheduled-actions/sweep` |
| Smart Score | `GET /api/plans/{pid}/smart-score` · `GET /api/prioritisation` |
| Lifecycle | `GET /api/lifecycle` · `GET /api/plans/{pid}/lifecycle` |

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

The intended model is a **Backend-For-Frontend**: this app's own
SvelteKit server holds the session and talks to the portfolio service;
the browser holds **no token** (no `localStorage`, no `mxi_access_token`,
no URL-fragment handoff). The portfolio service's blanket auth
enforcement (`PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`, off by default) is satisfied by the
BFF presenting a short-lived **PASETO v4.public** bearer server-side.
Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256/JWKS not used).

- **Sign in** — the top bar leads with **Sign in**, redirecting to
  `${VITE_AUTH_FRONTEND_URL}/signin?return_to=<origin + base>`; the
  magic-link establishes a server-side session and sets an httpOnly
  `__Host-mxi_session` cookie.
- **`$lib/auth.svelte`** — reactive **session-state** store (a signed-in
  flag derived from the cookie/BFF), not a token store. Browser JS never
  reads a credential.
- **BFF** — the SvelteKit server (`hooks.server.ts` / `+server.ts` /
  `+page.server.ts`) holds the session, exchanges it for a short-lived
  PASETO v4.public token, and calls the portfolio service server-side
  with that bearer.
- **`ApiClient`** — browser→BFF calls send the cookie
  (`credentials: 'include'`); state-changing requests carry a **CSRF
  token** (`X-CSRF-Token`). Safe `GET`/`HEAD` are CSRF-exempt.

Configure with `PUBLIC_API_BASE_URL` and `VITE_AUTH_FRONTEND_URL` (see
`.env.example`).

## What does NOT live here

- FHIR Plan UI — the service has no FHIR surface.
- Consent-management UI — out of scope for MVP.
- Finance / budgeting UI — not a finance system (entity spec §1.3).
- Posts feed / threaded comments — not part of the v1 portfolio
  sub-resource set (tasks / goals / issues only); roadmap-only.
- Members / role panel — not part of the v1 sub-resource set;
  roadmap-only.
- A login screen — sign-on is delegated to the central
  authentication-service magic-link SSO; this app only establishes /
  carries the cookie session via its BFF (no browser-held token).
