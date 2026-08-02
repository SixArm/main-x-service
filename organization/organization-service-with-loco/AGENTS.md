# AGENTS.md — Organization Service

Entry point for AI coding agents working in the `organization-service`
crate: a registry of **organization identities**
([schema.org/Organization](https://schema.org/Organization)).

> Read [`spec/index.md`](./spec/index.md) first — the living spec for
> this crate. The fuller entity-wide contract (and the `R-DUP` / `T-7` /
> `T-9` / `T-12` task IDs the code comments cite) lives in the umbrella
> spec at [`../spec/index.md`](../spec/index.md).

## What this is

A **loco.rs** service for organization records: CRUD + matching,
embedding the canonical [`organization-matcher`](../organization-matcher-rust-crate).
Notably, the API DTO **is** `organization_matcher::Organization` — the
service stores it verbatim (JSONB) and matches with the same type, so
there is no separate model or adapter to drift.

| Question | Answer |
|---|---|
| Framework | loco.rs 1.0.1 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration` 2.0). |
| Build / test | `cargo build` · `cargo test` (DB-free) · `cargo test -- --ignored` (request-level suite; needs Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `organizations` table: `pid`, `name`, `data` (JSONB Organization), `active`, soft-delete. |

## API surface

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/organizations` | Create (body: `Organization`) → `{pid, name}` |
| GET | `/api/organizations` | List active (capped 100) |
| GET | `/api/organizations/search?q=[&fuzzy][&phonetic]` | Tantivy full-text search (name, legal name, alternate names, identifiers, keywords, address, url); `fuzzy` = typo-tolerant, `phonetic` = Soundex |
| GET | `/api/organizations/{pid}` | Fetch the stored `Organization` (record-level ABAC; a `mask`-obligation allow returns the redacted view) |
| GET | `/api/organizations/{pid}/masked` | The masked view: telephone / email / street line / fiscal identifiers redacted |
| GET | `/api/organizations/{pid}/export` | GDPR right-of-access export (audited; masked when the policy says so) |
| PUT | `/api/organizations/{pid}` | Replace payload |
| DELETE | `/api/organizations/{pid}` | Soft-delete |
| POST | `/api/organizations/match` | Rank a `{query, candidates}` set (no persistence) |
| POST | `/api/organizations/check-duplicates` | Match a query against stored orgs |
| POST | `/api/organizations/deduplicate` | Batch-scan stored orgs pairwise; persist candidates in the stored review queue |
| GET | `/api/organizations/review-queue` | Stored review queue (filter `status`, `limit`) |
| POST | `/api/organizations/review-queue/{id}/decision` | Decide a pending review item (`confirmed` / `rejected`) |
| POST | `/api/organizations/merge` | Merge a duplicate into a survivor (`422` equal pids, `404` unknown) |
| GET | `/api/organizations/merges/recent` | Merge-history records |
| POST | `/api/organizations/import` | BLK-5: multipart JSONL/CSV upload → `202 {job_id}` |
| GET | `/api/organizations/import/{id}` | BLK-5: import job status + counts + `errors_url` |
| POST | `/api/organizations/export` | BLK-5: `{format, q, limit, offset, masking_profile, include_soft_deleted}` → `202 {job_id}` |
| GET | `/api/organizations/export/{id}` | BLK-5: export job status + `download_url` |
| GET | `/api/organizations/bulk-jobs` | BLK-5: recent bulk jobs, newest first |
| GET | `/api/organizations/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api/organizations/audit/recent` · `/{pid}/audit` | Audit trail |
| GET | `/api/organizations/events/recent` | In-memory event stream (frozen `EventView {kind,pid,name,seq}` projection of the canonical `Envelope`) |
| GET | `/swagger-ui` · `/api-docs/openapi.json` | API docs |
| GET | `/metrics.prom` | Prometheus metrics (text-exposition; root path, public) |

Plus loco's default `/_health`, `/_ping`.

## Scope

CRUD + matching + **name search** + **record merge** + **audit log** +
**event streaming** + **OpenAPI/Swagger** + **Prometheus metrics**
(`/metrics.prom`) + **offline PASETO v4.public verification**
(`AuthUser`/`MaybeAuthUser`, `/whoami`, audit/merge `actor`) +
**request-level tests** (Postgres, `#[ignore]`-gated) are wired. The
wire format is snake_case (`legal_name`, `same_as`, …) and validation
failures return `422`. Blanket `/api/*` auth enforcement is implemented
(`auth::enforce`, default-off via `ORGANIZATION_REQUIRE_AUTH`).
**Tantivy full-text search** (`src/search/`) replaced the Postgres
`ILIKE` name search: fuzzy + phonetic retrieval, and `check-duplicates`
blocks on the index rather than scanning. The index is derived — every
hit is resolved against Postgres — and rebuildable via
`cargo loco task search_reindex` (plus an automatic boot rebuild when it
is empty and the table is not). **Privacy** (`src/privacy.rs`) provides
field masking, the masked view, and the audited GDPR export, wired to
the ABAC `mask` obligation; there is deliberately **no consent model**
(an organization is not a data subject — the person service owns the
consent of the people behind it). Still deferred (spec §13): richer
validation, and moving the structured FHIR search onto the index. The published-Ed25519-key
set is fetched over HTTP once at boot when `ORGANIZATION_PASETO_KEYS_URL`
is set (fetched set wins; warn + env fallback via
`ORGANIZATION_PASETO_KEYS` otherwise — the service always boots); a
periodic refresh loop is a future spec item.

**BLK-5 async bulk import/export** (`src/bulk/`) is implemented, scoped
to **JSONL + CSV only** (no Parquet) and a **local-filesystem-only**
artifact store (no S3 backend — the trait is async so a future S3
backend needs no signature change). Stable key: LEI → DUNS → explicit
`pid` (`src/bulk/stable_key.rs`); a keyless row runs the same
search-blocking + matcher duplicate detection `check-duplicates` uses
and is queued in the review queue with `provenance = "import"` (the
`review_queue` table gained a `provenance` column,
`m20260803_000001`). Every written row goes through
`streaming::create_and_emit`/`update_and_emit`, so a bulk-imported
organization gets the same event/audit/search-index side effects as one
created interactively. **Known limitation:** the per-row upsert is not
SEC-B3 advisory-lock-protected (see spec §10.7 "Concurrency" for why —
a locked guard transaction deadlocked under this crate's own
`max_connections: 1` test config, since `streaming::create_and_emit`/
`update_and_emit` are not generic over `ConnectionTrait`); two
importers racing the identical stable key can both create a row.

Auth pivot done in this crate: the family moved from RS256 JWT + JWKS to
cookie sessions + short-lived PASETO v4.public verified offline against a
published Ed25519 key (RS256/JWKS decommissioned); the `*_REQUIRE_AUTH`
flag and enforcement semantics are unchanged, only the credential
changed. See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); `src/auth.rs` verifies PASETO via the
`authentication-verifier` crate (0.2, `from_paseto_keys_*`).

## Golden rules

1. **Spec-first.** Update `spec/index.md` with behavioural changes.
2. **Loco-idiomatic.** Endpoints are loco controllers in `app.rs`; new
   tables are `sea-orm-migration` migrations.
3. **Reuse the matcher type.** Do not fork an `Organization` DTO — the
   service uses `organization_matcher::Organization` directly.
4. **Auth** comes from the central
   [authentication-service](../../authentication/authentication-service-with-loco) (not
   embedded here): cookie sessions + offline PASETO v4.public verification.

## Layout

```
src/
├── app.rs                 loco Hooks (routes, truncate)
├── bin/main.rs            loco CLI entrypoint
├── controllers/organizations.rs   CRUD + match + check-duplicates + search
├── privacy.rs             masking + the GDPR export envelope
├── search/                Tantivy index (index.rs schema, mod.rs engine)
├── tasks/search.rs        `search_reindex` + boot self-heal
├── controllers/metrics.rs  GET /metrics.prom (root, public)
├── metrics.rs              process-wide Prometheus registry (OnceLock)
├── bulk/                   BLK-5 async bulk import/export (JSONL + CSV;
│                           columns, csv, jsonl, stable_key, error_report,
│                           pipeline, store, worker, handlers)
├── models/
│   ├── organizations.rs   CRUD helpers over the stored payload
│   ├── bulk_jobs.rs       BLK-5 job CRUD/status helpers
│   ├── review_queue.rs    batch-dedup review queue (now carries `provenance`)
│   └── _entities/{organizations,bulk_jobs}.rs  SeaORM entities
migration/src/            m20220101_000001_organizations, …_000002_audit_logs,
                          …_000003_merge_records, …_000004_event_outbox,
                          m20260719_000001_review_queue,
                          m20260803_000001_review_queue_provenance,
                          m20260803_000002_bulk_jobs
tests/requests/bulk.rs    BLK-5 request-level suite (Postgres-gated)
config/                   development/production/test yaml
```
