# case-front-end-with-svelte — documentation index

Operator UI for case CRUD + matching, consuming the
[Case Service](../case-service-with-loco).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, `src/` tree, API consumption map. |
| [README.md](./README.md) | Routes, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Flow

```text
/         ──>  GET  /api/cases                  list
/cases    ──>  GET  /api/cases                  SVAR DataGrid + FilterBar index
/board    ──>  GET  /api/cases (+ PUT per drag)  status Kanban
/new      ──>  POST /api/cases  {Case}          create -> /[pid]
/[pid]    ──>  GET  /api/cases/{pid}            detail
              POST /api/cases/check-duplicates   -> scored matches
              DELETE /api/cases/{pid}             soft-delete
              GET/POST/DELETE /api/cases/{pid}/links   "subject of this case" panel
/[pid]/edit ─> PUT  /api/cases/{pid}             edit
/merge    ──>  POST /api/cases/merge             merge a duplicate into a survivor
              GET  /api/cases/merges/recent       recent merge history
```

## Sign-in (SSO) — cookie session via BFF

```text
sidebar "Sign in"  ──>  ${AUTH_FRONTEND_URL}/signin?return_to=<this app>   signInUrl()
auth magic-link    ──>  auth service sets httpOnly cookie __Host-mxi_session
browser            ──>  holds the cookie only; no token, no localStorage, no fragment
this app's BFF     ──>  SvelteKit server exchanges the session for a short-lived
                        PASETO v4.public token and calls the case service server-side
                        (Authorization: Bearer <PASETO>); mutating calls carry a CSRF token
```

The browser never holds a credential; the SvelteKit server acts as a
Backend-For-Frontend. Once the case service activates `CASE_REQUIRE_AUTH`,
the BFF's PASETO bearer carries the operator through blanket auth
enforcement. See
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(source of truth; RS256/JWKS decommissioned).

> The BFF pivot has landed: `src/hooks.server.ts` + `src/lib/server/*`
> implement the session cookie → PASETO exchange → server-side proxy
> (`/api/proxy/[...path]`). There is no client-held bearer token and no
> URL-fragment handoff anywhere in this app's runtime.
