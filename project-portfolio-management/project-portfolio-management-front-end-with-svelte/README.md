# project-portfolio-management-front-end-with-svelte

Operator UI for the [Portfolio Service](../project-portfolio-management-service-with-loco):
plan **identity CRUD + matching + merge** over **one recursive
`/api/plans` collection**, plus the **project-management workspace**
(Kanban task board, governance, schedule) and a wide set of oversight /
executive views.

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA · SVAR Svelte
DataGrid · Lily Design System Svelte Headless · 13-locale i18n · full
theme catalogue.

> **Status: implemented (MVP, v0.1.0).** `pnpm run check` is 0 errors /
> 0 warnings, the vitest suite is green, and `pnpm run build` succeeds.
> The live work queue is [`spec/index.md`](spec/index.md) §13.

## Plans

The entity is a single recursive **plan** collection: every record is a
plan under `/api/plans`, with one REST collection / list / CRUD, and
matching runs across the whole collection (it is **not** gated by kind).
A plan carries an **optional** `kind` descriptive label (Portfolio /
Project / Product / Program / Practice / Process / Purpose / Pathway /
Proposal) and an optional `parent_ref` to another plan
— any plan may contain any other, so a plan detail page rolls up its
children.

| Concept | Value |
|---|---|
| Endpoint base | `/api/plans` |
| Record type | `Plan` (matcher DTO as JSONB) |
| `kind` | optional label: `Portfolio` / `Project` / `Product` / `Program` / `Practice` / `Process` / `Purpose` / `Pathway` / `Proposal` |
| `parent_ref` | optional pid of the containing plan |

## Routes

Every plan route lives under the static `plans/` directory.

| Route | Purpose |
|---|---|
| `/` | Landing — links into Plans |
| `/dashboard` | PPM dashboard — site tiles + RAG / stage rollups |
| `/plans` | Plan index (SVAR DataGrid + client-side FilterBar; row selection opens the detail) |
| `/plans/new` | Create |
| `/plans/[pid]` | Detail + edit / delete / check-duplicates |
| `/plans/[pid]/edit` | Edit |
| `/plans/merge` | Fold a confirmed-duplicate plan into a survivor, with an optional preview and a recent-merge history table |
| `/plans/[pid]/board` | Per-plan task Kanban + sprints + honest burndown + standup digest |
| `/plans/[pid]/governance` | Governance panel — stage / risk posture / budget variance, gate journey, risks, budget lines, benefits + ROI, OKR mappings, milestones, allocations |
| `/plans/[pid]/schedule` | Plan schedule — child timeframes, critical-path badges, finish-start violations |
| `/executive` | CEO area: portfolio-health briefing (server-derived RAG), decision log, benefits realization |
| `/financials` | CFO area: budget variance (category / kind / plan) + per-currency exposure — minor units, no FX |
| `/technology` | CTO area: technology radar (`tech:` tags) + dependency-risk lens (fan-out, red predecessors) |
| `/gantt` | Schedule Gantt (SVAR) — the selected plan's dated child plans as task bars, dependency edges as links, critical path highlighted (read-only) |
| `/capacity` | Resource capacity — per-person rollup over a window; over-allocation flagged |
| `/ideas` | Idea board — capture, vote, dismiss, convert to a draft proposal |
| `/objectives` | OKR objectives registry + per-objective alignment rollups |
| `/proposals` | Work-intake board — proposal pipeline (draft → … → promoted) + duplicate-demand checks + promote-to-plan |
| `/reports` | Saved reports — definitions, synchronous runs, CSV download |
| `/scenarios` | Scenario planning — what-if candidate portfolios, evaluate, commit |
| `/signin` | Magic-link sign-in (BFF flow) |
| `/verify` | Magic-link verification landing page |

## Layout & chrome

Global navigation is a full-width **top navigation bar** with a
**leftmost hamburger** toggle (no left sidebar; main content is
full-width). The nav has a single **Plans** destination (no collection
switcher). The chrome area carries:

- a **theme selector** (`lily-design-system-svelte-theme-picker`) —
  selecting a theme restyles the whole site (full shared catalogue);
- a **locale selector** (`lily-design-system-svelte-locale-picker`) —
  13 locales (`en`, `cy`, `es`, `fr`, `de`, `ar`, `ru`, `hi`, `zh`,
  `bn`, `pt`, `id`, `ur`); selecting one switches the language; `ar` /
  `ur` render RTL;
- a session affordance: **Sign in** leads to this app's own `/signin` +
  `/verify` magic-link pages (BFF), which establish a server-side
  session and set an httpOnly `__Host-mxi_session` cookie. The browser
  holds no token — this app's SvelteKit server acts as a BFF, exchanging
  the session for a short-lived PASETO v4.public token and calling the
  service through the same-origin `/api/proxy` route. See
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (RS256/JWKS not used).

## Prerequisites

- Node 20+ and pnpm
- A running [Portfolio Service](../project-portfolio-management-service-with-loco)
- The sibling Lily helper repo (theme / locale selectors are `file:`
  dependencies)

## Quick start

```bash
cp .env.example .env     # PROJECT_PORTFOLIO_MANAGEMENT_API_URL=http://localhost:5150
pnpm install
pnpm dev                 # http://localhost:5173
```

## Configuration

Both vars are read **server-side only** (`src/lib/server/config.ts`) by
the BFF proxy and the auth exchange — the browser never sees them.

| Var | Default | Purpose |
|---|---|---|
| `PROJECT_PORTFOLIO_MANAGEMENT_API_URL` | `http://localhost:5150` | Portfolio service REST base URL, forwarded to by `src/routes/api/proxy/[...path]`. |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication service base URL (BFF-side magic-link + session→PASETO exchange, `src/lib/server/auth.ts`). |

## How it works

The plan record body **is** the `project_portfolio_management_matcher::Plan`
shape (name, code, owner org, status, timeframe, goals, keywords, tags,
relationships, identifiers, sameAs, an **optional** `kind` label, and an
optional `parent_ref`). The form edits these; `check-duplicates` posts the
current record and lists stored matches **across the whole collection**
with their score and confidence **across the whole collection** (matching
is **not** gated by `kind`). The standalone `/plans/merge` page folds a
confirmed duplicate into a survivor (`POST /api/plans/merge`) and shows a
recent-merge history table (`GET /api/plans/merges/recent`).

**Not yet built**, per [`spec/index.md`](spec/index.md) §13: a
per-plan audit timeline (`GET /api/plans/{pid}/audit`), a recent-activity
feed (`GET /api/plans/events/recent`), a per-component match-score
breakdown visual, and a child-plan roll-up on the detail page
(`GET /api/plans?parent={pid}`) — none of these has a repository method,
a type, or a UI element yet. The one partial exception is name search:
`PlanRepository.search()` wraps `GET /api/plans/search?q=` and is unit-
tested, but no route calls it — the list page's search box filters
client-side over the already-loaded rows instead.

A plan is also a **project-management workspace**: its operational
sub-resources (goals, tasks, issues) live in the service under
`/api/plans/{pid}/…` and are **not** part of the matching surface (except
goal titles). The governance and schedule views (and the top-level Gantt /
dashboard / capacity views) consume those and the PPM endpoints.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm run build
pnpm test          # vitest unit suite
pnpm test:e2e      # Playwright smoke (runs against `vite preview`)
```

## License

Dual-licensed under MIT OR Apache-2.0.
