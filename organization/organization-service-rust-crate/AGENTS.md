# AGENTS.md — Organization Service

Entry point for AI coding agents working in the `organization-service`
crate: a registry of **organization identities**
([schema.org/Organization](https://schema.org/Organization)).

> Read [`spec/index.md`](./spec/index.md) first — the living spec.

## What this is

A **loco.rs** service for organization records: CRUD + matching,
embedding the canonical [`organization-matcher`](../organization-matcher-rust-crate).
Notably, the API DTO **is** `organization_matcher::Organization` — the
service stores it verbatim (JSONB) and matches with the same type, so
there is no separate model or adapter to drift.

| Question | Answer |
|---|---|
| Framework | loco.rs 0.16 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration`). |
| Build / test | `cargo build` · `cargo test` (DB-free) · `cargo test -- --ignored` (request-level suite; needs Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `organizations` table: `pid`, `name`, `data` (JSONB Organization), `active`, soft-delete. |

## API surface

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/organizations` | Create (body: `Organization`) → `{pid, name}` |
| GET | `/api/organizations` | List active (capped 100) |
| GET | `/api/organizations/search?q=` | Case-insensitive name search |
| GET | `/api/organizations/{pid}` | Fetch the stored `Organization` |
| PUT | `/api/organizations/{pid}` | Replace payload |
| DELETE | `/api/organizations/{pid}` | Soft-delete |
| POST | `/api/organizations/match` | Rank a `{query, candidates}` set (no persistence) |
| POST | `/api/organizations/check-duplicates` | Match a query against stored orgs |
| POST | `/api/organizations/merge` | Merge a duplicate into a survivor (`422` equal pids, `404` unknown) |
| GET | `/api/organizations/merges/recent` | Merge-history records |
| GET | `/api/organizations/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api/organizations/audit/recent` · `/{pid}/audit` | Audit trail |
| GET | `/api/organizations/events/recent` | In-memory event stream (frozen `EventView {kind,pid,name,seq}` projection of the canonical `Envelope`) |
| GET | `/swagger-ui` · `/api-docs/openapi.json` | API docs |

Plus loco's default `/_health`, `/_ping`.

## Scope

CRUD + matching + **name search** + **record merge** + **audit log** +
**event streaming** + **OpenAPI/Swagger** + **JWT verification**
(`AuthUser`/`MaybeAuthUser`, `/whoami`, audit/merge `actor`) +
**request-level tests** (Postgres, `#[ignore]`-gated) are wired. The
wire format is snake_case (`legal_name`, `same_as`, …) and validation
failures return `422`. Still deferred (spec §13): Tantivy full-text
(this uses Postgres `ILIKE`), per-field privacy/GDPR export, blanket
`/api/*` JWT enforcement + JWKS-fetch, richer validation.

## Golden rules

1. **Spec-first.** Update `spec/index.md` with behavioural changes.
2. **Loco-idiomatic.** Endpoints are loco controllers in `app.rs`; new
   tables are `sea-orm-migration` migrations.
3. **Reuse the matcher type.** Do not fork an `Organization` DTO — the
   service uses `organization_matcher::Organization` directly.
4. **JWT auth** comes from the central
   [authentication-service](../../authentication/authentication-service-rust-crate) (not
   embedded here).

## Layout

```
src/
├── app.rs                 loco Hooks (routes, truncate)
├── bin/main.rs            loco CLI entrypoint
├── controllers/organizations.rs   CRUD + match + check-duplicates
├── models/
│   ├── organizations.rs   CRUD helpers over the stored payload
│   └── _entities/organizations.rs  SeaORM entity
migration/src/            m20220101_000001_organizations
config/                   development/production/test yaml
```
