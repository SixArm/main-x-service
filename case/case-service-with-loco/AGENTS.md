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
| GET | `/api/cases/{pid}/audit/disclosures` | HIPAA §164.528 accounting of disclosures (record-level gated) |
| GET | `/api/cases/audit/verify` | Verify the tamper-evident audit hash chain |
| GET | `/api/cases/records/verify` | Verify row-level `content_hash` integrity |
| GET | `/api/cases/checkpoint` | Take an external-witness chain checkpoint (store off-box) |
| POST | `/api/cases/checkpoint/verify` | Check the chain still honours a recorded checkpoint |
| POST | `/api/cases/{pid}/erase` | GDPR Art. 17 erasure (destructive, `access=admin`) |
| GET | `/api/cases/events/recent` | In-memory event stream |
| POST | `/api/cases/{pid}/links` · GET · `DELETE /{id}` | Cross-service `subject_of` edges (case→person, §8.6) |
| GET | `/api/cases/links` | Bulk edge pull for reconciliation (privileged, audited — SEC-G1) |
| POST | `/api/cases/import` | Bulk import (multipart JSONL/CSV upload) → `202 {job_id}` |
| GET | `/api/cases/import/{id}` | Import job status + counts + `errors_url` |
| POST | `/api/cases/export` | Bulk export (JSON filter body) → `202 {job_id}` |
| GET | `/api/cases/export/{id}` | Export job status + `download_url` |
| GET | `/api/cases/bulk-jobs` | Recent bulk jobs, newest first |
| GET/POST/PUT/DELETE | `/fhir/Task{,/{id}}` · `GET /fhir/metadata` | FHIR R5 `Task` CRUD + search (best-effort mapping) |
| GET | `/api/compliance` | Service identification + build provenance |
| GET | `/api/compliance/sbom` | CycloneDX SBOM + SOUP register |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (root-mounted, public, `text/plain; version=0.0.4`) |

Plus loco's default `/_health`, `/_ping`. Every CRUD action writes an
`audit_logs` row and publishes a `created`/`updated`/`deleted` event. See
[`spec/index.md`](./spec/index.md) §12 for the compliance-endpoint detail
(gating, what each verifies) and §8.6/§9 for the links/FHIR detail.

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
per-row-ABAC scope limitations. Deferred (spec §13): a front-end merge
action (BUS-2/BUS-3 — the link-graph Fluvio consumer and rolling
`FluvioSink` to the other nine services — have both since landed
2026-08-03; no deployment yet points `CASE_FLUVIO_ENDPOINT` at a live
broker).

Also landed and equally real, but easy to miss because they predate the
items above: **cross-service `subject_of` links** (case→person,
`src/controllers/links.rs`, `entity_links` table, §8.6 — case is the
family's *first* entity_links write-side, 2026-07-10); a **FHIR R5
`Task` surface** (`src/fhir/`, mounted at `/fhir/Task{,/{id}}` +
`/fhir/metadata`, best-effort mapping — spec §9/§13); and a
**compliance suite** (`src/compliance/`, 2026-07-25..27) — tamper-evident
audit hash chain + read/disclosure auditing, row-level `content_hash`
record integrity, GDPR Art. 17 erasure (`POST /{pid}/erase`),
external-witness chain checkpoints, a keyed HMAC-SHA256 integrity MAC
(default off, no key ⇒ no MAC written), and a CycloneDX SBOM +
service-identification surface at `/api/compliance*`. Full detail in
[`spec/index.md`](./spec/index.md) §12.0/§12.0.1 — none of this had a
`spec/13` task record until this pass.

For a quick demo dataset, `cargo loco task seed_examples` loads the
repository's shared fixture (`examples/data/cases.jsonl`, 10 rows) via
the model-layer create (no duplicate check, no audit row, no event —
deliberate for a seed task); it refuses to insert into a non-empty
`cases` table (EX-4). It does not create the `subject_of` links to
person records — see `examples/data/case-subject-links.md`.

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
├── app.rs                 loco Hooks (routes, truncate, spawns the outbox relay)
├── bin/main.rs            loco CLI entrypoint
├── version.rs             Accepts-version header negotiation (api-versioning.md)
├── controllers/cases.rs   CRUD + match + check-duplicates + merge + audit/events/verify + erase + whoami
├── controllers/links.rs   cross-service `subject_of` edges (§8.6): create/list/delete + bulk pull
├── controllers/fhir.rs    FHIR R5 `Task` CRUD + search + CapabilityStatement
├── controllers/compliance.rs  service identification + SBOM (`/api/compliance*`)
├── controllers/docs.rs    OpenAPI JSON + Swagger UI
├── controllers/metrics.rs Prometheus /metrics.prom (root-mounted, public)
├── metrics.rs             process-wide Prometheus registry (CRUD counters + http_requests_total)
├── auth.rs                offline PASETO v4.public verification (AuthUser/MaybeAuthUser) via authentication-verifier; ABAC
├── merge.rs               pure record-merge logic (merge_cases)
├── openapi.rs             hand-written OpenAPI 3 document
├── search/                Tantivy full-text/fuzzy/phonetic index (index.rs schema + mod.rs engine)
├── streaming.rs           durable-bus Envelope + EventPublisher/EventSink seam (memory | outbox transports)
├── relay.rs               Phase-3 outbox relay (plain background loop from app.rs, not a loco worker) + FluvioSink
├── compliance/            audit_chain · disclosure · erasure (GDPR Art. 17) · checkpoint ·
│                          record_integrity · mac (keyed HMAC) · soup (SBOM) — see spec §12.0/§12.0.1
├── tasks/search.rs        `search_reindex` CLI task + boot-time rebuild-if-empty
├── tasks/seed_examples.rs `seed_examples` CLI task — loads the repo's
│                          demo fixture (examples/data/cases.jsonl) for
│                          the tutorials (EX-4)
├── tasks/integrity_key.rs `integrity_key` CLI task — generate/inspect the MAC root key
├── tasks/integrity_resign.rs `integrity_resign` CLI task — re-tag rows after a key rotation
├── validation.rs          title + opened_date + identifier/subject/keyword checks → 422
├── bulk/                  bulk import/export (BLK-5): mod · row (BulkCaseRow) · stable_key ·
│                          columns · csv · jsonl · error_report · store (async ArtifactStore) ·
│                          pipeline (process_import_job/process_export_job) · worker · handlers
├── models/
│   ├── cases.rs           CRUD helpers over the stored payload (+ find_by_agency_case_number)
│   ├── audit_logs.rs      audit-trail record/query helpers
│   ├── merge_records.rs   merge-history record/query helpers
│   ├── entity_links.rs    `subject_of` edge upsert/list/soft-delete (§8.6)
│   ├── event_outbox.rs    transactional outbox row helpers (Phase 2)
│   ├── bulk_jobs.rs       bulk-job CRUD + idempotency helpers (BLK-5)
│   ├── review_queue.rs    duplicate-review-queue CRUD (BLK-5; raw SQL, provenance from the start)
│   └── _entities/         SeaORM entities (cases, audit_logs, merge_records, entity_links, event_outbox, bulk_jobs)
├── observability.rs       structured logging + real OpenTelemetry OTLP export (PRO-H12 slice 6 — see below)
migration/src/            …_000001_cases … _000005_entity_links, …_000006_compliance …
                          _000011_integrity_mac, …_000012_review_queue, …_000013_bulk_jobs
config/                   development/production/test yaml
tests/otlp_export.rs       real OTLP/gRPC export proof, in-process collector, no database
tests/otlp_middleware.rs   the mounted `trace_mw` layer proved end to end over a real HTTP request
tests/otlp_collector/      the shared in-process OTLP/gRPC collector both otlp_* binaries use
```

## OpenTelemetry OTLP export

`src/observability.rs` (repo `tasks.md` PRO-H12 slice 6 of 7, landed
2026-09-02) is a close port of care-pathway-service's
`src/observability.rs` — itself a port of organization's, itself
course's, itself person's, itself link-graph-service's, the family's
first working exporter. This crate carried **no** `src/observability`
module at all before this change, and is the **third of the four
loco-idiomatic registries** (organization, care-pathway, case,
portfolio — `src/controllers/`, not `src/api/rest/`) to carry it.
`App::init_logger` installs it (loco's own `EnvFilter` + formatted
layer, plus the `tracing-opentelemetry` bridge over an OTLP/gRPC
exporter); `App::on_shutdown` flushes it. Export is **on by default** —
set `OTLP_ENDPOINT=""` to disable it — at `OTLP_ENDPOINT` (default
`http://localhost:4317`) with `service.name` from `OTLP_SERVICE_NAME`
(default `case-service`); both variables are **deliberately
unprefixed**, matching every other crate that carries this pipeline,
not the per-service `CASE_*` convention `CASE_REQUIRE_AUTH` and its
siblings use.

**Where this crate's shape forced real adaptation**, confirmed rather
than assumed:

- **Exactly one router-construction surface**, unlike the person-style
  crates' two. This crate is genuinely loco-idiomatic: `App::routes` +
  `App::after_routes` in `src/app.rs` is the only place a router gets
  built — confirmed by grepping `src/` and `tests/` for a second
  `Router::new()`/`create_router`: the one hit (`src/auth.rs`) is a
  unit test for the auth middleware itself, not an app-level router.
  `observability::trace_mw` is therefore layered **once**, as the
  outermost middleware in `after_routes` — the same precedent
  `require_auth_mw` and `require_version_mw` already set by being
  layered there.
- **No `tonic` rename needed** — this crate declares no `tonic`
  dependency of its own (no gRPC stub — `agents/share/overview.md`'s
  capability matrix), so the in-process OTLP collector tests' `tonic
  0.14` dev-dependency is a plain, un-renamed dependency, exactly as
  course's, organization's, and care-pathway's are.
- **The same SOUP-register bookkeeping care-pathway's port needed, plus
  one more.** This crate also carries an IEC 62304 SOUP register
  (`compliance/soup.tsv`, verified live by
  `every_direct_dependency_is_annotated`), so the new dependencies each
  needed a `name<TAB>purpose<TAB>safety relevance` row before `cargo
  test --lib` was green — 9 rows here (5 main, 4 test-only) rather than
  care-pathway's 8, because this crate has no existing `reqwest` main
  dependency the middleware test could reuse and so needed its own
  dev-dependency entry.

`tests/otlp_export.rs` and `tests/otlp_middleware.rs` (ported from
care-pathway-service, with `tests/otlp_collector/` — an in-process
OTLP/gRPC collector, unchanged) prove real export against a real gRPC
listener in a normal `cargo test` run: a `tracing` span and a metric
both reach the collector's decoded protobuf, and a served HTTP request
returns a `traceparent` whose trace id matches the exported span's.
None of this needs a database. Landing this raised `cargo test --lib`
from 253 to 261 (8 new `src/observability.rs` unit tests), plus 4 new
tests across the two `tests/otlp_*.rs` binaries. Verified
independently: `cargo fmt --check`, `cargo clippy --all-targets -- -D
warnings`, `cargo deny check`, `cargo bench --no-run`, and the MSRV
check (`cargo +1.96 check --all-targets`) all clean.

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
