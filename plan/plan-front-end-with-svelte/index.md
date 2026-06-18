# plan-front-end-with-svelte — documentation index

Operator UI for plan **identity CRUD + matching + name search + merge +
audit timeline** plus the **project-management workspace** (Kanban
board, issues, timeline / Gantt, burndown, goals, posts + comments,
members) and cookie-session / SSO auth (BFF; the browser holds no
token), consuming the [Plan Service](../plan-service-with-loco).

> **Status: spec-only (v0.1.0).** No `src/` yet — the build queue is
> [spec/index.md](./spec/index.md) §13.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, planned `src/` tree, API consumption map, layout shell. |
| [README.md](./README.md) | Routes, layout / chrome, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |
| [../spec/index.md](../spec/index.md) | Entity umbrella spec (cross-subproject contract). |

## Flow

```text
/         ──>  GET  /api/v1/plans                      list (SVAR DataGrid)
              GET  /api/v1/plans/search?q=migration    name search
              GET  /api/v1/plans/events/recent         recent activity -> PlanEvent[]
/new      ──>  POST /api/v1/plans  {Plan}              create -> /[pid]
/[pid]    ──>  GET  /api/v1/plans/{pid}                detail
              POST /api/v1/plans/check-duplicates       -> ScoredRef[] w/ MatchBreakdown
              POST /api/v1/plans/merge  {main_pid, duplicate_pid, reason?}  merge -> MergeResult
              GET  /api/v1/plans/{pid}/audit            audit timeline -> AuditEntry[]
              DELETE /api/v1/plans/{pid}                soft-delete
/[pid]/edit ─> PUT  /api/v1/plans/{pid}                edit

project-management workspace (sub-resources under /api/v1/plans/{pid}/…):
/[pid]/board    ──>  GET/POST …/tasks · PATCH …/tasks/{tid}   Kanban; drag = status change
/[pid]/issues   ──>  GET/POST …/issues · PUT …/issues/{iid}   issues (kind/severity/status)
/[pid]/timeline ──>  GET …/timeline    -> TimelineRow[]       Gantt (milestones + task ranges)
/[pid]/burndown ──>  GET …/burndown    -> BurndownPoint[]     remaining estimate over time
/[pid]/goals    ──>  GET/POST/PUT/DELETE …/goals             goals panel
/[pid]/posts    ──>  GET/POST …/posts · POST …/posts/{id}/comments   feed + threaded comments
/[pid]/members  ──>  GET/POST …/members · PATCH/DELETE …/members/{id}  role management
```

## Layout & chrome

Global navigation is a full-width **top bar** with a **leftmost
hamburger** toggle (no left sidebar; content is full-width). The chrome
area carries a **theme selector** (full shared catalogue — selecting a
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
short-lived **PASETO v4.public** token, and calls the plan service
server-side; mutating browser→BFF calls carry a CSRF token. Source of
truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256/JWKS not used).
