# Main Place Service — Specification

Source of truth for the **Main Place Service** crate. This document
articulates what the system *does*, *guarantees*, and *targets*. When
code and this spec disagree, this spec wins — update one or the other
with a deliberate decision recorded here.

For shared infrastructure (web tier, technology stack, observability,
compliance), see the project-root [`spec.md`](../spec.md),
[`AGENTS.md`](../AGENTS.md), and [`agents/share/*`](../agents/share/).
For per-crate reference detail, see [`AGENTS/`](AGENTS/).

## 1. Purpose

The Main Place Service is a centralised registry of **geographic
places**: hospitals, clinics, branches, civic structures, landforms,
administrative areas, business locations — anything modelled by
[schema.org/Place](https://schema.org/Place). It exists to:

- Give callers one canonical place record regardless of how many
  source systems (CRMs, OSM imports, GLN registries, GIS feeds) hold a
  shard.
- Match place records probabilistically and deterministically by name,
  address, geo coordinates, identifier (GLN, FIPS, GNIS, OSM ID), and
  by hierarchy (containedInPlace).
- Detect duplicate places (real-time on create, batch on demand) with
  a review queue and auto-merge for high-confidence cases.
- Expose place identity over REST and (planned) gRPC for downstream
  routing, geo-search, analytics, and mapping systems.
- Emit audit logs and event-streaming records for every CRUD / merge /
  link operation.

Sibling crates: [person](../main-person-service-rust-crate/),
[patient](../main-patient-index-rust-crate/),
[worker](../main-worker-service-rust-crate/),
[thing](../main-thing-service-rust-crate/),
[event](../main-event-service-rust-crate/).

## 2. Domain Model

Based on [schema.org/Place](https://schema.org/Place). Field-by-field
reference: [`AGENTS/models.md`](AGENTS/models.md).

### Place

- **Identity**: UUID `id` + multiple typed `identifiers` (GLN 13-digit
  Global Location Number, FIPS, GNIS, OSM ID, custom).
- **Names**: `name` (primary), `alternate_name` (aliases),
  `description` (long form).
- **Classification**: `place_type` (LocalBusiness, CivicStructure,
  AdministrativeArea, Landform, Residence, TouristAttraction,
  EducationalOrganization, Park, BodyOfWater, Other), `keywords` tags.
- **Address**: `address: PostalAddress` (street_address,
  address_locality, address_region, address_country, postal_code).
- **Geo**: `geo: GeoCoordinates` (latitude, longitude, elevation).
- **Identifiers**: GLN (13-digit), branch_code (short business code),
  plus the typed `identifiers` collection.
- **Hierarchy**: `contained_in_place` (parent) + `contains_place`
  (children).
- **Contact**: `telephone`, `fax_number`, `url`.
- **Operational**: `opening_hours_specification`,
  `is_accessible_for_free`, `public_access`, `smoking_allowed`,
  `maximum_attendee_capacity`, `amenity_feature`.
- **External cross-refs**: `same_as` (URLs to authoritative sources,
  used in deduplication).
- **Audit**: `active` (soft-delete flag), `created_at`, `updated_at`.

### Supporting types

`PostalAddress`, `GeoCoordinates`, `PlaceType`, `PlaceIdentifier`
(GLN / FIPS / GNIS / OSM / Custom), `AmenityFeature`,
`OpeningHoursSpecification`, `Organization`, `MergeRequest` /
`Response` / `Record`, `ReviewQueueItem`, `BatchDeduplicationRequest` /
`Response`, `Consent`.

### Invariants

- `name` must be non-empty.
- `geo.latitude ∈ [-90, 90]`, `geo.longitude ∈ [-180, 180]` when
  present.
- GLN must be exactly 13 digits and pass the GLN check-digit
  algorithm.
- `url` must be `http://` or `https://`.
- `telephone` must be in international `+` format.
- `address`, when present, must have at least one of
  `address_locality`, `postal_code`, or `address_country`.
- A place can be in at most one `contained_in_place`; cycles are
  rejected.
- Soft-delete is the only delete.

## 3. Functional Capabilities

### 3.1 Identity management

- Create / read / update / soft-delete place records.
- Multiple identifiers per place (GLN, FIPS, GNIS, OSM, custom).
- Hierarchy management via `contained_in_place` / `contains_place`.
- Multiple amenity features and opening-hours specifications.
- Event publish on every CRUD.

### 3.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).
Component weights tuned for places:

| Component | Weight | Algorithm |
|---|---|---|
| Name | 0.35 | Jaro-Winkler + Soundex phonetic bonus |
| Geo coordinates | 0.25 | Haversine distance with sigmoid decay |
| Address | 0.20 | Weighted postal/locality/street/region/country |
| Place type | 0.10 | Exact match |
| Identifier | 0.10 | Type + value exact |

Deterministic short-circuit: exact GLN match → 1.0.

Match quality: Certain / Probable / Possible / Unlikely (configurable
thresholds).

### 3.3 Search

Tantivy across indexed fields (name, alternate_name, identifiers,
address components, place_type). Full-text + fuzzy + boolean.
**Geo-radius search**: find places within `R` km of a coordinate via
Haversine + bounding-box pre-filter for efficiency. Pagination via
`offset` + `limit`.

### 3.4 Duplicate detection & merging

- Real-time `409 Conflict` on `POST /api/places` when an existing
  place is within proximity threshold + name match.
- Explicit `POST /api/places/check-duplicates`.
- Batch `POST /api/places/deduplicate`.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge transfers identifiers, alternate names, amenity features,
  opening-hours specifications, `same_as` URLs, hierarchy links;
  appends duplicate's name as `alternate_name` on the survivor; adds
  `Replaces` link; soft-deletes duplicate; records JSON snapshot;
  emits `Merged` event.

### 3.5 Validation & normalisation

Required `name`, coordinate bounds, GLN check digit, URL protocol,
telephone format, address completeness, place hierarchy acyclicity.
Address normalised (title-case locality, uppercase region/country,
expand St./Ave./Rd. abbreviations). Failed validation → `422`.

### 3.6 Privacy

Per-field masking for sensitive contact fields (phone, fax, exact
coordinates rounded to 2 decimal places). GDPR Article 15 export at
`GET /api/places/{id}/export`. Consent model where the place
represents a private residence.

### 3.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp.

## 4. Quality Attributes

| Attribute | Target |
|---|---|
| Scale | Millions of places, thousands of data sources |
| Create latency | ≤ 50 ms p50 |
| Read | ≤ 5 ms p50 |
| Search | ≤ 100 ms p50 |
| Geo-radius search | ≤ 200 ms p50 |
| Match | ≤ 500 ms p99 |
| Throughput | ≥ 1 000 req/sec single instance |
| Availability | HADR; stateless app tier; PostgreSQL replication |
| Fault tolerance | Graceful shutdown; connection pooling; health checks; non-root containers |
| Observability | OTLP traces / metrics / logs; `traceparent` per request |

## 5. Technology Stack

Project-wide stack: [`agents/share/stack-for-rust-loco.md`](../agents/share/stack-for-rust-loco.md).
Crate-specific:

- **Runtime**: Rust 1.93+ 2024 edition · Tokio 1.x
- **Web**: Axum 0.7 · Loco.rs 0.14 · Tera 1.20 · HTMX 2.0 · Alpine.js 3.14 · Lily HTML Headless (NHS UK theme)
- **Data**: PostgreSQL 18+ · SeaORM 1.1
- **Search**: Tantivy 0.22
- **Geo**: `geo` + `haversine` crates for distance, PostGIS for spatial queries
- **API docs**: utoipa 5.x + Swagger UI
- **gRPC**: Tonic 0.12 (stub)
- **Event streaming**: Fluvio 0.23 (in-memory publisher today)
- **Observability**: tracing + opentelemetry + opentelemetry-otlp
- **String matching**: strsim
- **Validation**: validator
- **Testing**: assertables, tempfile, tokio-test, criterion

## 6. API Surface

Complete reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/places/*` + `/api/audit/*` + `/api/health` |
| Geo-radius | `GET /api/places/nearby?lat=&lon=&radius_km=` |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` |

Note: this crate does **not** expose a FHIR R5 surface — places are
not a FHIR-resource concern.

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

## 7. Persistence

PostgreSQL 18+ via SeaORM. Tables (13):

`places`, `place_addresses`, `place_geo_coordinates`,
`place_identifiers`, `place_amenities`, `place_opening_hours`,
`place_same_as`, `place_hierarchy`, `place_links`,
`organizations`, `organization_addresses`, `place_match_scores`,
`audit_log`.

Required PostgreSQL extensions: `pg_stat_statements`, `uuid-ossp`,
`pgcrypto`, `pg_trgm`, `citext`, `unaccent`, **`postgis`** (for
spatial indexing and bounding-box pre-filter on geo-radius search).

## 8. Testing & Quality

Strategy: [`AGENTS/testing.md`](AGENTS/testing.md).

Current coverage (Phase 14–15):

- **Unit tests**: 104 — models (32), matching (45), validation (19),
  privacy (8).
- **Integration tests**: 67 — including `integration_models`,
  `integration_scoring`, `integration_edge_cases` (unicode names,
  geo poles, date line, GLN deterministic override, address
  normalisation edge cases, GDPR field preservation).
- **Criterion benchmarks**: 16 — matching, search, validation,
  privacy (`mask_place`, `mask_place_minimal`, `gdpr_export`,
  `gdpr_export_batch_100`).
- **Total**: 171 tests + 16 benchmarks.

CI: `test.yml`, `quality.yml` (fmt + clippy), `security.yml`.

## 9. Compliance

| Standard | Mechanism |
|---|---|
| GDPR Art. 15 | `GET /api/places/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| ISO/IEC 27001 | Operational controls (deployment-side) |
| schema.org/Place | Domain-model conformance |

Technology compliance: [`agents/share/compliance-for-technology.md`](../agents/share/compliance-for-technology.md).

## 10. Implementation Status

### Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Database schema | 13 tables, SeaORM entities, indexes, audit triggers |
| Domain model | Full schema.org/Place property coverage including PostalAddress, GeoCoordinates, hierarchy |
| Matching | Name (Jaro-Winkler + Soundex) + Geo (Haversine) + Address (weighted) + Identifier (GLN deterministic) |
| Search | Tantivy index + geo-radius support |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher |
| Audit log | AuditLogRepository with old/new JSON |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alternate-name + link + soft-delete + snapshot + event |
| Validation | Coordinate bounds, GLN check, URL protocol, telephone format, address completeness, 422 |
| Normalisation | Title-case locality, uppercase region/country, abbreviation expansion |
| Privacy | Phone/fax masking, geo-coordinate rounding (2 dp), GDPR export |
| Docker | Multi-stage Dockerfile, dev + test Compose |
| Tests | 171 tests + 16 Criterion benchmarks; CI workflows |
| Documentation | README, CLAUDE.md, AGENTS/* set, architecture, deploy guide |

### Open gaps

| Gap | Where |
|---|---|
| Fluvio production publisher | in-memory stub only |
| Event consumers | stub |
| gRPC API | scaffolded, not implemented |
| PostGIS-backed spatial queries | currently fallback to Haversine + bounding-box in app code |
| Place-hierarchy depth queries | linear walk; no recursive CTE today |

## 11. Roadmap

### Authentication & authorisation

JWT middleware, RBAC for editor / curator / read-only / service roles,
rate limiting, security headers.

### Observability & monitoring

Prometheus metrics alongside OTLP, complete OTLP trace exporter,
custom metrics (`place_created`, `geo_search_radius_km_histogram`,
etc.), Grafana dashboards + alerting.

### Performance optimisation

PostGIS spatial indexing, recursive CTEs for hierarchy depth queries,
N+1 batch fixes in the repository, load test at realistic place
volumes, profile and optimise matching hot paths.

### Infrastructure as code

OpenTofu modules, multi-cloud (GCP, AWS, Azure), secrets management,
backup and DR automation.

### Kubernetes

Helm chart, HPA, PVCs for the search index, ingress controllers,
Kubernetes health probes.

### Production readiness

Security audit + pen test, GDPR compliance validation, DR runbook +
drills, backup and restore procedures, incident-response procedures,
CI/CD pipeline.

### Feature enhancements

Complete gRPC, Fluvio production publisher + consumers, **OSM import
pipeline**, **GeoJSON export**, **tile-server integration for the web
UI map view**, **map-tile clustering for high-density searches**,
**reverse-geocoding endpoint**.

## 12. Change control

Material changes to this spec — domain-model fields, match-quality
thresholds, API-surface shape, compliance scope — should land in the
same commit as the code change. The cross-crate uniformity invariant
documented in the project-root [`spec.md`](../spec.md) applies to web
tier files only; this per-crate spec is local to the Main Place Service.
