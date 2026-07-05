## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Database schema | 13 tables, SeaORM entities, indexes, audit triggers |
| Domain model | Full schema.org/Place property coverage including PostalAddress, GeoCoordinates, hierarchy |
| Matching | Name (Jaro-Winkler + Soundex) + Geo (Haversine) + Address (weighted) + Identifier (GLN deterministic) |
| Search | Tantivy full-text + fuzzy index (`q` / `limit` / `fuzzy` / `mask_sensitive`); geo-radius `nearby` + `offset` deferred to T-9 |
| REST API | 14 endpoints + OpenAPI/Swagger + CORS + structured errors |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher |
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
| Tests | 151 unit + 86 integration (incl. 14 bridge) + 16 Criterion benchmarks |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| Geo-radius `nearby` HTTP endpoint + search `offset` | T-9 |
| PostGIS-backed spatial queries | T-1 |
| Hierarchy depth queries (recursive CTE) | T-2 |
| Fluvio production publisher | T-3 |
| Event consumers | (no task yet) |
| gRPC API | T-4 |
| OSM import pipeline | T-5 |
| Reverse-geocoding | T-6 |
| GeoJSON export | T-7 |

