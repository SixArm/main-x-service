# Portfolio Service

A registry of **work-item** records — and a project-management tool —
built on **loco.rs** and embedding the canonical
[project-portfolio-management-matcher](../project-portfolio-management-matcher-rust-crate).

A *work item* is a matchable identity for one of **four distinct kinds**:
a **Portfolio** (the umbrella container), a **Project**, a **Product**, or
a **Program**. Each kind is its own REST collection and table. The
service has two faces that share one record: a deduplicated, matchable
identity registry (the thin `WorkItem` payload) and a project workspace —
each work item *owns* operational sub-resources (goals, tasks, issues)
plus derived timeline / burndown views. Projects / Products / Programs sit
**under** a portfolio (they carry a `portfolio_ref` to their parent).

- Spec: [spec/index.md](./spec/index.md)
- Entity-wide contract: [portfolio entity spec](../spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Sibling UI: [project-portfolio-management-front-end-with-svelte](../project-portfolio-management-front-end-with-svelte)

> **Status: implemented (MVP, v0.1.0).** The crate builds and tests
> green; the remaining deferrals live in the work queue at
> [spec §13](./spec/index.md).

## API

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

Routes are under `/api/`. `{collection}` is one of `portfolios`,
`projects`, `products`, `programs` — each with the **identical** shape
below.

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/{collection}` | Create (`409` on real-time duplicate) |
| GET | `/api/{collection}` | List |
| GET | `/api/{collection}/{pid}` | Fetch |
| PUT | `/api/{collection}/{pid}` | Update |
| DELETE | `/api/{collection}/{pid}` | Soft-delete |
| GET | `/api/{collection}/search?q=` | Case-insensitive name search (`ILIKE`, cap 50) |
| POST | `/api/{collection}/match` | Rank `{query, candidates}` (cross-kind → `0.0`) |
| POST | `/api/{collection}/check-duplicates` | Match a query vs stored records in this collection |
| POST | `/api/{collection}/deduplicate` | Batch scan → review queue |
| POST | `/api/{collection}/merge` | Merge a duplicate into a same-kind survivor |
| GET | `/api/{collection}/merges/recent` | Merge-history records |
| * | `/api/{collection}/{pid}/goals` · `/tasks` · `/issues` | Operational sub-resource CRUD |
| GET | `/api/{collection}/{pid}/timeline` · `/burndown` | Derived Gantt / burndown views |
| POST·GET·DELETE | `/api/{collection}/{pid}/links` | Cross-service entity links |
| GET | `/api/{collection}/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/{collection}/events/recent` | In-memory event stream |
| GET | `/api/{collection}/whoami` | Verified PASETO-token claims (`401` without one) |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (root path, public under auth enforcement) |

See [AGENTS.md](./AGENTS.md) and [spec §9](./spec/index.md) for the full
route contract.

The body for a work item **is** the `project_portfolio_management_matcher::WorkItem` shape
(kind, name, code + owner org, parent `portfolio_ref`, goals, dates,
keywords, tags, relationships, identifiers, sameAs). **Matching is within
a collection only** — the matcher's R-GATE makes a project never match a
product. The high-volume operational data (tasks, issues) lives in
separate tables and is **never** fed to the matcher (goal **titles**
bridge in via `data.goals[]`).

## Quick start

> The goals / tasks / issues sub-resource routes in the route contract
> are deferred (spec §13); the commands below run against the shipped
> crate.

Requires PostgreSQL.

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/project_portfolio_management_service_development
cargo loco start        # migrations auto-run in development

# Create a portfolio (the umbrella kind)
curl -s localhost:5150/api/portfolios -H 'content-type: application/json' \
  -d '{"name":"Digital Transformation Portfolio",
       "goals":[{"title":"Modernise core systems","target_date":"2026-12-01"}]}'

# Create a project under that portfolio (carries portfolio_ref)
curl -s localhost:5150/api/projects -H 'content-type: application/json' \
  -d '{"name":"EHR Migration","portfolio_ref":"<portfolio-pid>",
       "code":"EHR-2026"}'

# Name search within a collection
curl -s 'localhost:5150/api/projects/search?q=migration'

# Match an explicit query against candidates (no persistence; same kind)
curl -s localhost:5150/api/projects/match -H 'content-type: application/json' \
  -d '{"query":{"name":"EHR Migration"},"candidates":[{"name":"EHR Migration Project"}]}'

# Merge a duplicate into a survivor (the survivor is `main_pid`; same kind)
curl -s localhost:5150/api/projects/merge -H 'content-type: application/json' \
  -d '{"main_pid":"<survivor-uuid>","duplicate_pid":"<duplicate-uuid>"}'

# Authenticated request: present a short-lived PASETO v4.public token
# minted by the auth-service (front-ends use a BFF + cookie session; the
# BFF holds the session and supplies this bearer server-side).
curl -s localhost:5150/api/projects/whoami \
  -H 'authorization: Bearer <paseto-v4.public-from-authentication-service>'
```

A validation failure returns `422` with **every** problem in one body
(blank `name` plus a malformed identifier plus a bad goal title come back
together). A real-time create duplicate returns `409 Conflict` with the
candidate matches (within that collection).

## Testing

```bash
cargo test                   # DB-free: matcher embedding (+ R-GATE), JSON
                             # round-trip, validation pins, merge + derived-view logic
cargo test -- --ignored      # request-level tests (need Postgres DATABASE_URL)
cargo clippy --all-targets
```

## Status

Implemented (MVP, v0.1.0): CRUD + `ILIKE` name search + within-kind
matching + record merge + payload validation across the four collections
(Portfolio / Project / Product / Program — one `work_items` table keyed
by `kind`), plus the governance / visibility / strategy phases
(proposals, gate reviews, risks, budget lines; dependencies, milestones,
allocations, capacity, reports, at-a-glance; ideas, scenarios,
objectives, benefits), audit log + event streaming (durable-bus Phase 2
outbox + Phase 3 relay/retention have landed; `src/relay.rs`, gated by
`PROJECT_PORTFOLIO_MANAGEMENT_EVENT_TRANSPORT=outbox` +
`PROJECT_PORTFOLIO_MANAGEMENT_EVENT_RELAY`) + OpenAPI/Swagger +
Prometheus metrics + offline PASETO v4.public verification (published
Ed25519 key) + blanket `/api/*` auth enforcement (off by default, gated
by `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`). Deferred (see
[spec §13](./spec/index.md)): the operational sub-resources (goals /
tasks / issues) + derived timeline / burndown views, `deduplicate` + the
review queue, cross-service entity links, Tantivy full-text/fuzzy
search, the durable bus's Fluvio broker sink, privacy, front-end merge
action, bulk import/export, the `posts` / `comments` / `members`
collaboration sub-resources, gRPC. Auth credentials are issued by the central
[authentication-service](../../authentication/authentication-service-with-loco):
the human session is a server-side cookie session, and peers verify a
short-lived PASETO v4.public token offline. See
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(source of truth; RS256/JWKS not used).

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
</content>
