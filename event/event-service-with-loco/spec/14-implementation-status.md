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
| Audit log | AuditLogRepository with old / new JSON |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, time-window guards, `422` |
| Privacy | Field masking, GDPR export, consent model |
| Authentication (peer verification) | Offline PASETO v4.public (Ed25519) bearer verification via `authentication-verifier` 0.2; `AuthUser` extractor + `GET /api/v1/whoami`; env-configured key set (T-8, verification part) |
| Authentication (blanket enforcement, default-off) | Env-gated `EVENT_REQUIRE_AUTH` middleware (`auth::enforce` + `require_auth_mw`) on every `/api/v1/*` route; public allow-list `/api/v1/health`; `/fhir/*` stubs out of scope; wired on both router surfaces; DB-free enforce-matrix + flag-parser tests (T-8, enforcement part) |
| Authentication (boot-time key fetch) | `EVENT_PASETO_KEYS_URL` set ⇒ key set fetched over HTTP once at boot (`state::boot_verifier` in `after_routes`, before shared-store insert / middleware capture; fetched set wins; failure warn-logs and falls back to `EVENT_PASETO_KEYS`/empty — always boots); no refresh loop (rotation re-fetch is roadmap) (T-8, fetch part) |
| Containers | Multi-stage Dockerfile built with Podman, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| FHIR Event mapping | T-1 (open question OQ-1) |
| Time-zone-aware fuzzy matching | T-2 |
| Recurrence / RRULE | T-3 |
| Fluvio production publisher | T-4 |
| Event consumers | (no task yet) |
| Dedup / merge / privacy integration tests | T-5 |
| gRPC API | T-6 |
| iCalendar I/O | T-7 |
| Authentication — roles (peer PASETO verification, default-off blanket enforcement, and boot-time published-key HTTP fetch delivered) | T-8 |

