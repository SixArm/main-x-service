# project-portfolio-management-front-end-with-svelte — documentation index

Operator UI for **plan** identity CRUD + matching + merge over **one
recursive `/api/plans` collection**, plus the **project-management
workspace** (Kanban board, governance, schedule), a wide set of
oversight / executive dashboard views, and cookie-session / SSO auth
(BFF; the browser holds no token), consuming the
[Portfolio Service](../project-portfolio-management-service-with-loco).

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
/plans          ──>  GET  /api/plans                       list (SVAR DataGrid + client-side filter)
/plans/new      ──>  POST /api/plans  {Plan}                create -> /plans/[pid]
/plans/[pid]    ──>  GET  /api/plans/{pid}                  detail
                     POST /api/plans/check-duplicates       -> ScoredRef[] (score + confidence)
                     DELETE /api/plans/{pid}               soft-delete
/plans/[pid]/edit ─> PUT /api/plans/{pid}                  edit
/plans/merge    ──>  POST /api/plans/merge  {main_pid, duplicate_pid, reason?}  merge -> MergeResponse
                     GET  /api/plans/merges/recent          merge history

project-management workspace (sub-resources under /api/plans/{pid}/…):
…/board    ──>  GET/POST …/tasks · PATCH …/tasks/{tid}    Kanban; drag = status change
…/board    ──>  GET/POST …/sprints · GET …/burndown?sprint=   sprint create/select + honest burndown

Not built (no repository method, type, or route): a name-search round
trip (GET /api/plans/search?q=), a recent-activity feed, a per-plan
audit timeline, a match-score breakdown visual, the detail page's
child-plan roll-up, and the issues / timeline / goals sub-resource
views. See spec/index.md §13.
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

Cookie session via a BFF: **Sign in** leads to this app's own `/signin` +
`/verify` pages (not a redirect to a central authentication front-end),
which run the magic-link and establish a server-side session, setting an
httpOnly `__Host-mxi_session` cookie. The browser holds **no token** (no
`localStorage`, no URL fragment). This app's SvelteKit server acts as a
**Backend-For-Frontend**: it holds the session, exchanges it for a
short-lived **PASETO v4.public** token, and calls the portfolio service
through the same-origin `/api/proxy` route; mutating browser→BFF calls
carry a CSRF token. Configure the BFF's upstream URLs with
`PROJECT_PORTFOLIO_MANAGEMENT_API_URL` and `AUTH_API_URL` (both default
to `http://localhost:5150`; see `.env.example`). Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256/JWKS not used).
