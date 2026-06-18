# AGENTS.md — Plan Service

Entry point for AI coding agents working in the `plan-service` crate: a
registry of **plan** records (project / product / programme / initiative /
portfolio / epic) **and** a project-management tool.

> Read [`spec/index.md`](./spec/index.md) first — the living spec for this
> crate. The entity-wide contract and canonical `Plan` model live in the
> [plan entity spec](../spec/index.md).
>
> **Spec-only today.** No Rust / Cargo crate exists yet; this is the
> inaugural doc-set. The build queue is [spec §13](./spec/index.md).

## What this is

A **loco.rs** service for plan records: CRUD + matching, embedding the
canonical [`plan-matcher`](../plan-matcher-rust-crate). The API DTO **is**
`plan_matcher::Plan` — stored verbatim (JSONB) and matched with the same
type, so there is no separate model or adapter to drift (mirrors
care-pathway). A `Plan` additionally **owns** high-volume operational
sub-resources (goals, tasks, issues, posts, comments, members) in their
own tables — **deliberately excluded** from the matcher payload.

| Question | Answer |
|---|---|
| Framework | loco.rs 0.16 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration`). |
| Build / test | `cargo build` · `cargo test` (DB-free) · `cargo test -- --ignored` (request tests, need Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `plans` table (`pid`, `name`, `data` JSONB `Plan`, `active`, soft-delete) + sub-resource tables + `entity_links`. |

## API surface

Routes under `/api/v1/`. See [spec §9](./spec/index.md) for the full
contract. Highlights:

| Group | Paths |
|---|---|
| Plan CRUD | `POST`/`GET` `/plans`, `GET`/`PUT`/`DELETE` `/plans/{pid}`, `GET /plans/search?q=` |
| Match | `POST /plans/match` · `/check-duplicates` · `/deduplicate` |
| Merge | `POST /plans/merge` (`422` equal pids, `404` unknown) · `GET /plans/merges/recent` |
| Sub-resources | `/plans/{pid}/{goals,tasks,issues,posts,comments,members}` (full CRUD) |
| Derived views | `GET /plans/{pid}/timeline` · `/burndown` |
| Cross-service links | `POST`/`GET`/`DELETE /plans/{pid}/links` (emits `linked`/`unlinked`) |
| Audit / events | `GET /plans/audit/recent` · `/{pid}/audit` · `/events/recent` |
| Auth | `GET /plans/whoami` (`401` without a valid token) |
| Docs / metrics | `GET /api-docs/openapi.json` · `/swagger-ui` · `/metrics.prom` |

Plus loco's default `/_health`, `/_ping`. Every CRUD action (plan and
sub-resource) writes an `audit_logs` row and publishes a
`created`/`updated`/`deleted` (and `merged`/`linked`/`unlinked`) event.

## MVP scope

CRUD + `ILIKE` name search + matching (embed `plan-matcher`,
`MatchingEngine::new(MatchConfig::default())`), real-time create duplicate
detection (`409`), the operational sub-resources + derived timeline /
burndown views, record merge, cross-service entity links (write side),
payload validation (`src/validation.rs`: UUID / PM-tool-id / URI
identifier shapes; non-blank goal titles; BCP-47 `in_language`),
OpenAPI 3 + Swagger UI, an audit log + in-memory event stream (durable-bus
Phase 1 — see `agents/share/event-bus.md`), offline PASETO v4.public
verification (`src/auth.rs`, embeds `authentication-verifier`; `/whoami` +
audit `actor`), and blanket `/api/*` auth enforcement wired but **off by
default** — gated by `PLAN_REQUIRE_AUTH`. Deferred (spec §13): Tantivy
full-text/fuzzy search, durable event bus Phases 2–3 (outbox + Fluvio),
privacy, front-end merge action, bulk import/export, published-key fetch
at boot, gRPC.

> Auth model (intended): the human session is a server-side cookie
> session; peers verify a short-lived **PASETO v4.public** token offline
> against the auth-service's published **Ed25519 key** (replacing RS256
> JWT + JWKS). Front-ends use a BFF (no browser token). Source of truth:
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
> (RS256/JWKS not used).

## Golden rules

1. **Spec-first.** Update `spec/index.md` with behavioural changes; the
   entity-wide [plan spec](../spec/index.md) owns the canonical model (§5).
2. **Loco-idiomatic.** Endpoints are loco controllers in `app.rs`; new
   tables are `sea-orm-migration` migrations.
3. **Reuse the matcher type.** Do not fork a `Plan` DTO.
4. **Partition rule.** Operational sub-resources and cross-service
   `entity_links` are **never** fed to the matcher; only the thin `Plan`
   payload is. Within-payload `relationships` **are** a matcher signal.
5. **Auth credentials** come from the central
   [authentication-service](../../authentication/authentication-service-with-loco)
   (cookie session for humans; offline PASETO v4.public for peers).

## Layout (intended, once generated)

```
src/
├── app.rs                    loco Hooks (routes, truncate)
├── bin/main.rs               loco CLI entrypoint
├── controllers/plans.rs      CRUD + match + check-duplicates + deduplicate + merge + audit/events + whoami
├── controllers/{tasks,issues,posts,comments,members,goals}.rs  sub-resource CRUD
├── controllers/views.rs      timeline + burndown derived read views
├── controllers/links.rs      cross-service entity links (write side)
├── controllers/docs.rs       OpenAPI JSON + Swagger UI
├── controllers/metrics.rs    root /metrics.prom Prometheus endpoint
├── metrics.rs                process-wide Prometheus registry
├── auth.rs                   offline PASETO v4.public verification (AuthUser/MaybeAuthUser) via authentication-verifier
├── merge.rs                  pure record-merge logic (merge_plans)
├── openapi.rs                OpenAPI 3 document
├── streaming.rs              CRUD/merge/link event stream — Phase 1 durable-bus envelope + EventPublisher seam + InMemoryPublisher
├── validation.rs             name + goal-title + identifier + BCP-47 checks → 422
├── models/
│   ├── plans.rs              CRUD helpers over the stored payload
│   ├── {tasks,issues,posts,comments,members,goals}.rs  sub-resource helpers
│   ├── entity_links.rs       cross-service link write-side helpers
│   ├── audit_logs.rs         audit-trail record/query helpers
│   ├── merge_records.rs      merge-history record/query helpers
│   └── _entities/…           SeaORM entities
migration/src/                …_000001_plans, …_000002_audit_logs, …_000003_merge_records,
                              …_000004_{tasks,issues,posts,comments,members,goals},
                              …_000005_entity_links
config/                       development/production/test yaml
```
