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
| Framework | loco.rs 1.0.1 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration` 2.0). |
| Build / test | `cargo build` · `cargo test` (DB-free: `tests/matching.rs` + controller 422 pin) · `cargo test -- --ignored` (request tests, need Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `care_pathways` table: `pid`, `name`, `data` (JSONB CarePathway), `active`, soft-delete. |

## API surface

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/care-pathways` | Create (body: `CarePathway`; blank `name` → `422`) → `{pid, name}` |
| GET | `/api/care-pathways` | List active (capped 100) |
| GET | `/api/care-pathways/search?q=` | Tantivy full-text search (`?fuzzy=true`, `?phonetic=true`) |
| GET | `/api/care-pathways/{pid}` | Fetch the stored `CarePathway` (record-level ABAC; a `mask`-obligation allow returns the redacted view) |
| GET | `/api/care-pathways/{pid}/masked` | The masked view: provider name / provider id redacted |
| GET | `/api/care-pathways/{pid}/export` | GDPR right-of-access export (audited as a disclosure; masked when the policy says so) |
| PUT | `/api/care-pathways/{pid}` | Replace payload |
| DELETE | `/api/care-pathways/{pid}` | Soft-delete |
| POST | `/api/care-pathways/match` | Rank a `{query, candidates}` set |
| POST | `/api/care-pathways/check-duplicates` | Match a query against stored pathways |
| POST | `/api/care-pathways/merge` | Merge a duplicate into a survivor (`422` equal pids, `404` unknown) |
| GET | `/api/care-pathways/merges/recent` | Merge-history records |
| GET | `/api/care-pathways/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api/care-pathways/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/care-pathways/events/recent` | In-memory event stream |
| GET | `/api/care-pathways/insights/{directory,coverage,variants,providers,languages}` | Registry lenses: setting/specialty facets, condition-coverage gaps, cross-provider variants, provider directory, language coverage |
| POST/GET | `/api/care-pathways/{pid}/instances` · `/{pid}/cohort` | Enrol a `person:` URN on a pathway; the chronic cohort view |
| — | `/api/instances/{pid}` (+ `/status` `/review` `/urgency` `/team` `/events` `/steps/{s}/complete`) | Instance lifecycle, review cadence, urgency, care team, steps |
| GET | `/api/instances/{caseload,overdue-reviews,care-team-load}` | Derived operational views |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (text-exposition; root path, public under auth enforcement) |

Plus loco's default `/_health`, `/_ping`. Every CRUD action writes an
`audit_logs` row and publishes a `created`/`updated`/`deleted` event.

## MVP scope

CRUD + Tantivy full-text/fuzzy/phonetic search + matching, with payload validation
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
`CARE_PATHWAY_REQUIRE_AUTH`. The durable event bus's
Phase-2 outbox/relay landed (`models/event_outbox.rs`, `src/relay.rs`),
default-off via `CARE_PATHWAY_EVENT_TRANSPORT` (`memory` unless set to
`outbox`). **Tantivy full-text/fuzzy/phonetic search** (`src/search/`)
replaces the `ILIKE` name search and backs search-blocked
`check-duplicates` candidates. **Privacy** (`src/privacy.rs`) provides
field masking (`provider_name` / `provider_id`), the always-masked
`/masked` view, and the audited GDPR `/export`, wired to the ABAC `mask`
obligation via `auth::authorize_record` + `auth::care_pathway_resource_attrs`
(`care_setting`, `sensitive_setting` for the mental-health/palliative
special-category settings). A pathway *template* names no patient, so
the masked field set is thin — see `src/privacy.rs`'s module docs for
why, and for the explicit note that the patient-identifying linkage
(`pathway_instances.subject_ref`) is a separate, not-yet-addressed
surface. The durable bus's real broker sink (BUS-3, `FluvioSink` in
`src/relay.rs`, behind this crate's own `fluvio` Cargo feature, off by
default) landed 2026-08-03, ported from case-service's BUS-1 reference —
see `agents/share/event-bus.md`. Deferred
(spec §13): instance-layer
masking/authz for `subject_ref`, front-end merge
action, terminology-server code-existence checks. The published key set
is fetched over HTTP once at boot when `CARE_PATHWAY_PASETO_KEYS_URL` is
set (fetched set wins; warn + env fallback via
`CARE_PATHWAY_PASETO_KEYS` otherwise — the service always boots); a
periodic refresh loop is a future spec item.

Auth pivot done in this crate: the family moved from RS256 JWT + JWKS to
cookie sessions + short-lived PASETO v4.public verified offline against a
published Ed25519 key (RS256/JWKS decommissioned); the
`CARE_PATHWAY_REQUIRE_AUTH` flag and enforcement semantics are unchanged,
only the credential changed. See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); `src/auth.rs` verifies PASETO via the
`authentication-verifier` crate (0.2, `from_paseto_keys_*`).

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
├── auth.rs                offline PASETO v4.public verification (AuthUser/MaybeAuthUser) via authentication-verifier (RS256/JWKS decommissioned)
├── merge.rs               pure record-merge logic (merge_pathways)
├── openapi.rs             hand-written OpenAPI 3 document
├── privacy.rs             field masking (provider name/id) + GDPR export envelope
├── relay.rs               durable-bus Phase 3 outbox relay (poll/ack loop) + FluvioSink (BUS-3, `fluvio` feature)
├── search/                Tantivy full-text/fuzzy/phonetic index (index.rs schema + mod.rs engine)
├── streaming.rs           CRUD/merge event stream — Phase 1 durable-bus
│                          envelope (Envelope) + EventPublisher seam +
│                          InMemoryPublisher; frozen EventView projection
├── validation.rs          name + condition-code (ICD/SNOMED) checks → 422
├── models/
│   ├── care_pathways.rs   CRUD helpers over the stored payload
│   ├── audit_logs.rs      audit-trail record/query helpers
│   ├── merge_records.rs   merge-history record/query helpers
│   ├── event_outbox.rs    durable-bus Phase 2: OutboxInsert::from_envelope mapping + enqueue (tx-generic) + relay poll/ack
│   └── _entities/{care_pathways,audit_logs,merge_records,event_outbox}.rs  SeaORM entities
migration/src/            …_000001_care_pathways, …_000002_audit_logs, …_000003_merge_records, …_000004_event_outbox
config/                   development/production/test yaml
compose.fluvio.yaml        opt-in local Fluvio broker (`fluvio` feature, BUS-3; not part of CI)
Dockerfile.fluvio-cli       support image for compose.fluvio.yaml's sc-setup step
tests/fluvio_relay.rs       `fluvio`-feature-gated, #[ignore]d live-broker round-trip test
```
