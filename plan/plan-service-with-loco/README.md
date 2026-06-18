# Plan Service

A registry of **plan** records — and a project-management tool — built
on **loco.rs** and embedding the canonical
[plan-matcher](../plan-matcher-rust-crate).

A *plan* is a matchable identity for a **project, product, programme,
initiative, portfolio, or epic**. The service has two faces that share
one record: a deduplicated, matchable identity registry (the thin `Plan`
payload) and a project workspace — each `Plan` *owns* operational
sub-resources (goals, tasks, issues, posts, comments, members) plus
derived timeline / burndown views.

- Spec: [spec/index.md](./spec/index.md)
- Entity-wide contract: [plan entity spec](../spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Sibling UI: [plan-front-end-with-svelte](../plan-front-end-with-svelte)

> **Status: spec-only.** No Rust / Cargo crate has been generated yet.
> This doc-set is the inaugural scaffold; the build queue is
> [spec §13](./spec/index.md).

## API

Routes are under `/api/v1/`.

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/v1/plans` | Create (`409` on real-time duplicate) |
| GET | `/api/v1/plans` | List |
| GET | `/api/v1/plans/{pid}` | Fetch |
| PUT | `/api/v1/plans/{pid}` | Update |
| DELETE | `/api/v1/plans/{pid}` | Soft-delete |
| GET | `/api/v1/plans/search?q=` | Case-insensitive name search (`ILIKE`, cap 50) |
| POST | `/api/v1/plans/match` | Rank `{query, candidates}` |
| POST | `/api/v1/plans/check-duplicates` | Match a query vs stored plans |
| POST | `/api/v1/plans/deduplicate` | Batch scan → review queue |
| POST | `/api/v1/plans/merge` | Merge a duplicate into a survivor |
| GET | `/api/v1/plans/merges/recent` | Merge-history records |
| * | `/api/v1/plans/{pid}/goals` · `/tasks` · `/issues` · `/posts` · `/comments` · `/members` | Operational sub-resource CRUD |
| GET | `/api/v1/plans/{pid}/timeline` · `/burndown` | Derived Gantt / burndown views |
| POST·GET·DELETE | `/api/v1/plans/{pid}/links` | Cross-service entity links |
| GET | `/api/v1/plans/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/v1/plans/events/recent` | In-memory event stream |
| GET | `/api/v1/plans/whoami` | Verified PASETO-token claims (`401` without one) |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (root path, public under auth enforcement) |

See [AGENTS.md](./AGENTS.md) and [spec §9](./spec/index.md) for the full
route contract.

The body for a plan **is** the `plan_matcher::Plan` shape (name, plan
code + owner org, plan type, goals, timeframe, keywords, relationships,
identifiers, sameAs). The high-volume operational data (tasks, issues,
posts, comments, members) lives in separate tables and is **never** fed
to the matcher.

## Quick start

> Spec-only today — the commands below describe the intended shape once
> the crate is generated (`loco new`, stripped of the auth starter).

Requires PostgreSQL.

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/plan_service_development
cargo loco start        # migrations auto-run in development

# Create
curl -s localhost:5150/api/v1/plans -H 'content-type: application/json' \
  -d '{"name":"EHR Migration Programme","plan_type":"Programme",
       "goals":[{"title":"Cut over by Q4","target_date":"2026-12-01"}]}'

# Name search
curl -s 'localhost:5150/api/v1/plans/search?q=migration'

# Match an explicit query against candidates (no persistence)
curl -s localhost:5150/api/v1/plans/match -H 'content-type: application/json' \
  -d '{"query":{"name":"EHR Migration Programme"},"candidates":[{"name":"EHR Migration"}]}'

# Merge a duplicate into a survivor (the survivor is `main_pid`)
curl -s localhost:5150/api/v1/plans/merge -H 'content-type: application/json' \
  -d '{"main_pid":"<survivor-uuid>","duplicate_pid":"<duplicate-uuid>"}'

# Add a task to a plan
curl -s localhost:5150/api/v1/plans/<pid>/tasks -H 'content-type: application/json' \
  -d '{"title":"Provision staging cluster","status":"Todo","estimate":8,"remaining":8}'

# Authenticated request: present a short-lived PASETO v4.public token
# minted by the auth-service (front-ends use a BFF + cookie session; the
# BFF holds the session and supplies this bearer server-side).
curl -s localhost:5150/api/v1/plans/whoami \
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

Spec-only scaffold. Planned: CRUD + `ILIKE` name search + matching +
record merge + operational sub-resources (goals / tasks / issues / posts
/ comments / members) + derived timeline / burndown views + cross-service
entity links + audit log + in-memory event streaming (durable-bus Phase 1)
+ OpenAPI/Swagger + Prometheus metrics + offline PASETO v4.public
verification (published Ed25519 key) + blanket `/api/*` auth enforcement
(off by default, gated by `PLAN_REQUIRE_AUTH`) + payload validation.
Deferred (see [spec §13](./spec/index.md)): Tantivy full-text/fuzzy
search, durable event bus Phases 2–3 (outbox → Fluvio), privacy,
front-end merge action, bulk import/export, gRPC. Auth credentials are
issued by the central
[authentication-service](../../authentication/authentication-service-with-loco):
the human session is a server-side cookie session, and peers verify a
short-lived PASETO v4.public token offline. See
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(source of truth; RS256/JWKS not used).

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
