# project-portfolio-management-front-end-with-svelte — documentation index

Operator UI for **plan** identity CRUD + matching + name search + merge +
audit timeline over **one recursive `/api/plans` collection**, plus the
**project-management workspace** (Kanban board, issues, timeline / Gantt,
burndown, goals) and cookie-session / SSO auth (BFF; the browser holds no
token), consuming the [Portfolio Service](../project-portfolio-management-service-with-loco).

> **Status: implemented (v0.1.0).** `npm run check` is 0/0 and the vitest
> suite passes; the build queue is [spec/index.md](./spec/index.md) §13.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, planned `src/` tree, API consumption map, layout shell. |
| [README.md](./README.md) | Plans, routes, layout / chrome, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |
| [../spec/index.md](../spec/index.md) | Entity umbrella spec (cross-subproject contract). |

## Plans

One recursive collection: every record is a **plan**, all under
`/api/plans`. `kind` (Portfolio / Project / Product / Program / Practice
/ Process / Purpose / Pathway / Proposal) is an **optional descriptive
label** — it does not gate matching or select a collection. Any plan may
contain any other via a `parent_ref`, and `GET /api/plans?parent={pid}`
rolls up a plan's direct children.

## Flow

```text
/plans          ──>  GET  /api/plans                       list (SVAR DataGrid)
                     GET  /api/plans/search?q=migration     name search
                     GET  /api/plans/events/recent          recent activity -> PlanEvent[]
/plans/new      ──>  POST /api/plans  {Plan}                create -> /plans/[pid]
/plans/[pid]    ──>  GET  /api/plans/{pid}                  detail
                     POST /api/plans/check-duplicates       -> ScoredRef[] w/ MatchBreakdown
                     POST /api/plans/merge  {main_pid, duplicate_pid, reason?}  merge -> MergeResult
                     GET  /api/plans/{pid}/audit            audit timeline -> AuditEntry[]
                     DELETE /api/plans/{pid}               soft-delete
                     GET  /api/plans?parent={pid}          child roll-up
/plans/[pid]/edit ─> PUT /api/plans/{pid}                  edit

project-management workspace (sub-resources under /api/plans/{pid}/…):
…/board    ──>  GET/POST …/tasks · PATCH …/tasks/{tid}    Kanban; drag = status change
…/issues   ──>  GET/POST …/issues · PUT …/issues/{iid}    issues (kind/severity/status)
…/timeline ──>  GET …/timeline    -> TimelineRow[]        Gantt (milestones + task ranges)
…/burndown ──>  GET …/burndown    -> BurndownPoint[]      remaining estimate over time
…/goals    ──>  GET/POST/PUT/DELETE …/goals              goals panel
```

## Layout & chrome

Global navigation is a full-width **top bar** with a **leftmost
hamburger** toggle (no left sidebar; content is full-width). The nav has
a single **Plans** destination (no collection switcher). The chrome area
carries a **theme selector** (full shared catalogue — selecting a
theme restyles the whole site), a **locale selector** (13 locales;
selecting one switches the language; `ar` / `ur` are RTL), and the
session affordance.

## Auth

Cookie session via a BFF: **Sign in**
(`${VITE_AUTH_FRONTEND_URL}/signin?return_to=<origin + base>`) runs the
magic-link, which establishes a server-side session and sets an httpOnly
`__Host-mxi_session` cookie. The browser holds **no token** (no
`localStorage`, no URL fragment). This app's SvelteKit server acts as a
**Backend-For-Frontend**: it holds the session, exchanges it for a
short-lived **PASETO v4.public** token, and calls the portfolio service
server-side; mutating browser→BFF calls carry a CSRF token. Source of
truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256/JWKS not used).
