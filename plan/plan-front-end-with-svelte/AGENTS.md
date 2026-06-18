# AGENTS.md — Plan Front-End

Operator UI for the [Plan Service](../plan-service-with-loco): plan
identity CRUD + matching + name search + merge + audit timeline, plus
the project-management workspace (Kanban board, issues, timeline /
Gantt, burndown, goals, posts + comments, members) and cookie-session /
SSO auth (BFF; the browser holds no token).

> Read [`spec/index.md`](./spec/index.md) first — the living spec
> (§1–§18). Entity umbrella: [`../spec/index.md`](../spec/index.md).
>
> **Status: spec-only (v0.1.0).** No `src/` yet — the build queue is
> spec §13.

## What this is

A SvelteKit 2 / Svelte 5 (runes) **SPA**. It calls the plan service
REST API under `/api/v1/plans/...`, whose request/response body for the
identity record **is** the `plan_matcher::Plan` shape itself (the API
DTO = the matcher type, persisted as JSONB). The operational
sub-resources (goals, tasks, issues, posts, comments, members) hang off
a plan under `/api/v1/plans/{pid}/…` and are **not** part of the
matching surface (except goal titles).

## Single source of truth

- The service's [`spec.md`](../plan-service-with-loco/spec/index.md) and
  [`AGENTS/`](../AGENTS/) describe the API contract. If a field changes
  in `Plan` (the matcher type) in the service, fix
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
4. **SVAR DataGrid** for tabular data (the plan list, sub-resource
   tables). Native HTML for simple lists.
5. **Lily Headless** for accessibility primitives (focus trap,
   listbox, combobox, dialog) and the theme / locale selectors. Native
   HTML elsewhere.
6. **No global stores** for HTTP state — construct a `PlanRepository`
   per page/component.
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
- The chrome area carries the **theme selector**
  (`lily-design-system-svelte-theme-select` — full shared catalogue;
  selecting a theme restyles the whole site; persists to
  `localStorage["plan:theme"]`), the **locale selector**
  (`lily-design-system-svelte-locale-select` — 13 locales; selecting
  one switches the language; `ar` / `ur` are RTL; persists to
  `localStorage["plan:locale"]`), and the session affordance (Sign in /
  Sign out; the browser holds no token — see **Auth** below).

## Layout (planned `src/` tree)

```
src/
├── lib/
│   ├── config.ts                 PUBLIC_API_BASE_URL (:5150) + VITE_AUTH_FRONTEND_URL (:5173) + signInUrl()
│   ├── auth.svelte.ts            reactive session-state store (signed-in flag from the httpOnly cookie); no token in JS
│   ├── i18n/                     13-locale catalogues (en default) + RTL flags + en fallback
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (get/post/put/patch/delete + ApiError); credentials: 'include' (cookie) + CSRF header
│   │   ├── types.ts              Plan + PlanType + PlanStatus + Goal + Task + Issue + Post + Comment + Member + Relationship + IdentifierScheme + PlanRef + ScoredRef + MatchBreakdown + MergeResult + AuditEntry + PlanEvent + TimelineRow + BurndownPoint
│   │   └── plans.ts              PlanRepository (CRUD + search + checkDuplicates + merge + audit + recentEvents + sub-resources + timeline + burndown)
│   └── components/               PlanForm, MatchBreakdown, KanbanBoard, IssuesList, Timeline, Burndown, GoalsPanel, PostsFeed, MembersPanel, pickers
└── routes/
    ├── +layout.svelte / +layout.ts   top-bar nav (leftmost hamburger) + theme/locale selectors + session affordance + SPA toggle
    ├── +page.svelte              list (SVAR DataGrid) + name-search + recent-activity toggle
    ├── new/+page.svelte          create
    ├── [pid]/+page.svelte        detail + delete + check-duplicates + MatchBreakdown + merge + audit timeline
    ├── [pid]/edit/+page.svelte   edit
    └── [pid]/{board,issues,timeline,burndown,goals,posts,members}/+page.svelte
```

## API consumption

| UI action | Endpoint |
|---|---|
| List | `GET /api/v1/plans` |
| Search | `GET /api/v1/plans/search?q=` |
| Recent activity | `GET /api/v1/plans/events/recent` → `PlanEvent[]` |
| Create | `POST /api/v1/plans` |
| Detail | `GET /api/v1/plans/{pid}` |
| Edit | `PUT /api/v1/plans/{pid}` |
| Delete | `DELETE /api/v1/plans/{pid}` |
| Check duplicates | `POST /api/v1/plans/check-duplicates` → `ScoredRef[]` w/ `MatchBreakdown` |
| Merge duplicate | `POST /api/v1/plans/merge` (body `{main_pid, duplicate_pid, reason?}`) |
| Audit timeline | `GET /api/v1/plans/{pid}/audit` → `AuditEntry[]` |
| Tasks (board) | `GET / POST …/{pid}/tasks` · `PUT / PATCH …/{pid}/tasks/{tid}` (PATCH = status move) |
| Issues | `GET / POST …/{pid}/issues` · `PUT …/{pid}/issues/{iid}` |
| Goals | `GET / POST …/{pid}/goals` · `PUT / DELETE …/{pid}/goals/{gid}` |
| Posts / comments | `GET / POST …/{pid}/posts` · `POST …/{pid}/posts/{poid}/comments` |
| Members | `GET / POST …/{pid}/members` · `PATCH / DELETE …/{pid}/members/{mid}` |
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
SvelteKit server holds the session and talks to the plan service; the
browser holds **no token** (no `localStorage`, no `mxi_access_token`, no
URL-fragment handoff). The plan service's blanket auth enforcement
(`PLAN_REQUIRE_AUTH`, off by default) is satisfied by the BFF presenting
a short-lived **PASETO v4.public** bearer server-side. Source of truth:
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
  PASETO v4.public token, and calls the plan service server-side with
  that bearer.
- **`ApiClient`** — browser→BFF calls send the cookie
  (`credentials: 'include'`); state-changing requests carry a **CSRF
  token** (`X-CSRF-Token`). Safe `GET`/`HEAD` are CSRF-exempt.

Configure with `PUBLIC_API_BASE_URL` and `VITE_AUTH_FRONTEND_URL` (see
`.env.example`).

## What does NOT live here

- FHIR Plan UI — the service has no FHIR surface.
- Consent-management UI — out of scope for MVP.
- Finance / budgeting UI — not a finance system (entity spec §1.3).
- Binary-attachment upload — posts / comments are Markdown text only.
- A login screen — sign-on is delegated to the central
  authentication-service magic-link SSO; this app only establishes /
  carries the cookie session via its BFF (no browser-held token).
