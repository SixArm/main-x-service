## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture, 40+ dependencies |
| Database schema | 12+ tables, SeaORM entities, indexes, audit triggers |
| Matching | Probabilistic + deterministic; Jaro-Winkler + Levenshtein + Soundex; configurable weights |
| Search | Tantivy 11-field index; fuzzy + phonetic + bulk + blocking |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| FHIR R5 | Person bidirectional conversion + search parameters + OperationOutcome |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher (Created / Updated / Deleted / Merged / Linked / Unlinked) |
| Audit log | AuditLogRepository with old / new JSON + user context |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, phone normalisation, address standardisation, `422` |
| Privacy | Field masking, GDPR export, consent model |
| Authentication (peer verification) | Offline PASETO v4.public (Ed25519) bearer verification via `authentication-verifier` 0.3; `AuthUser` extractor + `GET /api/whoami`; env-configured key set (T-1a) |
| Authentication (blanket enforcement) | Default-off `/api/*` enforcement middleware behind `PERSON_REQUIRE_AUTH` (lenient parse), public allow-list (health, OpenAPI/Swagger, metrics), layered on both router surfaces; DB-free unit-test matrix (T-1b) |
| Authentication (boot-time key fetch) | `PERSON_PASETO_KEYS_URL` fetched once at boot via `Verifier::from_paseto_keys_url` (verifier `fetch` feature); fetched set wins over `PERSON_PASETO_KEYS`; fetch failure warns and falls back to the env path — the service always boots; verifier swapped into `AppState` before routers/middleware are built; local-listener + dead-port tokio tests (T-1c fetch item) |
| Authorization (ABAC) | Inside the blanket guard: action derived from method + destructive named POSTs (`/merge`, `/deduplicate`, `/import`); shared `authentication-verifier` 0.3 engine evaluates `PERSON_ABAC_POLICY`/`_FILE` (else the built-in default policy) over the token's `attrs` claim; first-match-wins, default allow-read / deny-mutation; `401` vs `403` split with deciding-rule reason; DB-free §7 test matrix (T-1c) |
| Containers | Multi-stage Dockerfile built with Podman, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |
| Documentation | README, CLAUDE.md, AGENTS/* set, architecture, deploy guide, this spec |

### 14.2 Open gaps

Open gaps drive tasks in §13. Live gap list:

| Gap | Task |
|---|---|
| FHIR capability statement | T-4 |
| FHIR bundle (full) | T-3 |
| FHIR Organization resource | (no task yet — open in §16) |
| Fluvio production publisher | T-2 |
| Event consumers | (no task yet) |
| gRPC API | T-6 |
| Dedup / merge / privacy integration tests | T-5 |
| Spec-drift CI guard | T-7 |

