# care-pathway-front-end-with-svelte — documentation index

Operator UI for care-pathway CRUD + matching + name search + merge +
audit trail + recent activity + cookie-session / BFF auth (PASETO; pivot
in progress), consuming the
[Care Pathway Service](../care-pathway-service-with-loco).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, `src/` tree, API consumption map. |
| [README.md](./README.md) | Routes, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Flow

```text
/         ──>  GET  /api/care-pathways                  list
              GET  /api/care-pathways/search?q=stroke   name search
              GET  /api/care-pathways/events/recent     recent activity -> PathwayEvent[]
/new      ──>  POST /api/care-pathways  {CarePathway}   create -> /[pid]
/[pid]    ──>  GET  /api/care-pathways/{pid}            detail
              POST /api/care-pathways/check-duplicates   -> scored matches
              POST /api/care-pathways/merge  {main_pid, duplicate_pid, reason?}  merge -> MergeResult
              GET  /api/care-pathways/{pid}/audit        audit trail -> AuditEntry[]
              DELETE /api/care-pathways/{pid}             soft-delete
/[pid]/edit ─> PUT  /api/care-pathways/{pid}             edit
```

Auth (BFF): sign-in establishes a server-side **cookie session**
(`__Host-mxi_session`, httpOnly); the browser holds no token and talks
only to this front-end's own SvelteKit server (BFF), which exchanges the
session for a short-lived **PASETO v4.public** token and calls the
service server-side (mutations CSRF-protected; no `localStorage`, no
`mxi_access_token`). Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256 JWT + JWKS and the `#access_token` fragment handoff
decommissioned). The runtime is the BFF: `src/lib/server/` + the
`/api/proxy` server route hold the session and inject the PASETO
server-side.
