## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Database schema | 13 tables, SeaORM entities, indexes, audit triggers |
| Domain model | Full schema.org/Place property coverage including PostalAddress, GeoCoordinates, hierarchy |
| Matching | Name (Jaro-Winkler + Soundex) + Geo (Haversine) + Address (weighted) + Identifier (GLN deterministic) |
| Search | Tantivy index + geo-radius support (app-side Haversine + bbox pre-filter) |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher |
| Audit log | AuditLogRepository with old / new JSON |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alternate-name + link + soft-delete + snapshot + event |
| Validation | Coordinate bounds, GLN check, URL protocol, telephone format, address completeness, `422` |
| Normalisation | Title-case locality, uppercase region/country, abbreviation expansion |
| Privacy | Phone / fax masking, geo-coordinate rounding (2 dp), GDPR export |
| Tests | 171 tests + 16 Criterion benchmarks |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| PostGIS-backed spatial queries | T-1 |
| Hierarchy depth queries (recursive CTE) | T-2 |
| Fluvio production publisher | T-3 |
| Event consumers | (no task yet) |
| gRPC API | T-4 |
| OSM import pipeline | T-5 |
| Reverse-geocoding | T-6 |
| GeoJSON export | T-7 |
| Authentication / authorisation | T-8 |

