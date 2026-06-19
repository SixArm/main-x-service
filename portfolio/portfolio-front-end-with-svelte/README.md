# portfolio-front-end-with-svelte

Operator UI for the [Portfolio Service](../portfolio-service-with-loco):
work-item **identity CRUD + matching + name search + merge + audit
timeline** across **four matchable collections** (portfolios, projects,
products, programs), plus the **project-management workspace** (Kanban
task board, issues, Gantt / timeline, burndown, goals).

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA · SVAR Svelte
DataGrid · Lily Design System Svelte Headless · 13-locale i18n · full
theme catalogue.

> **Status: spec-only (v0.1.0).** This project is documentation first —
> there is no `src/` yet. The build queue is [`spec/index.md`](spec/index.md) §13.

## Collections

The entity has **four matchable work-item kinds**, each its own REST
collection / list / CRUD; matching is **within a collection only** (a
project never matches a product). A **Portfolio** is the umbrella;
**Project** / **Product** / **Program** carry a `portfolio_ref` to their
parent portfolio, and a portfolio detail page rolls up its children.

| Collection | Endpoint base | Kind |
|---|---|---|
| Portfolios | `/api/v1/portfolios` | `Portfolio` (umbrella) |
| Projects | `/api/v1/projects` | `Project` (child) |
| Products | `/api/v1/products` | `Product` (child) |
| Programs | `/api/v1/programs` | `Program` (child) |

## Routes

`{collection} ∈ { portfolios, projects, products, programs }`.

| Route | Purpose |
|---|---|
| `/` | Collection switcher (defaults to `/portfolios`) |
| `/{collection}` | List work items (SVAR DataGrid) + name-search box + recent-activity toggle |
| `/{collection}/new` | Create |
| `/{collection}/[pid]` | Detail + delete + check-duplicates + MatchBreakdown + merge + audit timeline (portfolio detail also rolls up its child work items) |
| `/{collection}/[pid]/edit` | Edit |
| `/{collection}/[pid]/board` | Kanban task board (Todo / InProgress / InReview / Done / Blocked; drag = status change) |
| `/{collection}/[pid]/issues` | Issues list (kind / severity / status) |
| `/{collection}/[pid]/timeline` | Gantt / timeline (goal milestones + task date ranges) |
| `/{collection}/[pid]/burndown` | Burndown chart (remaining estimate over time) |
| `/{collection}/[pid]/goals` | Goals panel |

(The collection switcher and the project-management views may ship as
top-bar controls / detail-page tabs rather than discrete routes; the spec
fixes the capabilities, not the URLs.)

## Layout & chrome

Global navigation is a full-width **top navigation bar** with a
**leftmost hamburger** toggle (no left sidebar; main content is
full-width). The chrome area carries:

- a **collection switcher** (portfolios / projects / products /
  programs);
- a **theme selector** (`lily-design-system-svelte-theme-select`) —
  selecting a theme restyles the whole site (full shared catalogue);
- a **locale selector** (`lily-design-system-svelte-locale-select`) —
  13 locales (`en`, `cy`, `es`, `fr`, `de`, `ar`, `ru`, `hi`, `zh`,
  `bn`, `pt`, `id`, `ur`); selecting one switches the language; `ar` /
  `ur` render RTL;
- a session affordance: **Sign in** redirects to the central
  authentication front-end (SSO); the magic-link establishes a
  server-side session and sets an httpOnly cookie. The browser holds no
  token — this app's SvelteKit server acts as a BFF, exchanging the
  session for a short-lived PASETO v4.public token to call the service
  server-side. See
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (RS256/JWKS not used).

## Prerequisites

- Node 20+ and pnpm
- A running [Portfolio Service](../portfolio-service-with-loco)
- The sibling Lily helper repo (theme / locale selectors are `file:`
  dependencies)

## Quick start (once scaffolded)

```bash
cp .env.example .env     # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                 # http://localhost:5173
```

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `PUBLIC_API_BASE_URL` | `http://localhost:5150` | Portfolio service REST base URL (`/api/v1/{collection}/...`). |
| `VITE_AUTH_FRONTEND_URL` | `http://localhost:5173` | Central authentication front-end base URL. "Sign in" redirects to `${VITE_AUTH_FRONTEND_URL}/signin?return_to=…`; the magic-link establishes a server-side session and sets an httpOnly cookie — the browser holds no token, and this app's BFF supplies a short-lived PASETO v4.public bearer server-side (see `agents/share/authentication-sessions.md`; RS256/JWKS not used). |

## How it works

The work-item record body **is** the `portfolio_matcher::WorkItem` shape
(required `kind`, name, code, owner org, status, timeframe, goals,
keywords, tags, relationships, identifiers, sameAs, and — for child kinds
— `portfolio_ref`). The form edits these; `check-duplicates` posts the
current record and lists stored matches **within its collection** with
their scores and a per-component **MatchBreakdown** (name, goals, code,
owner org, portfolio, timeframe, keywords, relationships, tags — `kind`
is a hard match gate, not a scored component). The detail page offers a
per-row **Merge into this record** action (`POST …/merge`) and a
per-work-item audit timeline (`GET …/{pid}/audit`). A portfolio detail
page additionally rolls up its child projects / products / programs.

A work item is also a **project-management workspace**: its operational
sub-resources (goals, tasks, issues) live in the service under
`/api/v1/{collection}/{pid}/…` and are **not** part of the matching
surface (except goal titles). The board / issues / timeline / burndown /
goals views consume those endpoints.

## Testing (once scaffolded)

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm run build
pnpm test          # vitest unit suite
pnpm test:e2e      # Playwright smoke (runs against `vite preview`)
```

## License

Dual-licensed under MIT OR Apache-2.0.
