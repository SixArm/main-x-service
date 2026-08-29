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
| Framework | loco.rs 1.1.0 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration` 2.0). |
| Build / test | `cargo build` · `cargo test` (DB-free) · `cargo test -- --ignored` (request-level suite; needs Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `organizations` table: `pid`, `name`, `data` (JSONB Organization), `active`, soft-delete. |

## API surface

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/organizations` | Create (body: `Organization`) → `{pid, name}` |
| GET | `/api/organizations?limit=&offset=` | List active, paginated (`X-Total-Count`/`X-Limit`/`X-Offset`; default first 100, `limit` clamps to 500, `offset` beyond 10 000 is `400`) |
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
| POST | `/api/organizations/import` | BLK-5: multipart JSONL/CSV/TSV upload → `202 {job_id}` |
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

**FHIR R5 — family reference implementation.** `GET`/`POST`/`PUT`/`DELETE
/fhir/Organization{,/{id}}`, `GET /fhir/Organization?<params>` (a
searchset `Bundle`; `_id`, `_lastUpdated`, `_count`, `identifier`,
`name`, `address`, `address-city`, `address-postalcode`), and `GET
/fhir/metadata` (the `CapabilityStatement`) — see
[`agents/share/fhir.md`](../../agents/share/fhir.md). `src/fhir/` maps
the stored `organization_matcher::Organization` to a FHIR
`Organization` (`high` fidelity); every non-2xx response is an
`OperationOutcome`; these routes sit behind the same auth+ABAC guard as
`/api/*`. Organization was built first here and is the copy source for
the other in-scope services.

## Scope

CRUD (paginated list/search — `?limit=&offset=`, `X-Total-Count`/
`X-Limit`/`X-Offset`) + matching + **name search** + **record merge** +
a stored **review queue** + **audit log** + **event streaming**
(in-memory + a durable transactional outbox, both default-off transport
selectable) + **OpenAPI/Swagger** + **Prometheus metrics**
(`/metrics.prom`) + **offline PASETO v4.public verification**
(`AuthUser`/`MaybeAuthUser`, `/whoami`, audit/merge `actor`) + **ABAC
policy authorization** inside the blanket guard + a **FHIR R5 API**
(family reference implementation, above) + **header-based API
versioning** (`Accepts-version`, above) + **request-level tests**
(Postgres, `#[ignore]`-gated) are wired. The wire format is snake_case
(`legal_name`, `same_as`, …) and validation failures return `422`.
Blanket `/api/*` (and `/fhir/*`) auth enforcement is implemented
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
validation beyond identifier check-digits (URL/country-code format),
real-time duplicate check on create, and moving the structured FHIR
search onto the index. The published-Ed25519-key set is fetched over
HTTP at boot when `ORGANIZATION_PASETO_KEYS_URL` is set (fetched set
wins; warn + env fallback via `ORGANIZATION_PASETO_KEYS` otherwise —
the service always boots) and **refreshed periodically thereafter**
(`ORGANIZATION_PASETO_KEYS_REFRESH_SECS`, default hourly; AU-2); the
ABAC policy file is likewise watched and hot-reloaded without a
restart. For a quick demo dataset,
`cargo loco task seed_examples` loads the repository's shared fixture
(`examples/data/organizations.jsonl`, 20 rows) via the model-layer
create (no duplicate check, no audit row, no event — deliberate for a
seed task); it refuses to insert into a non-empty table (EX-4).

**BLK-5 async bulk import/export** (`src/bulk/`) is implemented, scoped
to **JSONL + CSV + TSV** (no Parquet; TSV lands 2026-08-21, sharing the
CSV codec via a declared delimiter — never sniffed) and a
**local-filesystem-only**
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

**Durable event bus Phase 3** (`src/relay.rs`) is implemented: a
background relay drains the `event_outbox` table to an `EventSink`,
gated by `ORGANIZATION_EVENT_TRANSPORT=outbox` **and**
`ORGANIZATION_EVENT_RELAY` (both default-off, so it is a no-op by
default). The default sink is a no-broker `LoggingSink`; **`FluvioSink`**
(BUS-3, ported from case-service's BUS-1 reference) is the real-broker
sink, behind this crate's own `fluvio` Cargo feature (off by default).
`ORGANIZATION_FLUVIO_ENDPOINT` selects it over `LoggingSink`; set
without the `fluvio` feature compiled in, the relay refuses to start
(logged at `error`) rather than silently falling back —
`compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a local broker
for opt-in manual runs (not part of any automated CI stage).

Auth pivot done in this crate: the family moved from RS256 JWT + JWKS to
cookie sessions + short-lived PASETO v4.public verified offline against a
published Ed25519 key (RS256/JWKS decommissioned); the `*_REQUIRE_AUTH`
flag and enforcement semantics are unchanged, only the credential
changed. See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); `src/auth.rs` verifies PASETO via the
`authentication-verifier` crate (0.9, `from_paseto_keys_*`).

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
├── version.rs              header-based API versioning (`Accepts-version`,
│                           `require_version_mw`, layered in `after_routes`)
├── auth.rs                 offline PASETO v4.public verify + ABAC; AU-2's
│                           `ReloadableVerifier`/`ReloadablePolicy`,
│                           `spawn_key_refresh`, `spawn_policy_watcher`
├── controllers/organizations.rs   CRUD + match + check-duplicates + search
├── controllers/fhir.rs     mounted FHIR R5 surface (`/fhir/Organization`,
│                           `/fhir/metadata`) — family reference implementation
├── fhir/                   FHIR resource structs, to/from-FHIR conversions,
│                           `OperationOutcome`, searchset `Bundle`, search-param parsing
├── privacy.rs             masking + the GDPR export envelope
├── compliance/             row-level integrity: content_hash (SHA-256),
│                           content_hash_sha3 (SHA3-256), content_mac
│                           (HMAC-SHA256, keyed) over organizations +
│                           audit_logs rows (mod.rs, record_integrity.rs,
│                           audit_integrity.rs, mac.rs); default-off
│                           without `ORGANIZATION_INTEGRITY_MAC_KEY`
├── search/                Tantivy index (index.rs schema, mod.rs engine)
├── tasks/search.rs        `search_reindex` + boot self-heal
├── tasks/seed_examples.rs `seed_examples` — loads the repo's demo
│                           fixture (examples/data/organizations.jsonl)
│                           for the tutorials (EX-4)
├── controllers/metrics.rs  GET /metrics.prom (root, public)
├── metrics.rs              process-wide Prometheus registry (OnceLock)
├── relay.rs                durable event bus Phase 3: EventSink seam,
│                           LoggingSink (default), FluvioSink (BUS-3,
│                           behind the `fluvio` Cargo feature), drain +
│                           retention loop
├── bulk/                   BLK-5 async bulk import/export (JSONL + CSV +
│                           TSV — TSV shares the csv codec via a declared
│                           delimiter; columns, csv, jsonl, stable_key,
│                           error_report, pipeline, store, worker, handlers)
├── models/
│   ├── organizations.rs   CRUD helpers over the stored payload
│   ├── bulk_jobs.rs       BLK-5 job CRUD/status helpers
│   ├── review_queue.rs    batch-dedup review queue (now carries `provenance`)
│   └── _entities/{organizations,bulk_jobs}.rs  SeaORM entities
migration/src/            m20220101_000001_organizations, …_000002_audit_logs,
                          …_000003_merge_records, …_000004_event_outbox,
                          m20260719_000001_review_queue,
                          m20260728_000001_integrity_digests,
                          m20260803_000001_review_queue_provenance,
                          m20260803_000002_bulk_jobs
tests/fluvio_relay.rs     `#![cfg(feature = "fluvio")]`-gated, `#[ignore]`d
                          live-broker relay round-trip (BUS-3)
tests/enforcement.rs      AU-2 activation proof: blanket guard + ABAC over
                          the real router (own binary — process-wide OnceLocks)
tests/masking.rs          privacy layer end to end: `mask` obligation on
                          GET, the masked view, the audited export (own binary)
tests/outbox_audit.rs     under `outbox` transport: entity + event_outbox +
                          audit_logs commit in one transaction (own binary)
tests/seed_examples_db.rs seed_examples (EX-4): first run seeds 20 rows,
                          second run is a no-op
tests/requests/bulk.rs    BLK-5 request-level suite (Postgres-gated)
config/                   development/production/test yaml
```

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`, `organization-matcher`)
live outside `organization/organization-service-with-loco/`:

```sh
podman build -f organization/organization-service-with-loco/Dockerfile \
  -t organization-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /_health` returns `200`. This exercise found and
fixed a real bug: `config/production.yaml`'s `mailer.smtp.auth.user`/
`password` used an unquoted Tera `{{ get_env(name="…", default="") }}`
call, which renders as YAML `null` (not `""`) when the env var is
unset — loco's `SmtpAuth` fields are `String`, not `Option<String>`, so
this failed config parsing at boot with "invalid type: unit value,
expected a string". The fix is quoting the Tera output. This crate's
`.gitignore` also excluded `config/production.yaml` entirely (a loco
scaffold default nobody had removed), which is why the bug had never
been caught — the file never left this machine, so no other checkout
could exercise it. Both are fixed (the file is now tracked; see the
`.gitignore` for the reasoning). See `.containerignore` at the
repository root (excludes every crate's `target/`, or the build context
would try to copy hundreds of GB of build artifacts). The wired
multi-service `examples/compose/` stacks (DEP-1) that build on this are
not yet written.
