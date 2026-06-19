# Portfolio Service — documentation index

A loco.rs registry of work-item records — and a project-management tool:
CRUD + matching (embedding the canonical portfolio-matcher) across the
**four matchable kinds** (Portfolio / Project / Product / Program), plus
operational sub-resources (goals, tasks, issues) and derived timeline /
burndown views.

> **Spec-only today.** No Rust / Cargo crate exists yet; this is the
> inaugural doc-set. The build queue is [spec §13](./spec/index.md).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** for this crate (§1–§18). |
| [../spec/index.md](../spec/index.md) | Entity-wide contract + canonical `WorkItem` model (§5). |
| [AGENTS.md](./AGENTS.md) | How to work here; API surface; MVP scope. |
| [README.md](./README.md) | User-facing intro + quick start. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

`{collection}` is one of `portfolios` / `projects` / `products` /
`programs` — each with the identical shape below.

```text
create   ──>  POST   /api/v1/{collection}                {WorkItem}          -> {pid, name}  (409 on duplicate)
read     ──>  GET    /api/v1/{collection}/{pid}                              -> WorkItem
update   ──>  PUT    /api/v1/{collection}/{pid}          {WorkItem}          -> {pid, name}
delete   ──>  DELETE /api/v1/{collection}/{pid}                              -> 204
list     ──>  GET    /api/v1/{collection}                                    -> [{pid, name}]  (cap 100)
search   ──>  GET    /api/v1/{collection}/search?q=migration                -> [{pid, name}]  (ILIKE, cap 50)
dedupe   ──>  POST   /api/v1/{collection}/check-duplicates  {query}          -> [{pid, score, ...}]
match    ──>  POST   /api/v1/{collection}/match   {query, candidates}        -> ranked results (cross-kind → 0.0)
batch    ──>  POST   /api/v1/{collection}/deduplicate                        -> review-queue items
merge    ──>  POST   /api/v1/{collection}/merge   {main_pid, duplicate_pid}  -> merge record (same kind only)
merges   ──>  GET    /api/v1/{collection}/merges/recent                      -> [merge record]

goals    ──>  POST   /api/v1/{collection}/{pid}/goals    {Goal}              -> {pid, ...}
tasks    ──>  POST   /api/v1/{collection}/{pid}/tasks    {Task}              -> {pid, ...}
issues   ──>  POST   /api/v1/{collection}/{pid}/issues   {Issue}             -> {pid, ...}
timeline ──>  GET    /api/v1/{collection}/{pid}/timeline                     -> Gantt projection
burndown ──>  GET    /api/v1/{collection}/{pid}/burndown                     -> remaining-vs-estimate series

links    ──>  POST·GET·DELETE /api/v1/{collection}/{pid}/links               -> cross-service edges
audit    ──>  GET    /api/v1/{collection}/audit/recent  ·  /{pid}/audit      -> [audit row]
events   ──>  GET    /api/v1/{collection}/events/recent                      -> [{kind, pid, name, seq}]
whoami   ──>  GET    /api/v1/{collection}/whoami          (Bearer PASETO)    -> verified claims (401 without)
docs     ──>  GET    /api-docs/openapi.json  ·  /swagger-ui                  -> OpenAPI 3 + Swagger UI
metrics  ──>  GET    /metrics.prom                                           -> Prometheus text (public)
```

The `WorkItem` body shape is the `portfolio-matcher` type (kind, name,
code, owner org, parent `portfolio_ref`, goals, dates, keywords, tags,
relationships, identifiers, sameAs). The four kinds — Portfolio / Project
/ Product / Program — are **distinct collections and tables**, and
**matching is within a collection only** (the matcher's R-GATE makes a
project never match a product). Projects / Products / Programs carry a
`portfolio_ref` to their parent portfolio. The high-volume operational
data — tasks, issues — lives in **separate tables** keyed by the parent
`(kind, pid)` and is **never** fed to the matcher (the partition rule);
only the thin identity payload is matched, with goal **titles** bridging
in via `data.goals[]`.

A create/update payload is validated (blank `name`, UUID / PM-tool-id /
URI identifier shapes, blank goal titles, BCP-47 `in_language`, child-kind
`portfolio_ref`) and returns `422` with every problem in one body. A
real-time create duplicate returns `409 Conflict` with the candidate
matches. Each create/update/delete/merge (work item and sub-resource)
writes an `audit_logs` row, publishes an event, and bumps the matching
per-collection Prometheus counter.

Auth is optional per route by default: send `Authorization: Bearer
<paseto>` — a short-lived PASETO v4.public token minted by the
[authentication-service](../../authentication/authentication-service-with-loco)
and verified offline against its published Ed25519 key (front-ends use a
BFF + cookie session, so the browser holds no token). `whoami` echoes the
verified claims (`401` without one), while every other handler stamps the
token `sub` as the audit `actor`. Leads, assignees, and owners are user /
org identities. Flip `PORTFOLIO_REQUIRE_AUTH=1` to require that token on
every `/api/*` route (`/metrics.prom`, `/api-docs/openapi.json`,
`/swagger-ui`, `/_health`, `/_ping` stay public). Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256/JWKS not used).
</content>
