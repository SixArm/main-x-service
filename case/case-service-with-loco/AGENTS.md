# AGENTS.md — Case Service

Entry point for AI coding agents working in the `case-service` crate: a
registry of **governmental case** records.

> Read [`spec/index.md`](./spec/index.md) first — the living spec.

## What this is

A **loco.rs** service for case records: CRUD + matching, embedding the
canonical [`case-matcher`](../case-matcher-rust-crate). The API DTO **is**
`case_matcher::Case` — stored verbatim (JSONB) and matched with the same
type, so there is no separate model or adapter to drift.

| Question | Answer |
|---|---|
| Framework | loco.rs 1.0.1 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration` 2.0). |
| Build / test | `cargo build` · `cargo test` (DB-free unit + `tests/matching.rs`) · `cargo test -- --ignored` (request tests, need Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `cases` table: `pid`, `title`, `data` (JSONB Case), `active`, soft-delete. |

## API surface

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/cases` | Create (body: `Case`; blank `title` → `422`) → `{pid, title}` |
| GET | `/api/cases` | List active (capped 100) |
| GET | `/api/cases/search?q=` | Tantivy full-text search (`?fuzzy=true`, `?phonetic=true`) |
| GET | `/api/cases/{pid}` | Fetch the stored `Case` |
| PUT | `/api/cases/{pid}` | Replace payload |
| DELETE | `/api/cases/{pid}` | Soft-delete |
| POST | `/api/cases/match` | Rank a `{query, candidates}` set |
| POST | `/api/cases/check-duplicates` | Match a query against stored cases |
| POST | `/api/cases/merge` | Merge a duplicate into a survivor (`422` equal pids, `404` unknown) |
| GET | `/api/cases/merges/recent` | Merge-history records |
| GET | `/api/cases/whoami` | Verified PASETO-token claims (`401` without one) |
| GET | `/api/cases/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/cases/events/recent` | In-memory event stream |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (root-mounted, public, `text/plain; version=0.0.4`) |

Plus loco's default `/_health`, `/_ping`. Every CRUD action writes an
`audit_logs` row and publishes a `created`/`updated`/`deleted` event.

## MVP scope

CRUD + `ILIKE` title search + matching, with payload validation (blank
title, ISO-8601 `opened_date`, non-blank identifier values / subjects /
keywords; `src/validation.rs`), OpenAPI 3 + Swagger UI (`src/openapi.rs`,
`controllers/docs.rs`), an audit log + in-memory event stream on every
CRUD/merge (`models/audit_logs.rs`, `src/streaming.rs`), record merge
(`src/merge.rs` + `models/merge_records.rs`, `POST /merge`), and offline
PASETO v4.public verification (`src/auth.rs`, embeds
`authentication-verifier`; `/whoami` + audit `actor`). The event stream
is **durable-bus Phase 1**:
`src/streaming.rs` publishes a canonical versioned `Envelope` behind an
`EventPublisher` trait (in-memory `InMemoryPublisher`), and
`/events/recent` returns the flat `EventView { kind, pid, name, seq }`
projection unchanged (see
[`agents/share/event-bus.md`](../../agents/share/event-bus.md) §4–§5).
The durable event bus's Phase-2 transactional outbox + relay landed
(`models/event_outbox.rs`, `src/relay.rs`; default-off via
`CASE_EVENT_TRANSPORT=memory`). Blanket `/api/*` auth enforcement is
implemented, default-off via `CASE_REQUIRE_AUTH` (activation is a
deployment decision). **Tantivy full-text/fuzzy/phonetic search**
(`src/search/`) replaces the `ILIKE` title search and backs
search-blocked `check-duplicates` candidates (spec §13 T-6). Deferred
(spec §13): the durable bus's Phase-3 Fluvio broker sink, privacy,
front-end merge action.

> **Auth pivot done here.** The family moved from RS256 JWT + JWKS to
> cookie sessions + offline **PASETO v4.public** verification (published
> Ed25519 key) — see
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
> (source of truth; RS256/JWKS decommissioned). `src/auth.rs` verifies
> PASETO v4.public via `authentication-verifier`; the
> paseto-keys-over-HTTP fetch landed 2026-07-04 (spec §13): set
> `CASE_PASETO_KEYS_URL` to fetch the published key set once at boot
> (`auth::init` from `App::after_routes`; fetched key set wins, env
> `CASE_PASETO_KEYS` fallback, the service always boots).

## Golden rules

1. **Spec-first.** Update `spec/index.md` with behavioural changes.
2. **Loco-idiomatic.** Endpoints are loco controllers in `app.rs`; new
   tables are `sea-orm-migration` migrations.
3. **Reuse the matcher type.** Do not fork a `Case` DTO.
4. **Auth credentials** come from the central
   [authentication-service](../../authentication/authentication-service-with-loco).

## Layout

```
src/
├── app.rs                 loco Hooks (routes, truncate)
├── bin/main.rs            loco CLI entrypoint
├── controllers/cases.rs   CRUD + match + check-duplicates + merge + audit/events + whoami
├── controllers/docs.rs    OpenAPI JSON + Swagger UI
├── controllers/metrics.rs Prometheus /metrics.prom (root-mounted, public)
├── metrics.rs             process-wide Prometheus registry (CRUD counters + http_requests_total)
├── auth.rs                offline PASETO v4.public verification (AuthUser/MaybeAuthUser) via authentication-verifier
├── merge.rs               pure record-merge logic (merge_cases)
├── openapi.rs             hand-written OpenAPI 3 document
├── search/                Tantivy full-text/fuzzy/phonetic index (index.rs schema + mod.rs engine)
├── streaming.rs           durable-bus Phase 1: Envelope + EventPublisher seam (in-memory); indexes/deindexes on every write
├── tasks/search.rs        `search_reindex` CLI task + boot-time rebuild-if-empty
├── validation.rs          title + opened_date + identifier/subject/keyword checks → 422
├── models/
│   ├── cases.rs           CRUD helpers over the stored payload
│   ├── audit_logs.rs      audit-trail record/query helpers
│   ├── merge_records.rs   merge-history record/query helpers
│   └── _entities/{cases,audit_logs,merge_records}.rs  SeaORM entities
migration/src/            …_000001_cases, …_000002_audit_logs, …_000003_merge_records
config/                   development/production/test yaml
```
