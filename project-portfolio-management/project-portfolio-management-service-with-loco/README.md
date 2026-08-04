# Project Portfolio Management Service

A registry of **plan** records — and a project-management tool —
built on **loco.rs** and embedding the canonical
[project-portfolio-management-matcher](../project-portfolio-management-matcher-rust-crate).

A *plan* is a matchable identity in one recursive collection. Its `kind`
— **Portfolio**, **Project**, **Product**, **Program**, **Practice**,
**Process**, **Purpose**, **Pathway**, or **Proposal** — is an
**optional descriptive label** (a display / grouping hint), not a
required discriminator and not a separate collection. The service has
two faces that share one record: a deduplicated, matchable identity
registry (the thin `Plan` payload) and a project workspace — each plan
*owns* operational sub-resources (goals, tasks, issues) plus derived
timeline / burndown views. Any plan may contain any other plan via a
`parent_ref` to its parent (a recursive tree); a `parent_ref` that
points a plan at itself or at one of its descendants is rejected (`422`,
containment-cycle check).

- Spec: [spec/index.md](./spec/index.md)
- Entity-wide contract: [portfolio entity spec](../spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Sibling UI: [project-portfolio-management-front-end-with-svelte](../project-portfolio-management-front-end-with-svelte)

> **Status: implemented (MVP, v0.1.0).** The crate builds and tests
> green; the remaining deferrals live in the work queue at
> [spec §13](./spec/index.md).

## API

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

Routes are under `/api/`. Plans live in **one** collection, `/api/plans`;
sub-resources hang off `/api/plans/{pid}/...`.

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/plans` | Create (`409` on real-time duplicate) |
| GET | `/api/plans` | List |
| GET | `/api/plans/{pid}` | Fetch |
| PUT | `/api/plans/{pid}` | Update |
| DELETE | `/api/plans/{pid}` | Soft-delete |
| GET | `/api/plans/search?q=` | Tantivy full-text/fuzzy/phonetic search (`?fuzzy=`, `?phonetic=`, `?kind=`) |
| POST | `/api/plans/match` | Rank `{query, candidates}` (kind-agnostic) |
| POST | `/api/plans/check-duplicates` | Match a query vs stored plans (blocked on the Tantivy index) |
| POST | `/api/plans/deduplicate` | Batch scan → review queue — **deferred, spec §13** |
| POST | `/api/plans/merge` | Merge a duplicate into any survivor plan |
| GET | `/api/plans/merges/recent` | Merge-history records |
| GET | `/api/plans/{pid}/masked` · `/export` | Masked view (always redacted) · audited GDPR export |
| * | `/api/plans/{pid}/tasks` | Task-board CRUD + Kanban `PATCH` move (story points, WIP limits) — `/goals` · `/issues` **deferred, spec §13** |
| GET | `/api/plans/{pid}/sprints` · `/burndown` · `/velocity` · `/standup` | Sprint tooling — `/timeline` (Gantt) **deferred, spec §13** |
| POST·GET·DELETE | `/api/plans/{pid}/links` | Cross-service entity links — **deferred, spec §13** |
| POST·GET | `/api/reviews` (+ `/consensus`, `/{pid}/respond`, `/{pid}/submit`) | Collaborative review: delegate to internal or external experts |
| POST | `/api/plans/{pid}/tasks/{t_pid}/assign` | Assign / unassign a task (`null` unassigns) |
| GET | `/api/assignees/workload` | Open work per assignee, incl. the unassigned pile |
| GET | `/api/notifications` (+ `/{pid}/read`) | In-app inbox (no email / push transport) |
| POST·GET | `/api/automations` (+ `/{pid}/enable`·`/disable`, `/runs`) | Workflow automation: rules fired by board moves |
| POST·GET | `/api/scheduled-actions` (+ `/sweep`) | Set and forget: the deadline queue |
| GET | `/api/plans/{pid}/smart-score` · `/api/prioritisation` | Smart Score + the ranked queue (with the full breakdown) |
| GET | `/api/lifecycle` · `/api/plans/{pid}/lifecycle` | Bird's-eye funnel + next-phase readiness checklist |
| GET | `/api/plans/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/plans/events/recent` | In-memory event stream |
| GET | `/api/plans/whoami` | Verified PASETO-token claims (`401` without one) |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (root path, public under auth enforcement) |

The table above is the identity-registry core. This service is also a
full **project-management tool**: PPM governance (`/api/proposals`,
`/api/plans/{pid}/gate-reviews`, `/risks`, `/budget-lines`), visibility
(`/api/dependencies`, `/plans/{pid}/schedule`, `/milestones`,
`/allocations`, `/capacity`, `/at-a-glance`), strategy (`/api/ideas`,
`/api/scenarios`, `/api/objectives`, `/api/plans/{pid}/benefits`),
executive insights (`/api/executive/*`, `/financials/*`,
`/technology/*`), oversight (`/api/board/*`, `/auditor/*`,
`/compliance/*`, `/risk/heatmap`, `/security/register`,
`/regulator/extract`), and row-level integrity verification
(`/api/compliance/records/verify`, `/audit/verify`) — see
[AGENTS.md](./AGENTS.md) for the full table and
[spec §9.9–§9.15](./spec/index.md) for each area's contract.

See [AGENTS.md](./AGENTS.md) and [spec §9](./spec/index.md) for the full
route contract.

The body for a plan **is** the `project_portfolio_management_matcher::Plan`
shape (optional `kind` label, name, code + owner org, `parent_ref`, goals,
dates, keywords, tags, relationships, identifiers, sameAs). **Matching is
kind-agnostic** — there is no kind gate, so any two plans may match
regardless of their (optional) labels. The high-volume operational data
(tasks, issues) lives in separate tables and is **never** fed to the
matcher (goal **titles** bridge in via `data.goals[]`).

## Quick start

> The goals / tasks / issues sub-resource routes in the route contract
> are deferred (spec §13); the commands below run against the shipped
> crate.

Requires PostgreSQL.

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/project_portfolio_management_service_development
cargo loco start        # migrations auto-run in development

# Create a plan labelled as a portfolio (the umbrella kind)
curl -s localhost:5150/api/plans -H 'content-type: application/json' \
  -d '{"name":"Digital Transformation Portfolio","kind":"portfolio",
       "goals":[{"title":"Modernise core systems","target_date":"2026-12-01"}]}'

# Create a plan under that one (carries parent_ref; kind label optional)
curl -s localhost:5150/api/plans -H 'content-type: application/json' \
  -d '{"name":"EHR Migration","kind":"project","parent_ref":"<parent-pid>",
       "code":"EHR-2026"}'

# Name search across all plans
curl -s 'localhost:5150/api/plans/search?q=migration'

# Match an explicit query against candidates (no persistence; kind-agnostic)
curl -s localhost:5150/api/plans/match -H 'content-type: application/json' \
  -d '{"query":{"name":"EHR Migration"},"candidates":[{"name":"EHR Migration Project"}]}'

# Merge a duplicate into any survivor plan (the survivor is `main_pid`)
curl -s localhost:5150/api/plans/merge -H 'content-type: application/json' \
  -d '{"main_pid":"<survivor-uuid>","duplicate_pid":"<duplicate-uuid>"}'

# Authenticated request: present a short-lived PASETO v4.public token
# minted by the auth-service (front-ends use a BFF + cookie session; the
# BFF holds the session and supplies this bearer server-side).
curl -s localhost:5150/api/plans/whoami \
  -H 'authorization: Bearer <paseto-v4.public-from-authentication-service>'
```

A validation failure returns `422` with **every** problem in one body
(blank `name` plus a malformed identifier plus a bad goal title come back
together). A real-time create duplicate returns `409 Conflict` with the
candidate matches.

## Testing

```bash
cargo test                   # DB-free: matcher embedding, JSON round-trip,
                             # validation pins, merge + derived-view logic
cargo test -- --ignored      # request-level tests (need Postgres DATABASE_URL)
cargo clippy --all-targets
```

## Status

Implemented (MVP, v0.1.0 + PPM Phases A/B/C): CRUD + Tantivy
full-text/fuzzy/phonetic name search (`?fuzzy=`, `?phonetic=`, `?kind=`;
replaced the earlier `ILIKE` search 2026-08-02) + kind-agnostic matching
+ record merge + payload validation over one recursive `plans`
collection (`kind` is an optional Portfolio / Project / Product /
Program / Practice / Process / Purpose / Pathway / Proposal label; one
`plans` table with a nullable `kind` and a `parent_pid`), plus the
governance / visibility / strategy phases (proposals, gate reviews,
risks, budget lines; dependencies, milestones, allocations, capacity,
reports, at-a-glance; ideas, scenarios, objectives, benefits), the
engineering-team core (Kanban task board with WIP limits + story
points, sprints, an honest `done_at`-only burndown, velocity, standup,
DevOps deploy/incident ingest — spec §9.14), the executive insight areas
(CEO/CFO/CTO views — spec §9.12) and oversight areas (board / auditor /
compliance-register / risk-heatmap / security / regulator — spec §9.13),
the collaboration / automation / prioritisation capabilities (collaborative
review with strict-majority consensus, assignee management, workflow
automation with a full run log, the claim-based set-and-forget
scheduler, the explainable Smart Score, and bird's-eye lifecycle
readiness — spec §9.4a), field masking + audited GDPR export wired to
the ABAC `mask` obligation (`lead_ref` dropped entirely, owner org
masked — spec §12/§13, 2026-08-02), row-level integrity verification
(SHA-256 + SHA3-256 digests + a keyed MAC over `plans` and `audit_logs`,
default off without a configured MAC key — spec §9.15), audit log
+ event streaming (durable-bus Phase 2 outbox + Phase 3 relay/retention
have landed; `src/relay.rs`, gated by
`PROJECT_PORTFOLIO_MANAGEMENT_EVENT_TRANSPORT=outbox` +
`PROJECT_PORTFOLIO_MANAGEMENT_EVENT_RELAY`; the `FluvioSink` real-broker
sink is feature-gated and off by default) + OpenAPI/Swagger +
Prometheus metrics + offline PASETO v4.public verification (published
Ed25519 key) + blanket `/api/*` auth enforcement (off by default, gated
by `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`). Deferred (see [spec
§13](./spec/index.md)): the `goals` / `issues` operational sub-resources
(only `tasks` is wired) + the derived `/timeline` view, `deduplicate` +
the review queue, cross-service entity links, front-end merge action,
bulk import/export, the `posts` / `comments` / `members` collaboration
sub-resources, gRPC. Auth credentials are issued by the central
[authentication-service](../../authentication/authentication-service-with-loco):
the human session is a server-side cookie session, and peers verify a
short-lived PASETO v4.public token offline. See
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(source of truth; RS256/JWKS not used).

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
</content>
