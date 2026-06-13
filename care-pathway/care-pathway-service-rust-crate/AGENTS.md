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

Plus loco's default `/_health`, `/_ping`. Every CRUD action writes an
`audit_logs` row and publishes a `created`/`updated`/`deleted` event.

## MVP scope

CRUD + matching, with `condition_codes` format validation (ICD-10 /
ICD-11 / SNOMED CT SCTID Verhoeff; `src/validation.rs`), OpenAPI 3 +
Swagger UI (`src/openapi.rs`, `controllers/docs.rs`), an audit log +
in-memory event stream on every CRUD/merge (`models/audit_logs.rs`,
`src/streaming.rs`), record merge (`src/merge.rs` + `models/merge_records.rs`,
`POST /merge`), and offline RS256 JWT verification (`src/auth.rs`,
embeds `authentication-verifier`; `/whoami` + audit `actor`). Deferred
(spec §13): Tantivy search, durable event bus, privacy, front-end merge
action, blanket `/api/*` JWT enforcement + JWKS-fetch, terminology-server
code-existence checks.

## Golden rules

1. **Spec-first.** Update `spec/index.md` with behavioural changes.
2. **Loco-idiomatic.** Endpoints are loco controllers in `app.rs`; new
   tables are `sea-orm-migration` migrations.
3. **Reuse the matcher type.** Do not fork a `CarePathway` DTO.
4. **JWT auth** comes from the central
   [authentication-service](../../authentication/authentication-service-rust-crate).

## Layout

```
src/
├── app.rs                 loco Hooks (routes, truncate)
├── bin/main.rs            loco CLI entrypoint
├── controllers/care_pathways.rs   CRUD + match + check-duplicates + merge + audit/events + whoami
├── controllers/docs.rs    OpenAPI JSON + Swagger UI
├── auth.rs                RS256 JWT verification (AuthUser/MaybeAuthUser) via authentication-verifier
├── merge.rs               pure record-merge logic (merge_pathways)
├── openapi.rs             hand-written OpenAPI 3 document
├── streaming.rs           in-memory CRUD/merge event stream (PathwayEvent)
├── validation.rs          name + condition-code (ICD/SNOMED) checks → 422
├── models/
│   ├── care_pathways.rs   CRUD helpers over the stored payload
│   ├── audit_logs.rs      audit-trail record/query helpers
│   ├── merge_records.rs   merge-history record/query helpers
│   └── _entities/{care_pathways,audit_logs,merge_records}.rs  SeaORM entities
migration/src/            …_000001_care_pathways, …_000002_audit_logs, …_000003_merge_records
config/                   development/production/test yaml
```
