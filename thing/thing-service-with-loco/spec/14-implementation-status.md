## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Domain model | schema.org/Thing canonical properties + PropertyValue identifiers |
| Matching | Probabilistic (name / identifier / description / URL / sameAs) + deterministic (DOI / ISBN / ISSN / GTIN / MPN / SerialNumber / UUID short-circuit) + Soundex bonus |
| Search | Tantivy index on name / alternate_names / description / identifier value / URL / same_as |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| Validation | Required `name`, URL formats, per-type identifier formats, normalisation |
| Privacy | Per-field masking (`owner`, identifier `value`), GDPR export, consent model |
| Authentication (peer verification) | Offline PASETO v4.public (Ed25519) bearer verification via `authentication-verifier` 0.3; `AuthUser` extractor + `GET /api/whoami`; env-configured key set (T-4, verification part) |
| Authentication (blanket enforcement, default-off) | Env-gated `THING_REQUIRE_AUTH` middleware (`auth::enforce` + `require_auth_mw`) on every `/api/*` route; public allow-list `/api/health`; wired on both router surfaces; DB-free enforce-matrix + flag-parser tests (T-4, enforcement part) |
| Authentication (boot-time key fetch) | `THING_PASETO_KEYS_URL` set ⇒ key set fetched over HTTP once at boot (`state::boot_verifier` in `after_routes`, before shared-store insert / middleware capture; fetched set wins; failure warn-logs and falls back to `THING_PASETO_KEYS`/empty — always boots); no refresh loop (rotation re-fetch is roadmap) (T-4, fetch part) |
| Authorization (ABAC) | Inside the blanket guard: action derived from method + destructive named POSTs (`/merge`, `/deduplicate`, `/import`); shared `authentication-verifier` 0.3 engine evaluates `THING_ABAC_POLICY`/`_FILE` (else the built-in default policy) over the token's `attrs` claim; first-match-wins, default allow-read / deny-mutation; `401` vs `403` split with deciding-rule reason; DB-free §7 test matrix (T-4, authorization part) |
| Authentication (key rotation + policy hot-reload) | `spawn_key_refresh` re-fetches `THING_PASETO_KEYS_URL` on an interval; `ReloadablePolicy` + `spawn_policy_watcher` reload `THING_ABAC_POLICY_FILE` on mtime change; no restart required (AU-1, 2026-08-01) |
| Prometheus metrics | `GET /metrics.prom` at the application root (T-7) |
| FHIR R5 (`Device`) | Read/create/update/delete/search at `/fhir/Device{,/{id}}` + `GET /fhir/metadata`, `medium` fidelity (T-9) |
| Durable event bus (Phase 2 outbox + Phase 3 relay) | `event_outbox` transactional write inside the entity write's transaction (`THING_EVENT_TRANSPORT=outbox`); relay/retention loop (`THING_EVENT_RELAY`); real-broker `FluvioSink` behind the `fluvio` Cargo feature (`THING_FLUVIO_ENDPOINT`), off by default — `LoggingSink` unchanged (T-10, BUS-3) |
| Stored review queue + decisions | `review_queue` table (normalized-pair UNIQUE upsert); `GET /api/things/review-queue` + `POST /api/things/review-queue/{id}/decision` (2026-07-19) |
| Row-level integrity digests | SHA-256/SHA-3/optional-MAC over the assembled record (child tables included), stamped on every write; `GET /api/records/verify` + `GET /api/audit/verify` (2026-07-28) |
| Server-owned wire fields optional | `POST /api/things` no longer demands `id`/`created_at`/`updated_at`/collection fields it owns and discards (QA-SERVER-FIELDS, 2026-08-04) |
| Tests | 197 unit (`cargo test --lib`) + 6 DB-free/gated integration files + `duplicate_detection` bridge suite + `enforcement` activation proof + Criterion benchmarks |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| Fluvio production publisher wired to a real deployment target | T-1 |
| `Matcher` trait abstraction | T-2 |
| gRPC API | T-3 |
| Embedding-based similarity | T-5 |
| Spec-drift CI guard | T-6 |
| Bulk import / export | T-8 |

