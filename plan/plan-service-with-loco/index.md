# Plan Service — documentation index

A loco.rs registry of plan records — and a project-management tool:
CRUD + matching (embedding the canonical plan-matcher), plus operational
sub-resources (goals, tasks, issues, posts, comments, members) and derived
timeline / burndown views.

> **Spec-only today.** No Rust / Cargo crate exists yet; this is the
> inaugural doc-set. The build queue is [spec §13](./spec/index.md).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** for this crate (§1–§18). |
| [../spec/index.md](../spec/index.md) | Entity-wide contract + canonical `Plan` model (§5). |
| [AGENTS.md](./AGENTS.md) | How to work here; API surface; MVP scope. |
| [README.md](./README.md) | User-facing intro + quick start. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

```text
create   ──>  POST   /api/v1/plans                {Plan}              -> {pid, name}  (409 on duplicate)
read     ──>  GET    /api/v1/plans/{pid}                              -> Plan
update   ──>  PUT    /api/v1/plans/{pid}          {Plan}              -> {pid, name}
delete   ──>  DELETE /api/v1/plans/{pid}                              -> 204
list     ──>  GET    /api/v1/plans                                    -> [{pid, name}]  (cap 100)
search   ──>  GET    /api/v1/plans/search?q=migration                -> [{pid, name}]  (ILIKE, cap 50)
dedupe   ──>  POST   /api/v1/plans/check-duplicates  {query}          -> [{pid, score, ...}]
match    ──>  POST   /api/v1/plans/match   {query, candidates}        -> ranked results
batch    ──>  POST   /api/v1/plans/deduplicate                        -> review-queue items
merge    ──>  POST   /api/v1/plans/merge   {main_pid, duplicate_pid}  -> merge record
merges   ──>  GET    /api/v1/plans/merges/recent                      -> [merge record]

tasks    ──>  POST   /api/v1/plans/{pid}/tasks    {Task}              -> {pid, ...}
issues   ──>  POST   /api/v1/plans/{pid}/issues   {Issue}             -> {pid, ...}
posts    ──>  POST   /api/v1/plans/{pid}/posts    {Post}              -> {pid, ...}
comments ──>  POST   /api/v1/plans/{pid}/comments {Comment}           -> {pid, ...}
members  ──>  POST   /api/v1/plans/{pid}/members  {Member}            -> {pid, ...}
goals    ──>  POST   /api/v1/plans/{pid}/goals    {Goal}              -> {pid, ...}
timeline ──>  GET    /api/v1/plans/{pid}/timeline                     -> Gantt projection
burndown ──>  GET    /api/v1/plans/{pid}/burndown                     -> remaining-vs-estimate series

links    ──>  POST·GET·DELETE /api/v1/plans/{pid}/links               -> cross-service edges
audit    ──>  GET    /api/v1/plans/audit/recent  ·  /{pid}/audit      -> [audit row]
events   ──>  GET    /api/v1/plans/events/recent                      -> [{kind, pid, name, seq}]
whoami   ──>  GET    /api/v1/plans/whoami          (Bearer PASETO)    -> verified claims (401 without)
docs     ──>  GET    /api-docs/openapi.json  ·  /swagger-ui           -> OpenAPI 3 + Swagger UI
metrics  ──>  GET    /metrics.prom                                    -> Prometheus text (public)
```

The `Plan` body shape is the `plan-matcher` type (name, plan code, owner
org, plan type, goals, timeframe, keywords, relationships, identifiers,
sameAs). The high-volume operational data — tasks, issues, posts,
comments, members — lives in **separate tables** keyed by the plan `pid`
and is **never** fed to the matcher (the partition rule); only the thin
identity payload is matched.

A create/update payload is validated (blank `name`, UUID / PM-tool-id /
URI identifier shapes, blank goal titles, BCP-47 `in_language`) and
returns `422` with every problem in one body. A real-time create
duplicate returns `409 Conflict` with the candidate matches. Each
create/update/delete/merge (plan and sub-resource) writes an `audit_logs`
row, publishes an event, and bumps the matching Prometheus counter.

Auth is optional per route by default: send `Authorization: Bearer
<paseto>` — a short-lived PASETO v4.public token minted by the
[authentication-service](../../authentication/authentication-service-with-loco)
and verified offline against its published Ed25519 key (front-ends use a
BFF + cookie session, so the browser holds no token). `whoami` echoes the
verified claims (`401` without one), while every other handler stamps the
token `sub` as the audit `actor`. Members, assignees, and authors are
user identities. Flip `PLAN_REQUIRE_AUTH=1` to require that token on every
`/api/*` route (`/metrics.prom`, `/api-docs/openapi.json`, `/swagger-ui`,
`/_health`, `/_ping` stay public). Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256/JWKS not used).
