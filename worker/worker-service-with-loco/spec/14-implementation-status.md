## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture, 40+ dependencies |
| Database schema | 12+ tables, SeaORM entities, indexes, audit triggers |
| Matching | Probabilistic + deterministic; Jaro-Winkler + Levenshtein + Soundex; configurable weights |
| Search | Tantivy 11-field index; fuzzy + phonetic + bulk + blocking |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| FHIR R5 | `Worker` bidirectional conversion + search parameters; routes **mounted** via `fhir_routes()` in `App::routes` (and mirrored in `create_router`); pinned by `tests/api_integration_test.rs::test_fhir_worker_route_is_mounted` |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher (Created / Updated / Deleted / Merged / Linked / Unlinked) |
| Audit log | AuditLogRepository with old / new JSON + user context |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, phone normalisation, address standardisation, `422` |
| Privacy | Field masking, GDPR export, consent model |
| Authentication (peer verification) | Offline PASETO v4.public (Ed25519) bearer verification via `authentication-verifier` 0.2; `AuthUser` extractor + `GET /api/v1/whoami`; env-configured key set (T-1a) |
| Containers | Multi-stage Dockerfile built with Podman, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| Authentication — blanket enforcement (peer PASETO verification delivered, T-1a) | T-1b |
| Fluvio production publisher | T-2 |
| FHIR capability statement | T-3 |
| FHIR bundle (full) | T-3 |
| FHIR Organization resource | T-4 |
| Event consumers | (no task yet) |
| Dedup / merge / privacy integration tests | T-5 |
| gRPC API | T-6 |
| Credential-expiry workflow | T-7 |
| Role / assignment history | T-8 |
| FHIR routes mounted on the loco router | T-9 (done 2026-06-13) |

