## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Database schema | 13 tables, SeaORM entities, indexes, audit triggers |
| Domain model | Full schema.org/Place property coverage including PostalAddress, GeoCoordinates, hierarchy |
| Matching | Name (Jaro-Winkler + Soundex) + Geo (Haversine) + Address (weighted) + Identifier (GLN deterministic) |
| Search | Tantivy full-text + fuzzy index (`q` / `limit` / `offset` / `fuzzy` / `mask_sensitive`, `X-Total-Count`/`X-Limit`/`X-Offset` headers); geo-radius `GET /api/places/nearby` (bounding-box SQL pre-filter + `within_radius` Haversine, same pagination headers) — T-9 |
| REST API | 17 endpoints + OpenAPI/Swagger + CORS + structured errors |
| FHIR R5 API | `Location` resource: read/create/update/delete/search at `/fhir/Location{,/{id}}` + `GET /fhir/metadata` `CapabilityStatement`; reuses the native validators, event/audit path, and blanket auth+ABAC guard (T-11) |
| Integrity verification | SHA-256 + SHA3-256 digests and a keyed HMAC-SHA256 MAC over place records and `audit_log` rows, via the shared `integrity-mac` crate; `GET /api/records/verify` + `GET /api/audit/verify`; default off (`mac_absent`, not a mismatch) until `PLACE_INTEGRITY_MAC_KEY`/`_KEY_FILE` is configured — no hash chain / external-witness checkpoint yet (unlike person/worker/care-pathway/case) |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher; durable-bus Phase 2 outbox + Phase 3 relay (`PLACE_EVENT_TRANSPORT=outbox`, default `memory`), with a real-broker `FluvioSink` behind this crate's own `fluvio` Cargo feature (off by default) — `PLACE_FLUVIO_ENDPOINT` selects it over the default `LoggingSink` (T-12, T-12b, T-12c/BUS-3) |
| Audit log | AuditLogRepository with old / new JSON |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alternate-name + link + soft-delete + snapshot + event |
| Validation | Coordinate bounds, GLN GS1 check digit, URL protocol, telephone format, opening-hours `HH:MM` times, address completeness, `422` |
| Normalisation | Title-case locality, uppercase region/country, abbreviation expansion |
| Privacy | Phone / fax masking, geo-coordinate rounding (2 dp), GDPR export |
| Authentication (peer verification) | Offline PASETO v4.public (Ed25519) bearer verification via `authentication-verifier` 0.3; `AuthUser` extractor + `GET /api/whoami`; env-configured key set (T-8) |
| Authentication (blanket enforcement) | Default-off `PLACE_REQUIRE_AUTH`-gated middleware on both router surfaces: valid PASETO bearer required on every route except the public allow-list (`/api/health`, `/_health`, `/_ping`, `/api-docs/openapi.json`, `/swagger-ui*`, `/metrics.prom`); pure `auth::enforce` decision + DB-free test matrix (T-8) |
| Authentication (boot-time key fetch) | `PLACE_PASETO_KEYS_URL` set ⇒ key set fetched over HTTP once at boot (`state::boot_verifier` in `after_routes`, before middleware capture; fetched set wins; failure warn-logs and falls back to `PLACE_PASETO_KEYS`/empty — always boots); no refresh loop (rotation re-fetch is roadmap) (T-8) |
| Authorization (ABAC) | Inside the blanket guard: action derived from method + destructive named POSTs (`/merge`, `/deduplicate`, `/import`); shared `authentication-verifier` 0.3 engine evaluates `PLACE_ABAC_POLICY`/`_FILE` (else the built-in default policy) over the token's `attrs` claim; first-match-wins, default allow-read / deny-mutation; `401` vs `403` split with deciding-rule reason; DB-free §7 test matrix (T-8) |
| Tests | 221 unit (`cargo test --lib`, +2 DB-gated `#[ignore]`) + 86 integration (incl. 14 bridge; +9 DB/broker-gated `#[ignore]`, incl. 5 for T-9's `nearby` + search `offset`) + 16 Criterion benchmarks |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| PostGIS-backed spatial queries (the `nearby` bounding-box pre-filter, T-9, is the SQL-range interim; PostGIS spatial indexing is the further step) | T-1 |
| Hierarchy depth queries (recursive CTE) | T-2 |
| Fluvio production publisher (deployment flip: enable at runtime, wire the search-reindex consumer) | T-12b follow-up |
| Event consumers | (no task yet) |
| gRPC API | T-4 |
| OSM import pipeline | T-5 |
| Reverse-geocoding | T-6 |
| GeoJSON export | T-7 |

