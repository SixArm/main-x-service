## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Database schema | Tables + SeaORM entities + indexes + audit triggers |
| Matching | Probabilistic + deterministic; configurable weights |
| Search | Tantivy index; fuzzy + bulk + name+date blocking |
| REST API | Core endpoints + OpenAPI/Swagger + CORS + structured errors |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher (index-level events) |
| Durable event bus (outbox + relay) | Phase 2 transactional outbox (`event_outbox` table; `Envelope`/`EventTransport`; `OutboxInsert` shares the entity write's tx) + Phase 3 relay (`src/relay.rs`: `EventSink`/`LoggingSink`, `drain_once`, `purge_published`; `relay::spawn` in `after_routes`) + Phase 3 real-broker `FluvioSink` (BUS-3, done 2026-08-03; behind the `fluvio` Cargo feature, off by default; ported from case-service's BUS-1 reference). Gated by `EVENT_EVENT_TRANSPORT=outbox` + `EVENT_EVENT_RELAY` (both off by default); `EVENT_EVENT_RELAY_INTERVAL_SECS` (5) + `EVENT_EVENT_RETENTION_DAYS` (7, enforced by `purge_published`); `EVENT_FLUVIO_ENDPOINT` selects `FluvioSink` over `LoggingSink`, `EVENT_EVENT_TOPIC` (default `mxi.event.events`). BUS-2 (link-graph Fluvio consumer) remains |
| Audit log | AuditLogRepository with old / new JSON |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, time-window guards, `422` |
| Privacy | Field masking, GDPR export, consent model |
| Authentication (peer verification) | Offline PASETO v4.public (Ed25519) bearer verification via `authentication-verifier` 0.3; `AuthUser` extractor + `GET /api/whoami`; env-configured key set (T-8, verification part) |
| Authentication (blanket enforcement, default-off) | Env-gated `EVENT_REQUIRE_AUTH` middleware (`auth::enforce` + `require_auth_mw`) on every `/api/*` route; public allow-list `/api/health`; `/fhir/*` stubs out of scope; wired on both router surfaces; DB-free enforce-matrix + flag-parser tests (T-8, enforcement part) |
| Authentication (boot-time key fetch) | `EVENT_PASETO_KEYS_URL` set ⇒ key set fetched over HTTP once at boot (`state::boot_verifier` in `after_routes`, before shared-store insert / middleware capture; fetched set wins; failure warn-logs and falls back to `EVENT_PASETO_KEYS`/empty — always boots); no refresh loop (rotation re-fetch is roadmap) (T-8, fetch part) |
| Authorization (ABAC) | Inside the blanket guard: action derived from method + destructive named POSTs (`/merge`, `/deduplicate`, `/import`); shared `authentication-verifier` 0.3 engine evaluates `EVENT_ABAC_POLICY`/`_FILE` (else the built-in default policy) over the token's `attrs` claim; first-match-wins, default allow-read / deny-mutation; `401` vs `403` split with deciding-rule reason; DB-free §7 test matrix (T-8, authorization part) |
| Containers | Multi-stage Dockerfile built with Podman, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |
| Keyed integrity verification | `src/compliance/` (`mac`, `record_integrity`, `audit_integrity`) — SHA-256 + SHA3-256 digests and a keyed HMAC-SHA256 MAC over each `Event` record and each `audit_log` row, via the shared `integrity-mac` crate; `GET /api/records/verify` + `GET /api/audit/verify`; default off (no `EVENT_INTEGRITY_MAC_KEY`/`_KEY_FILE` ⇒ no MAC written, rows report `mac_absent`); no hash chain / external-witness checkpoint (unlike person/worker/care-pathway/case) |

### 14.2 Open gaps → tasks

FHIR Event mapping (T-1) and the production Fluvio publisher (T-4) are
**resolved** — T-1 via T-10 (`Appointment` mapping, live), T-4 via T-11
(transactional outbox + relay + `FluvioSink`) — and are no longer
listed here; see their §13 entries for what shipped and what each
literally superseded.

| Gap | Task |
|---|---|
| Time-zone-aware fuzzy matching | T-2 |
| Recurrence / RRULE | T-3 |
| Event consumers (a deployment pointing `EVENT_FLUVIO_ENDPOINT` at a live broker; the link-graph aggregator's own consumer is tracked as BUS-2, elsewhere) | (no task yet) |
| Dedup / merge / privacy integration tests | T-5 |
| gRPC API | T-6 |
| iCalendar I/O | T-7 |
| Bulk import / export | T-9 |

