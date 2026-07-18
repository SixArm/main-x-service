# AGENTS.md — Portfolio Service

Entry point for AI coding agents working in the `project-portfolio-management-service` crate:
a registry of **work-item** records — across **four matchable kinds**
(Portfolio / Project / Product / Program) — **and** a project-management
tool.

> Read [`spec/index.md`](./spec/index.md) first — the living spec for this
> crate. The entity-wide contract and canonical `WorkItem` model live in
> the [portfolio entity spec](../spec/index.md).
>
> **Implemented (MVP, v0.1.0).** The crate builds, `cargo test` is green
> (DB-free unit + matcher-embedding + JSON round-trip; the request suite is
> `#[ignore]`d, needs Postgres), `clippy --all-targets --all-features -- -D
> warnings` is clean, `cargo fmt` is clean, zero `#[allow]`. Scope shipped:
> four-collection CRUD + within-collection matching + validation + record
> merge + audit + in-memory events + offline PASETO auth + Prometheus +
> OpenAPI/Swagger. **Deferred** (spec §13): the operational sub-resources
> (goals / tasks / issues) + derived views, `deduplicate` + review queue,
> cross-service links, bulk import/export, Tantivy search.
>
> **Persistence note (implementation decision, 2026-06-19).** The four
> collections are realised as **one `work_items` table with a `kind`
> discriminator** (+ a denormalised `portfolio_pid`), not four physical
> tables. The observable contract is unchanged — four REST collections,
> distinct record identities, within-`kind` matching (the matcher's kind
> gate), parent roll-up — and a per-kind physical split is a later
> migration behind the same API.

## What this is

A **loco.rs** service for work-item records: CRUD + matching, embedding
the canonical [`project-portfolio-management-matcher`](../project-portfolio-management-matcher-rust-crate). The
API DTO **is** `project_portfolio_management_matcher::WorkItem` — stored verbatim (JSONB) and
matched with the same type, so there is no separate model or adapter to
drift (mirrors care-pathway). A `WorkItem` carries a required `kind`
discriminator (Portfolio / Project / Product / Program); each kind is its
**own REST collection** (realised as one `work_items` table keyed by
`kind` — see the persistence note above). A Portfolio is the umbrella;
Projects / Products / Programs sit **under** a portfolio (a `portfolio_ref`
to their parent). A work item is additionally intended to **own**
high-volume operational sub-resources (goals, tasks, issues) — **excluded**
from the matcher payload (only goal **titles** bridge in) — which are
**deferred** (spec §13).

| Question | Answer |
|---|---|
| Framework | loco.rs 0.16 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration`). |
| Build / test | `cargo build` · `cargo test` (DB-free) · `cargo test -- --ignored` (request tests, need Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `work_items` table (`pid`, `kind`, `name`, `data` JSONB `WorkItem`, denormalised `portfolio_pid`, `active`, soft-delete) shared by the four collections, keyed by `kind`; + `audit_logs` + `merge_records`. Sub-resource tables + `entity_links` deferred (spec §13). |

## API surface

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

Routes under `/api/`; `{collection}` ∈ `{portfolios, projects,
products, programs}` (identical controller shape each). See
[spec §9](./spec/index.md) for the full contract. Highlights:

| Group | Paths |
|---|---|
| Work-item CRUD | `POST`/`GET` `/{collection}`, `GET`/`PUT`/`DELETE` `/{collection}/{pid}`, `GET /{collection}/search?q=` |
| Match | `POST /{collection}/match` · `/check-duplicates` · `/deduplicate` (within-collection; R-GATE) |
| Merge | `POST /{collection}/merge` (`422` equal pids / cross-kind, `404` unknown) · `GET /{collection}/merges/recent` |
| Sub-resources | `/{collection}/{pid}/{goals,tasks,issues}` (full CRUD) |
| Derived views | `GET /{collection}/{pid}/timeline` · `/burndown` |
| Cross-service links | `POST`/`GET`/`DELETE /{collection}/{pid}/links` (emits `linked`/`unlinked`) |
| Strategy (PPM Phase C) | `/ideas` (+ `vote`/`dismiss`/`convert`) · `/scenarios` (+ `/{pid}/evaluate`/`commit`) · `/objectives` (+ `/{pid}/alignment`) · `/{collection}/{pid}/objectives` · `/{collection}/{pid}/benefits` (+ `/{b_pid}/realize`) |
| Visibility (PPM Phase B) | `POST`/`GET /dependencies` (+ `DELETE /{pid}`) · `GET /portfolios/{pid}/schedule` · `/{collection}/{pid}/milestones` (+ `/{m_pid}/complete`) · `/{collection}/{pid}/allocations` (+ `DELETE /{a_pid}`) · `GET /capacity` · `/reports` (+ `/{pid}/run?format=json|csv`) · `GET /at-a-glance` (ETag) |
| Governance (PPM Phase A) | `POST`/`GET /proposals` (+ `/{pid}` + `submit`/`review`/`approve`/`reject`/`promote`/`duplicates`) · `/{collection}/{pid}/gate-reviews` · `/risks` (+ `/{risk_pid}` + `escalate`) · `/budget-lines` (+ `/{line_pid}/actual`) · `GET /{collection}/{pid}/governance` |
| Audit / events | `GET /{collection}/audit/recent` · `/{pid}/audit` · `/events/recent` |
| Auth | `GET /{collection}/whoami` (`401` without a valid token) |
| Docs / metrics | `GET /api-docs/openapi.json` · `/swagger-ui` · `/metrics.prom` |

Plus loco's default `/_health`, `/_ping`. Every CRUD action (work item and
sub-resource) writes an `audit_logs` row and publishes a
`created`/`updated`/`deleted` (and `merged`/`linked`/`unlinked`) event.
**Matching is within a collection only** — the matcher's R-GATE makes a
cross-`kind` pair score `0.0`, so you never match a project against a
product.

## MVP scope

CRUD + `ILIKE` name search + matching (embed `project-portfolio-management-matcher`,
`MatchingEngine::new(MatchConfig::default())`) across the four
collections, real-time create duplicate detection (`409`,
within-collection), the operational sub-resources (goals / tasks / issues)
+ derived timeline / burndown views, record merge, cross-service entity
links (write side), payload validation (`src/validation.rs`: UUID /
PM-tool-id / URI identifier shapes; non-blank goal titles; BCP-47
`in_language`; child-kind `portfolio_ref`), OpenAPI 3 + Swagger UI, an
audit log + in-memory event stream (durable-bus Phase 1 — see
`agents/share/event-bus.md`), offline PASETO v4.public verification
(`src/auth.rs`, embeds `authentication-verifier`; `/whoami` + audit
`actor`; boot-time published-key fetch when `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL`
is set — fetched key set wins, env `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS` fallback, the
service always boots; spec §13, done 2026-07-04), and blanket `/api/*`
auth enforcement wired but **off by
default** — gated by `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`. Deferred (spec §13):
Tantivy full-text/fuzzy search, durable event bus Phases 2–3 (outbox +
Fluvio), privacy, front-end merge action, bulk import/export, the
`posts` / `comments` / `members` collaboration
sub-resources, gRPC.

> Auth model (intended): the human session is a server-side cookie
> session; peers verify a short-lived **PASETO v4.public** token offline
> against the auth-service's published **Ed25519 key** (replacing RS256
> JWT + JWKS). Front-ends use a BFF (no browser token). Source of truth:
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
> (RS256/JWKS not used).

## Golden rules

1. **Spec-first.** Update `spec/index.md` with behavioural changes; the
   entity-wide [portfolio spec](../spec/index.md) owns the canonical model
   (§5).
2. **Loco-idiomatic.** Endpoints are loco controllers in `app.rs`; new
   tables are `sea-orm-migration` migrations.
3. **Reuse the matcher type.** Do not fork a `WorkItem` DTO.
4. **Four kinds, one core.** Portfolio / Project / Product / Program are
   distinct REST collections sharing one parameterised controller core (a
   leading `{collection}` path segment selects the `kind`); do not fork
   four divergent controllers. The collections are stored in one
   `work_items` table keyed by `kind` (implementation decision — see the
   persistence note at the top); a per-kind physical table split is a later
   migration behind the same API.
5. **Within-kind matching only.** The matcher's R-GATE makes cross-`kind`
   pairs score `0.0`; dedup, check-duplicates, and merge stay scoped to a
   single collection.
6. **Partition rule.** Operational sub-resources and cross-service
   `entity_links` are **never** fed to the matcher; only the thin
   `WorkItem` payload is (goal titles bridge via `data.goals[]`).
   Within-payload `relationships` **are** a matcher signal.
7. **Auth credentials** come from the central
   [authentication-service](../../authentication/authentication-service-with-loco)
   (cookie session for humans; offline PASETO v4.public for peers).

## Layout (intended, once generated)

```
src/
├── app.rs                    loco Hooks (routes, truncate)
├── bin/main.rs               loco CLI entrypoint
├── controllers/work_items.rs shared parameterised CRUD + match + check-duplicates + deduplicate + merge + audit/events + whoami (per kind)
├── controllers/{tasks,issues,goals}.rs  sub-resource CRUD
├── controllers/views.rs      timeline + burndown derived read views
├── controllers/links.rs      cross-service entity links (write side)
├── controllers/docs.rs       OpenAPI JSON + Swagger UI
├── controllers/metrics.rs    root /metrics.prom Prometheus endpoint
├── metrics.rs                process-wide Prometheus registry (per-collection counters)
├── auth.rs                   offline PASETO v4.public verification (AuthUser/MaybeAuthUser) via authentication-verifier
├── merge.rs                  pure record-merge logic (merge_work_items; cross-kind rejected)
├── openapi.rs                OpenAPI 3 document
├── streaming.rs              CRUD/merge/link event stream — Phase 1 durable-bus envelope + EventPublisher seam + InMemoryPublisher
├── validation.rs             name + goal-title + identifier + BCP-47 + portfolio_ref checks → 422
├── models/
│   ├── work_items.rs         CRUD helpers over the stored payload (parameterised by kind)
│   ├── {tasks,issues,goals}.rs  sub-resource helpers
│   ├── entity_links.rs       cross-service link write-side helpers
│   ├── audit_logs.rs         audit-trail record/query helpers
│   ├── merge_records.rs      merge-history record/query helpers
│   └── _entities/…           SeaORM entities
migration/src/                …_000001_portfolios, …_projects, …_products, …_programs,
                              …_000002_audit_logs, …_000003_merge_records,
                              …_000004_{tasks,goals,issues}, …_000005_entity_links
config/                       development/production/test yaml
```
</content>
