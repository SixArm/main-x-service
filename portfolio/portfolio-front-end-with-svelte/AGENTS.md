# AGENTS.md — Portfolio Front-End

Operator UI for the [Portfolio Service](../portfolio-service-with-loco):
work-item identity CRUD + matching + name search + merge + audit timeline
across **four matchable collections** (portfolios, projects, products,
programs), plus the project-management workspace (Kanban board, issues,
timeline / Gantt, burndown, goals) and cookie-session / SSO auth (BFF; the
browser holds no token).

> Read [`spec/index.md`](./spec/index.md) first — the living spec
> (§1–§18). Entity umbrella: [`../spec/index.md`](../spec/index.md).
>
> **Status: implemented (MVP, v0.1.0).** `pnpm install` works, `pnpm run
> check` is **0 errors / 0 warnings**, `pnpm test` (vitest) is green (41
> tests), and `pnpm run build` succeeds. Shipped: the four-collection
> identity surface — a `[collection]` route layer (`/portfolios`,
> `/projects`, `/products`, `/programs`) with list + name-search + create +
> detail + edit + delete + check-duplicates, the `WorkItemForm`, the
> collection-bound `WorkItemRepository`, the lean `ApiClient`, the BFF
> proxy + cookie-session auth, and the 13-locale i18n + top-bar hamburger +
> theme/locale selectors. **Deferred** (roadmap): the rich project-
> management views (Kanban board, issues, Gantt/timeline, burndown, goals
> panels), the MatchBreakdown visual, and the merge/audit-timeline UI — the
> service defers those sub-resources too.

## What this is

A SvelteKit 2 / Svelte 5 (runes) **SPA**. It calls the portfolio service
REST API under `/api/v1/{portfolios,projects,products,programs}/...`,
whose request/response body for the identity record **is** the
`portfolio_matcher::WorkItem` shape itself (the API DTO = the matcher
type, persisted as JSONB). There are **four matchable collections** — one
per `WorkItemKind` (Portfolio / Project / Program / Product) — each with
the identical controller shape; matching is **within a collection only**
(the matcher gates on `kind`). A Portfolio is the umbrella; Project /
Product / Program carry a `portfolio_ref` to their parent. The
operational sub-resources (goals, tasks, issues) hang off a work item
under `/api/v1/{collection}/{pid}/…` and are **not** part of the matching
surface (except goal titles).

## Single source of truth

- The service's [`spec.md`](../portfolio-service-with-loco/spec/index.md)
  and [`AGENTS/`](../AGENTS/) describe the API contract. If a field
  changes in `WorkItem` (the matcher type) in the service, fix
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
4. **SVAR DataGrid** for tabular data (each collection list, sub-resource
   tables). Native HTML for simple lists.
5. **Lily Headless** for accessibility primitives (focus trap,
   listbox, combobox, dialog) and the theme / locale selectors. Native
   HTML elsewhere.
6. **No global stores** for HTTP state — construct a
   `WorkItemRepository` (bound to a collection) per page/component.
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
- The chrome area carries the **collection switcher** (portfolios /
  projects / products / programs), the **theme selector**
  (`lily-design-system-svelte-theme-select` — full shared catalogue;
  selecting a theme restyles the whole site; persists to
  `localStorage["portfolio:theme"]`), the **locale selector**
  (`lily-design-system-svelte-locale-select` — 13 locales; selecting
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
│   │   ├── types.ts              WorkItem + WorkItemKind + WorkItemStatus + Goal + Task + Issue (+ IssueKind/IssueSeverity/IssueStatus) + Relationship + RelationKind + WorkItemIdentifier + IdentifierScheme + WorkItemRef + ScoredRef + MatchBreakdown + MergeResult + AuditEntry + WorkItemEvent + TimelineRow + BurndownPoint
│   │   └── work-items.ts         WorkItemRepository (collection-bound: CRUD + search + checkDuplicates + merge + audit + recentEvents + roll-up + sub-resources + timeline + burndown)
│   └── components/               WorkItemForm, MatchBreakdown, KanbanBoard, IssuesList, Timeline, Burndown, GoalsPanel, pickers
└── routes/
    ├── +layout.svelte / +layout.ts   top-bar nav (leftmost hamburger) + collection switcher + theme/locale selectors + session affordance + SPA toggle
    ├── +page.svelte              collection switcher (defaults to /portfolios)
    └── [collection]/
        ├── +page.svelte         list (SVAR DataGrid) + name-search + recent-activity toggle
        ├── new/+page.svelte     create
        ├── [pid]/+page.svelte   detail + delete + check-duplicates + MatchBreakdown + merge + audit timeline (+ portfolio roll-up)
        ├── [pid]/edit/+page.svelte   edit
        └── [pid]/{board,issues,timeline,burndown,goals}/+page.svelte
```

## API consumption

`{collection} ∈ { portfolios, projects, products, programs }`.

| UI action | Endpoint |
|---|---|
| List | `GET /api/v1/{collection}` |
| Search | `GET /api/v1/{collection}/search?q=` |
| Recent activity | `GET /api/v1/{collection}/events/recent` → `WorkItemEvent[]` |
| Create | `POST /api/v1/{collection}` |
| Detail | `GET /api/v1/{collection}/{pid}` |
| Edit | `PUT /api/v1/{collection}/{pid}` |
| Delete | `DELETE /api/v1/{collection}/{pid}` |
| Check duplicates | `POST /api/v1/{collection}/check-duplicates` → `ScoredRef[]` w/ `MatchBreakdown` |
| Merge duplicate | `POST /api/v1/{collection}/merge` (body `{main_pid, duplicate_pid, reason?}`) |
| Audit timeline | `GET /api/v1/{collection}/{pid}/audit` → `AuditEntry[]` |
| Portfolio roll-up | `GET /api/v1/{projects,products,programs}?portfolio_ref={pid}` |
| Tasks (board) | `GET / POST …/{pid}/tasks` · `PUT / PATCH …/{pid}/tasks/{tid}` (PATCH = status move) |
| Issues | `GET / POST …/{pid}/issues` · `PUT …/{pid}/issues/{iid}` |
| Goals | `GET / POST …/{pid}/goals` · `PUT / DELETE …/{pid}/goals/{gid}` |
| Timeline | `GET …/{pid}/timeline` → `TimelineRow[]` |
| Burndown | `GET …/{pid}/burndown` → `BurndownPoint[]` |

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
enforcement (`PORTFOLIO_REQUIRE_AUTH`, off by default) is satisfied by the
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

- FHIR WorkItem UI — the service has no FHIR surface.
- Consent-management UI — out of scope for MVP.
- Finance / budgeting UI — not a finance system (entity spec §1.3).
- Posts feed / threaded comments — not part of the v1 portfolio
  sub-resource set (tasks / goals / issues only); roadmap-only.
- Members / role panel — not part of the v1 sub-resource set;
  roadmap-only.
- A login screen — sign-on is delegated to the central
  authentication-service magic-link SSO; this app only establishes /
  carries the cookie session via its BFF (no browser-held token).
