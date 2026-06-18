# AGENTS.md — Care Pathway Service

Entry point for AI coding agents working in the `care-pathway-service`
crate: a registry of **clinical care-pathway** records.

> Read [`spec/index.md`](./spec/index.md) first — the living spec.

## What this is

A **loco.rs** service for care-pathway records: CRUD + matching,
embedding the canonical [`care-pathway-matcher`](../care-pathway-matcher-rust-crate).
The API DTO **is** `care_pathway_matcher::CarePathway` — stored verbatim
(JSONB) and matched with the same type, so there is no separate model or
adapter to drift.

| Question | Answer |
|---|---|
| Framework | loco.rs 0.16 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration`). |
| Build / test | `cargo build` · `cargo test` (DB-free: `tests/matching.rs` + controller 422 pin) · `cargo test -- --ignored` (request tests, need Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `care_pathways` table: `pid`, `name`, `data` (JSONB CarePathway), `active`, soft-delete. |

## API surface

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/care-pathways` | Create (body: `CarePathway`; blank `name` → `422`) → `{pid, name}` |
| GET | `/api/care-pathways` | List active (capped 100) |
| GET | `/api/care-pathways/search?q=` | Case-insensitive name search (`ILIKE`, cap 50) |
| GET | `/api/care-pathways/{pid}` | Fetch the stored `CarePathway` |
| PUT | `/api/care-pathways/{pid}` | Replace payload |
| DELETE | `/api/care-pathways/{pid}` | Soft-delete |
| POST | `/api/care-pathways/match` | Rank a `{query, candidates}` set |
| POST | `/api/care-pathways/check-duplicates` | Match a query against stored pathways |
| POST | `/api/care-pathways/merge` | Merge a duplicate into a survivor (`422` equal pids, `404` unknown) |
| GET | `/api/care-pathways/merges/recent` | Merge-history records |
| GET | `/api/care-pathways/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api/care-pathways/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/care-pathways/events/recent` | In-memory event stream |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (text-exposition; root path, public under auth enforcement) |

Plus loco's default `/_health`, `/_ping`. Every CRUD action writes an
`audit_logs` row and publishes a `created`/`updated`/`deleted` event.

## MVP scope

CRUD + `ILIKE` name search + matching, with payload validation
(`condition_codes` ICD-10 / ICD-11 / SNOMED CT SCTID Verhoeff;
`identifiers` UUID / DOI shapes; `in_language` BCP-47 syntax;
`src/validation.rs`), OpenAPI 3 +
Swagger UI (`src/openapi.rs`, `controllers/docs.rs`), an audit log +
in-memory event stream on every CRUD/merge (`models/audit_logs.rs`,
`src/streaming.rs`), record merge (`src/merge.rs` + `models/merge_records.rs`,
`POST /merge`), offline PASETO v4.public verification (`src/auth.rs`,
embeds `authentication-verifier`; `/whoami` + audit `actor`), and blanket
`/api/*` auth enforcement (`auth::enforce` + an `after_routes` middleware
in `app.rs`) wired but **off by default** — gated by
`CARE_PATHWAY_REQUIRE_AUTH`. Deferred
(spec §13): Tantivy full-text/fuzzy search (name search via `ILIKE` is
done), search-blocked dedup candidates, durable event bus Phases 2–3
(outbox + Fluvio; Phase 1 in-memory envelope + `EventPublisher` seam is
done — see `agents/share/event-bus.md`), privacy,
front-end merge
action, published-key-over-HTTP fetch at boot, terminology-server
code-existence checks.

Auth pivot in progress: the family moved from RS256 JWT + JWKS to cookie
sessions + short-lived PASETO v4.public verified offline against a
published Ed25519 key (RS256/JWKS decommissioned); the
`CARE_PATHWAY_REQUIRE_AUTH` flag and enforcement semantics are unchanged,
only the credential changes. See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); code follow-up tracked in spec §13.

## Golden rules

1. **Spec-first.** Update `spec/index.md` with behavioural changes.
2. **Loco-idiomatic.** Endpoints are loco controllers in `app.rs`; new
   tables are `sea-orm-migration` migrations.
3. **Reuse the matcher type.** Do not fork a `CarePathway` DTO.
4. **Auth** comes from the central
   [authentication-service](../../authentication/authentication-service-with-loco):
   cookie sessions + offline PASETO v4.public verification.

## Layout

```
src/
├── app.rs                 loco Hooks (routes, truncate)
├── bin/main.rs            loco CLI entrypoint
├── controllers/care_pathways.rs   CRUD + match + check-duplicates + merge + audit/events + whoami
├── controllers/docs.rs    OpenAPI JSON + Swagger UI
├── controllers/metrics.rs root /metrics.prom Prometheus endpoint
├── metrics.rs             process-wide Prometheus registry (CRUD/merge counters + http_requests_total)
├── auth.rs                offline PASETO v4.public verification (AuthUser/MaybeAuthUser) via authentication-verifier (RS256/JWKS decommissioned; code follow-up per spec §13)
├── merge.rs               pure record-merge logic (merge_pathways)
├── openapi.rs             hand-written OpenAPI 3 document
├── streaming.rs           CRUD/merge event stream — Phase 1 durable-bus
│                          envelope (Envelope) + EventPublisher seam +
│                          InMemoryPublisher; frozen EventView projection
├── validation.rs          name + condition-code (ICD/SNOMED) checks → 422
├── models/
│   ├── care_pathways.rs   CRUD helpers over the stored payload
│   ├── audit_logs.rs      audit-trail record/query helpers
│   ├── merge_records.rs   merge-history record/query helpers
│   └── _entities/{care_pathways,audit_logs,merge_records}.rs  SeaORM entities
migration/src/            …_000001_care_pathways, …_000002_audit_logs, …_000003_merge_records
config/                   development/production/test yaml
```
