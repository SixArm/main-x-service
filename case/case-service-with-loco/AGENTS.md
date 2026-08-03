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
| GET | `/api/cases/{pid}` | Fetch the stored `Case` (record-level ABAC; a `mask`-obligation allow returns the redacted view) |
| GET | `/api/cases/{pid}/masked` | The masked view: `subjects` / `identifiers` / `same_as` / `case_number` redacted |
| GET | `/api/cases/{pid}/export` | GDPR right-of-access export (audited; masked when the policy says so) |
| PUT | `/api/cases/{pid}` | Replace payload |
| DELETE | `/api/cases/{pid}` | Soft-delete |
| POST | `/api/cases/match` | Rank a `{query, candidates}` set |
| POST | `/api/cases/check-duplicates` | Match a query against stored cases |
| POST | `/api/cases/merge` | Merge a duplicate into a survivor (`422` equal pids, `404` unknown) |
| GET | `/api/cases/merges/recent` | Merge-history records |
| GET | `/api/cases/whoami` | Verified PASETO-token claims (`401` without one) |
| GET | `/api/cases/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/cases/events/recent` | In-memory event stream |
| POST | `/api/cases/import` | Bulk import (multipart JSONL/CSV upload) → `202 {job_id}` |
| GET | `/api/cases/import/{id}` | Import job status + counts + `errors_url` |
| POST | `/api/cases/export` | Bulk export (JSON filter body) → `202 {job_id}` |
| GET | `/api/cases/export/{id}` | Export job status + `download_url` |
| GET | `/api/cases/bulk-jobs` | Recent bulk jobs, newest first |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (root-mounted, public, `text/plain; version=0.0.4`) |

Plus loco's default `/_health`, `/_ping`. Every CRUD action writes an
`audit_logs` row and publishes a `created`/`updated`/`deleted` event.

## MVP scope

CRUD + Tantivy full-text/fuzzy/phonetic search + matching, with payload validation (blank
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
`CASE_EVENT_TRANSPORT=memory`). **Phase 3's real-broker sink** (BUS-1,
landed 2026-08-03) is `FluvioSink` in `src/relay.rs`, behind this
crate's own `fluvio` Cargo feature (off by default): `CASE_FLUVIO_ENDPOINT`
selects it over the default `LoggingSink`; unset without the feature ⇒
unchanged behaviour; **set** without the feature ⇒ the relay refuses to
start (logged, not a silent no-broker fallback that would mark rows
published without reaching a real broker). `compose.fluvio.yaml` +
`Dockerfile.fluvio-cli` provision a local broker for opt-in manual runs
(not part of any automated CI stage). Blanket `/api/*` auth enforcement is
implemented, default-off via `CASE_REQUIRE_AUTH` (activation is a
deployment decision). **Tantivy full-text/fuzzy/phonetic search**
(`src/search/`) replaces the `ILIKE` title search and backs
search-blocked `check-duplicates` candidates (spec §13 T-6). **Privacy**
was already partial here — `mask_case` (in `controllers/cases.rs`) has
redacted `subjects` / `identifiers` / `same_as` / `case_number` since
the ABAC work landed, wired to the `mask` obligation on `GET /{pid}`.
What was missing — the always-masked `GET /{pid}/masked` view and the
audited `GET /{pid}/export` GDPR envelope — landed too (spec §13,
2026-08-02), reusing the existing `disclosure::action::EXPORT`
machinery for the export's HIPAA §164.528 accounting. **Bulk
import/export** (spec §8.7/§13, BLK-5, landed 2026-08-03) is wired:
async job-based JSONL/CSV import + export via `src/bulk/` and a loco
`BackgroundWorker`, stable-keyed on the agency-scoped
`(agency_id, case_number)` pair then `pid`, reusing `mask_case` as the
default export redaction and gating every export's audit write ahead of
job completion (SEC-B8). No Parquet, no S3 in this rollout; see
`spec/index.md` §8.7 for the documented SEC-B3 concurrency and
per-row-ABAC scope limitations. Deferred (spec §13): BUS-2 (link-graph
Fluvio consumer) and BUS-3 (roll `FluvioSink` to the other nine
services), front-end merge action.

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
├── bulk/                  bulk import/export (BLK-5): mod · row (BulkCaseRow) · stable_key ·
│                          columns · csv · jsonl · error_report · store (async ArtifactStore) ·
│                          pipeline (process_import_job/process_export_job) · worker · handlers
├── models/
│   ├── cases.rs           CRUD helpers over the stored payload (+ find_by_agency_case_number)
│   ├── audit_logs.rs      audit-trail record/query helpers
│   ├── merge_records.rs   merge-history record/query helpers
│   ├── bulk_jobs.rs       bulk-job CRUD + idempotency helpers (BLK-5)
│   ├── review_queue.rs    duplicate-review-queue CRUD (BLK-5; raw SQL, provenance from the start)
│   └── _entities/{cases,audit_logs,merge_records,bulk_jobs}.rs  SeaORM entities
migration/src/            …_000001_cases, …_000002_audit_logs, …_000003_merge_records,
                          …_000012_review_queue, …_000013_bulk_jobs
config/                   development/production/test yaml
```

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`, `entity-ref`,
`case-matcher`) live outside `case/case-service-with-loco/`:

```sh
podman build -f case/case-service-with-loco/Dockerfile \
  -t case-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /_health` returns `200`. This exercise found and
fixed two real bugs: (1) `src/compliance/soup.rs` embeds the IEC 62304
SOUP register via a relative `include_str!` from `compliance/soup.tsv`
at the crate root — the Dockerfile now copies that directory
explicitly. (2) `config/production.yaml`'s `mailer.smtp.auth.user`/
`password` used an unquoted Tera `{{ get_env(name="…", default="") }}`
call, which renders as YAML `null` (not `""`) when the env var is
unset — loco's `SmtpAuth` fields are `String`, not `Option<String>`, so
this failed config parsing at boot with "invalid type: unit value,
expected a string". This crate's `.gitignore` also excluded
`config/production.yaml` entirely (a loco scaffold default nobody had
removed), which is why the SMTP bug had never been caught — the file
never left this machine, so no other checkout could exercise it. Both
are fixed (the file is now tracked; see the `.gitignore` for the
reasoning). See `.containerignore` at the repository root (excludes
every crate's `target/`, or the build context would try to copy
hundreds of GB of build artifacts). The wired multi-service
`examples/compose/` stacks (DEP-1) that build on this are not yet
written.
