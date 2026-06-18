# plan-front-end-with-svelte

Operator UI for the [Plan Service](../plan-service-with-loco): plan
**identity CRUD + matching + name search + merge + audit timeline**,
plus the **project-management workspace** (Kanban task board, issues,
Gantt / timeline, burndown, goals, posts + threaded comments, members).

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA · SVAR Svelte
DataGrid · Lily Design System Svelte Headless · 13-locale i18n · full
theme catalogue.

> **Status: spec-only (v0.1.0).** This project is documentation first —
> there is no `src/` yet. The build queue is [`spec/index.md`](spec/index.md) §13.

## Routes

| Route | Purpose |
|---|---|
| `/` | List plans (SVAR DataGrid) + name-search box + recent-activity toggle |
| `/new` | Create |
| `/[pid]` | Detail + delete + check-duplicates + MatchBreakdown + merge + audit timeline |
| `/[pid]/edit` | Edit |
| `/[pid]/board` | Kanban task board (Todo / InProgress / InReview / Done / Blocked; drag = status change) |
| `/[pid]/issues` | Issues list (kind / severity / status) |
| `/[pid]/timeline` | Gantt / timeline (goal milestones + task date ranges) |
| `/[pid]/burndown` | Burndown chart (remaining estimate over time) |
| `/[pid]/goals` | Goals panel |
| `/[pid]/posts` | Posts feed + threaded comments |
| `/[pid]/members` | Members panel (role management) |

(Project-management views may ship as detail-page tabs rather than
discrete sub-routes; the spec fixes the capabilities, not the URLs.)

## Layout & chrome

Global navigation is a full-width **top navigation bar** with a
**leftmost hamburger** toggle (no left sidebar; main content is
full-width). The chrome area carries:

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
- A running [Plan Service](../plan-service-with-loco)
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
| `PUBLIC_API_BASE_URL` | `http://localhost:5150` | Plan service REST base URL (`/api/v1/plans/...`). |
| `VITE_AUTH_FRONTEND_URL` | `http://localhost:5173` | Central authentication front-end base URL. "Sign in" redirects to `${VITE_AUTH_FRONTEND_URL}/signin?return_to=…`; the magic-link establishes a server-side session and sets an httpOnly cookie — the browser holds no token, and this app's BFF supplies a short-lived PASETO v4.public bearer server-side (see `agents/share/authentication-sessions.md`; RS256/JWKS not used). |

## How it works

The plan record body **is** the `plan_matcher::Plan` shape (name, plan
code, owner org, plan type / status, timeframe, goals, keywords, tags,
relationships, identifiers, sameAs). The form edits these;
`check-duplicates` posts the current record and lists stored matches
with their scores and a per-component **MatchBreakdown** (name, goals,
plan code, owner org, plan type, timeframe, keywords, relationships,
tags). The detail page offers a per-row **Merge into this record**
action (`POST /merge`) and a per-plan audit timeline
(`GET /{pid}/audit`).

A plan is also a **project-management workspace**: its operational
sub-resources (goals, tasks, issues, posts, comments, members) live in
the service under `/api/v1/plans/{pid}/…` and are **not** part of the
matching surface (except goal titles). The board / issues / timeline /
burndown / goals / posts / members views consume those endpoints.

## Testing (once scaffolded)

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm run build
pnpm test          # vitest unit suite
pnpm test:e2e      # Playwright smoke (runs against `vite preview`)
```

## License

Dual-licensed under MIT OR Apache-2.0.
