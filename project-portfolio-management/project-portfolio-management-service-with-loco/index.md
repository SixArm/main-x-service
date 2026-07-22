# Portfolio Service — documentation index

A loco.rs registry of plan records — and a project-management tool:
CRUD + matching (embedding the canonical project-portfolio-management-matcher) over one
recursive `plans` collection with an **optional** Portfolio / Project /
Product / Program / Practice / Process / Purpose / Pathway / Proposal
kind label, plus operational sub-resources (goals,
tasks, issues) and derived timeline / burndown views.

> **Implemented (MVP, v0.1.0).** The crate exists and builds (`src/`,
> seven migrations); the remaining deferrals live in the work queue at
> [spec §13](./spec/index.md).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** for this crate (§1–§18). |
| [../spec/index.md](../spec/index.md) | Entity-wide contract + canonical `Plan` model (§5). |
| [AGENTS.md](./AGENTS.md) | How to work here; API surface; MVP scope. |
| [README.md](./README.md) | User-facing intro + quick start. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

Plans live in one collection at `/api/plans`; plan-scoped sub-resources
hang off `/api/plans/{pid}/...`.

```text
create   ──>  POST   /api/plans                {Plan}                     -> {pid, name}  (409 on duplicate)
read     ──>  GET    /api/plans/{pid}                                     -> Plan
update   ──>  PUT    /api/plans/{pid}          {Plan}                     -> {pid, name}
delete   ──>  DELETE /api/plans/{pid}                                     -> 204
list     ──>  GET    /api/plans                                           -> [{pid, name}]  (cap 100)
search   ──>  GET    /api/plans/search?q=migration                       -> [{pid, name}]  (ILIKE, cap 50)
dedupe   ──>  POST   /api/plans/check-duplicates  {query}                 -> [{pid, score, ...}]
match    ──>  POST   /api/plans/match   {query, candidates}               -> ranked results (kind-agnostic)
batch    ──>  POST   /api/plans/deduplicate                              -> review-queue items
merge    ──>  POST   /api/plans/merge   {main_pid, duplicate_pid}         -> merge record (any two plans)
merges   ──>  GET    /api/plans/merges/recent                            -> [merge record]

goals    ──>  POST   /api/plans/{pid}/goals    {Goal}                     -> {pid, ...}
tasks    ──>  POST   /api/plans/{pid}/tasks    {Task}                     -> {pid, ...}
issues   ──>  POST   /api/plans/{pid}/issues   {Issue}                    -> {pid, ...}
timeline ──>  GET    /api/plans/{pid}/timeline                            -> Gantt projection
burndown ──>  GET    /api/plans/{pid}/burndown                            -> remaining-vs-estimate series

links    ──>  POST·GET·DELETE /api/plans/{pid}/links                      -> cross-service edges
audit    ──>  GET    /api/plans/audit/recent  ·  /{pid}/audit             -> [audit row]
events   ──>  GET    /api/plans/events/recent                            -> [{kind, pid, name, seq}]
whoami   ──>  GET    /api/plans/whoami          (Bearer PASETO)           -> verified claims (401 without)
docs     ──>  GET    /api-docs/openapi.json  ·  /swagger-ui                  -> OpenAPI 3 + Swagger UI
metrics  ──>  GET    /metrics.prom                                           -> Prometheus text (public)
```

The `Plan` body shape is the `project-portfolio-management-matcher` type (optional `kind`
label, name, code, owner org, `parent_ref`, goals, dates, keywords, tags,
relationships, identifiers, sameAs). All plans live in **one collection
and table**, and **matching is kind-agnostic** (there is no kind gate, so
any two plans may match). Any plan may contain any other plan via a
`parent_ref` to its parent (a recursive tree; a self- or descendant-cycle
is rejected `422`). The high-volume operational data — tasks, issues —
lives in **separate tables** keyed by the parent plan `pid` and is
**never** fed to the matcher (the partition rule); only the thin identity
payload is matched, with goal **titles** bridging in via `data.goals[]`.

A create/update payload is validated (blank `name`, UUID / PM-tool-id /
URI identifier shapes, blank goal titles, BCP-47 `in_language`,
`parent_ref` UUID + containment-cycle check) and returns `422` with every
problem in one body. A real-time create duplicate returns `409 Conflict`
with the candidate matches. Each create/update/delete/merge (plan and
sub-resource) writes an `audit_logs` row, publishes an event, and bumps
the matching Prometheus counter.

Auth is optional per route by default: send `Authorization: Bearer
<paseto>` — a short-lived PASETO v4.public token minted by the
[authentication-service](../../authentication/authentication-service-with-loco)
and verified offline against its published Ed25519 key (front-ends use a
BFF + cookie session, so the browser holds no token). `whoami` echoes the
verified claims (`401` without one), while every other handler stamps the
token `sub` as the audit `actor`. Leads, assignees, and owners are user /
org identities. Flip `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH=1` to require that token on
every `/api/*` route (`/metrics.prom`, `/api-docs/openapi.json`,
`/swagger-ui`, `/_health`, `/_ping` stay public). Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256/JWKS not used).
</content>
