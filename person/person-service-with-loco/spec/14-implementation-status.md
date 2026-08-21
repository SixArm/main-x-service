## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture, 40+ dependencies |
| Database schema | 12+ tables, SeaORM entities, indexes, audit triggers |
| Matching | Probabilistic + deterministic; Jaro-Winkler + Levenshtein + Soundex; configurable weights |
| Search | Tantivy 11-field index; fuzzy + phonetic + bulk + blocking |
| REST API | 35+ endpoints (person CRUD, search, match/dedup/merge, review queue, links, bulk import/export, privacy, audit/compliance) + OpenAPI/Swagger + CORS + structured errors — see [agents/restful.md](../agents/restful.md) for the full table |
| FHIR R5 | `Patient` primary resource (full CRUD + search) + `Person` read-only alias, `GET /fhir/metadata` `CapabilityStatement`, searchset `Bundle`, `OperationOutcome` errors (T-11) |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | In-memory publisher (default) or a Postgres transactional outbox (`PERSON_EVENT_TRANSPORT=outbox`) drained by a relay to `LoggingSink` or, behind the `fluvio` feature, a real-broker `FluvioSink` (Created / Updated / Deleted / Merged / Linked / Unlinked) |
| Audit log | AuditLogRepository with old / new JSON + user context; hash-chained with checkpointing and out-of-band-edit / wholesale-deletion detection (`src/compliance/`) |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event, locked `FOR UPDATE`, self-merge rejected, audit written in-transaction |
| Validation | Required fields, format checks, phone normalisation, address standardisation, input-size caps, `422` |
| Privacy | Field masking, GDPR export, GDPR erasure (`POST /{id}/erase`), disclosure accounting (HIPAA §164.528), consent model |
| Cross-service links | `entity_links` write side: `same_identity` (person↔worker) + `works_at`/`member_of` (person→organization); `linked`/`unlinked` events under the outbox transport; bulk reconciliation pull for the link-graph aggregator (T-9, T-22) |
| Bulk import/export | Async loco `worker` jobs; JSONL + CSV + Parquet (feature-gated, export-only) formats; local-filesystem or S3-compatible (feature-gated) artifact store; keyless-row → duplicate-detection → review-queue routing; masking + audit on every export (T-10, BLK-2/3/4) |
| Authentication (peer verification) | Offline PASETO v4.public (Ed25519) bearer verification via `authentication-verifier` 0.3; `AuthUser` extractor + `GET /api/whoami`; env-configured key set (T-1a) |
| Authentication (blanket enforcement) | Default-off `/api/*` enforcement middleware behind `PERSON_REQUIRE_AUTH` (lenient parse), public allow-list (health, OpenAPI/Swagger, metrics), layered on both router surfaces; DB-free unit-test matrix (T-1b) |
| Authentication (boot-time key fetch) | `PERSON_PASETO_KEYS_URL` fetched once at boot via `Verifier::from_paseto_keys_url` (verifier `fetch` feature); fetched set wins over `PERSON_PASETO_KEYS`; fetch failure warns and falls back to the env path — the service always boots; verifier now lives in a process-wide `ReloadableVerifier` that the blanket guard **and** the `AuthUser` extractors read per request, so a rotation reaches both together; `spawn_key_refresh` re-fetches on `PERSON_PASETO_KEYS_REFRESH_SECS` (default 3600, `0` off) and keeps the current keys on failure; `policy()` is a `ReloadablePolicy` with `spawn_policy_watcher` hot-reloading `PERSON_ABAC_POLICY_FILE` on an mtime change; activation proved end-to-end by `tests/enforcement.rs` (own binary); local-listener + dead-port tokio tests (T-1c fetch item) |
| Authorization (ABAC) | Inside the blanket guard: action derived from method + destructive named POSTs (`/merge`, `/deduplicate`, `/import`); shared `authentication-verifier` 0.3 engine evaluates `PERSON_ABAC_POLICY`/`_FILE` (else the built-in default policy) over the token's `attrs` claim; first-match-wins, default allow-read / deny-mutation; `401` vs `403` split with deciding-rule reason; DB-free §7 test matrix (T-1c) |
| Containers | Multi-stage Dockerfile built with Podman, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |
| Documentation | README, CLAUDE.md, agents/* set, architecture, deploy guide, this spec |

### 14.2 Open gaps

Open gaps drive tasks in §13. Live gap list:

| Gap | Task |
|---|---|
| Event consumers (`src/streaming/consumer.rs` is still a stub) | (no task yet) |
| gRPC API (Tonic stub, no working server) | T-6 |
| Dedup / merge / privacy integration tests (a dedicated end-to-end suite; individual paths are covered by scattered tests today) | T-5 |
| Spec-drift CI guard | T-7 |

Closed since the last full pass: FHIR capability statement + bundle
handling (T-3, T-4 — delivered by T-11, 2026-07-07) and the Fluvio
production sink (T-2 — delivered by BUS-3, 2026-08-03). See §13 for the
detail on each.

