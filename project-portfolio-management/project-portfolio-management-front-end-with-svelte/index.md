# project-portfolio-management-front-end-with-svelte — documentation index

Operator UI for work-item **identity CRUD + matching + name search +
merge + audit timeline** across **four matchable collections**
(portfolios, projects, products, programs), plus the
**project-management workspace** (Kanban board, issues, timeline / Gantt,
burndown, goals) and cookie-session / SSO auth (BFF; the browser holds no
token), consuming the [Portfolio Service](../project-portfolio-management-service-with-loco).

> **Status: spec-only (v0.1.0).** No `src/` yet — the build queue is
> [spec/index.md](./spec/index.md) §13.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, planned `src/` tree, API consumption map, layout shell. |
| [README.md](./README.md) | Collections, routes, layout / chrome, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |
| [../spec/index.md](../spec/index.md) | Entity umbrella spec (cross-subproject contract). |

## Collections

Four matchable work-item kinds, each its own collection / list / CRUD;
matching is within a collection only. A **Portfolio** is the umbrella;
**Project** / **Product** / **Program** carry a `portfolio_ref` to their
parent and roll up under it.

| Collection | Kind | Parent |
|---|---|---|
| `/portfolios` | `Portfolio` | — (umbrella) |
| `/projects` | `Project` | `portfolio_ref` |
| `/products` | `Product` | `portfolio_ref` |
| `/programs` | `Program` | `portfolio_ref` |

## Flow

`{collection} ∈ { portfolios, projects, products, programs }`.

```text
/{collection}         ──>  GET  /api/{collection}                   list (SVAR DataGrid)
                          GET  /api/{collection}/search?q=migration name search
                          GET  /api/{collection}/events/recent      recent activity -> WorkItemEvent[]
/{collection}/new     ──>  POST /api/{collection}  {WorkItem}       create -> /{collection}/[pid]
/{collection}/[pid]   ──>  GET  /api/{collection}/{pid}             detail
                          POST /api/{collection}/check-duplicates    -> ScoredRef[] w/ MatchBreakdown
                          POST /api/{collection}/merge  {main_pid, duplicate_pid, reason?}  merge -> MergeResult
                          GET  /api/{collection}/{pid}/audit         audit timeline -> AuditEntry[]
                          DELETE /api/{collection}/{pid}            soft-delete
                          (portfolio) GET /api/{projects,products,programs}?portfolio_ref={pid}  roll-up
/{collection}/[pid]/edit ─> PUT /api/{collection}/{pid}             edit

project-management workspace (sub-resources under /api/{collection}/{pid}/…):
…/board    ──>  GET/POST …/tasks · PATCH …/tasks/{tid}    Kanban; drag = status change
…/issues   ──>  GET/POST …/issues · PUT …/issues/{iid}    issues (kind/severity/status)
…/timeline ──>  GET …/timeline    -> TimelineRow[]        Gantt (milestones + task ranges)
…/burndown ──>  GET …/burndown    -> BurndownPoint[]      remaining estimate over time
…/goals    ──>  GET/POST/PUT/DELETE …/goals              goals panel
```

## Layout & chrome

Global navigation is a full-width **top bar** with a **leftmost
hamburger** toggle (no left sidebar; content is full-width). The chrome
area carries a **collection switcher** (portfolios / projects / products /
programs), a **theme selector** (full shared catalogue — selecting a
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
