## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture, 40+ dependencies |
| Database schema | 12+ tables, SeaORM entities, indexes, audit triggers |
| Matching | Probabilistic + deterministic; Jaro-Winkler + Levenshtein + Soundex; configurable weights |
| Search | Tantivy 11-field index; fuzzy + phonetic + bulk + blocking |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| FHIR R5 | `Practitioner` bidirectional conversion + search parameters; routes **mounted** via `fhir_routes()` in `App::routes` (and mirrored in `create_router`); pinned by `tests/api_integration_test.rs::test_fhir_practitioner_route_is_mounted` |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher (Created / Updated / Deleted / Merged / Linked / Unlinked) |
| Audit log | AuditLogRepository with old / new JSON + user context |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, phone normalisation, address standardisation, `422` |
| Privacy | Field masking, GDPR export, consent model |
| Assessments | Aptitude / personality / psychometric / selection tests: `worker_assessments` table + domain model with the category↔scale rule, lifecycle machine, and score bands; six endpoints under `/api/workers/{id}/assessments` + the derived `assessment-profile`; validation, worker-level ABAC + `mask` obligation on every read path, audit on read and mutation (T-14, done 2026-07-23) |
| Authentication (peer verification) | Offline PASETO v4.public (Ed25519) bearer verification via `authentication-verifier` 0.3; `AuthUser` extractor + `GET /api/whoami`; env-configured key set (T-1a) |
| Authentication (blanket enforcement) | Default-off `WORKER_REQUIRE_AUTH` middleware on both router surfaces: pure `enforce(...)` + `apply_enforcement` in `src/api/rest/auth.rs`; public allow-list = health/ping, `/api/health`, OpenAPI/Swagger, `/metrics.prom`; DB-free unit-test matrix (T-1b enforcement sub-item, done 2026-07-04) |
| Authentication (boot-time key fetch) | `WORKER_PASETO_KEYS_URL` fetched once at boot via `Verifier::from_paseto_keys_url` (verifier `fetch` feature); fetched set wins over `WORKER_PASETO_KEYS`; fetch failure warns and falls back to the env path — the service always boots; verifier swapped into `AppState` before routers/middleware are built; local-listener + dead-port tokio tests (T-1b fetch sub-item, done 2026-07-04) |
| Authorization (ABAC) | Inside the blanket guard: action derived from method + destructive named POSTs (`/merge`, `/deduplicate`, `/import`); shared `authentication-verifier` 0.3 engine evaluates `WORKER_ABAC_POLICY`/`_FILE` (else the built-in default policy) over the token's `attrs` claim; first-match-wins, default allow-read / deny-mutation; `401` vs `403` split with deciding-rule reason; DB-free §7 test matrix (T-1b, done 2026-07-05) |
| gRPC API | Real `tonic::transport::Server` (T-6, PRO-H11, 2026-09-02) generated from `proto/worker.proto` — `CreateWorker`/`GetWorker`/`ListWorkers`/`DeleteWorker`, delegating to the same `AppState`/`WorkerRepository`/`validate_worker`/duplicate-detection REST uses; blanket-enforcement + record-level ABAC gRPC-side (`grpc_enforce`, `authorize_record`) honour `WORKER_REQUIRE_AUTH` on both surfaces together; spawned alongside the REST router in `App::after_routes` |
| Containers | Multi-stage Dockerfile built with Podman, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| Fluvio production publisher | T-2 |
| FHIR bundle (typed `Bundle`/`BundleEntry`, `POST`/transaction bundles) | T-3 (CapabilityStatement + ad hoc searchset Bundle already done, 2026-07-07) |
| FHIR Organization resource | T-4 |
| Event consumers | (no task yet) |
| Dedup / merge / privacy integration tests | T-5 |
| gRPC API — landed 2026-09-02: a real Tonic server (Create/Get/List/Delete Worker; no Update RPC, no match/merge/search/assessments/FHIR over gRPC yet) | T-6 |
| Credential-expiry workflow | T-7 |
| Role / assignment history | T-8 |
| Assessment front-end views + FHIR `Observation` projection | T-14 follow-ups (not queued) |
| FHIR routes mounted on the loco router | T-9 (done 2026-06-13) |

