# Place Service — Living Specification

> **Source of truth.** This document is the canonical artefact for the
> Place Service crate. When code and spec disagree, the spec wins —
> open a task in §13 to bring the code in line, do not silently rewrite
> the spec.
>
> **Three-part PRs.** A behavioural change is one PR: spec edit + code
> edit + test edit. See [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md).

For shared infrastructure (web tier, technology stack, observability,
compliance), see the project-root [`spec.md`](../spec.md),
[`AGENTS.md`](../AGENTS.md), and [`agents/share/*`](../agents/share/).
For per-crate reference detail (architecture, model field tables,
matching constants), see [`AGENTS/`](AGENTS/).

## Table of contents

1. [Purpose and Vision](#1-purpose-and-vision)
2. [Scope](#2-scope)
3. [Stakeholders and Users](#3-stakeholders-and-users)
4. [Glossary](#4-glossary)
5. [Domain Model](#5-domain-model)
6. [Functional Requirements](#6-functional-requirements)
7. [Non-Functional Requirements](#7-non-functional-requirements)
8. [Architecture](#8-architecture)
9. [API Surface](#9-api-surface)
10. [Persistence](#10-persistence)
11. [Testing Strategy](#11-testing-strategy)
12. [Compliance](#12-compliance)
13. [Tasks](#13-tasks)
14. [Implementation Status](#14-implementation-status)
15. [Roadmap](#15-roadmap)
16. [Open Questions](#16-open-questions)
17. [References](#17-references)
18. [Change Control](#18-change-control)

## 1. Purpose and Vision

### 1.1 Purpose

The Place Service is a centralised registry of **geographic places**:
sites, branches, civic structures, landforms, administrative areas,
business locations — anything modelled by
[schema.org/Place](https://schema.org/Place).

### 1.2 Vision

One canonical place record regardless of how many source systems
(CRMs, OSM imports, GLN registries, GIS feeds) hold a shard:

- Match probabilistically and deterministically by name, address,
  geo coordinates, identifier (GLN, FIPS, GNIS, OSM ID), and
  hierarchy (`containedInPlace`).
- Detect duplicates in real time on create *and* in batch on demand
  with a review queue + auto-merge.
- Expose place identity over REST (and gRPC, planned) for downstream
  routing, geo-search, analytics, and mapping systems.
- Provide **geo-radius search**: find places within R km of a
  coordinate via Haversine + bounding-box pre-filter.
- Emit audit logs and event-streaming records for every CRUD / merge
  / link.

### 1.3 Non-goals

- **Not** a tile server — `tile-server integration` is a roadmap
  item (§15) for the web UI map view; the API does not serve tiles.
- **Not** a routing engine — places have coordinates; turn-by-turn
  is out of scope.
- **Not** a geocoder — reverse-geocoding endpoint is a roadmap item.

## 2. Scope

### 2.1 In scope

- Place identity CRUD with soft delete and full audit trail.
- schema.org/Place properties (`name`, `alternate_name`,
  `description`, `place_type`, `address`, `geo`, `telephone`,
  `fax_number`, `url`, opening hours, amenities, accessibility flags).
- Multiple identifiers (GLN, FIPS, GNIS, OSM ID, branch_code, custom).
- Hierarchy (`contained_in_place` / `contains_place`).
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy + boolean search.
- **Geo-radius search** with Haversine + bounding-box pre-filter.
- Real-time + batch duplicate detection with review queue +
  auto-merge.
- Record merging with link tracking and JSON snapshots.
- Per-field privacy masking (phone / fax / coordinate rounding),
  GDPR Article 15 export, consent records.
- REST API (Axum) + gRPC stub.
- Server-rendered web UI.
- PostgreSQL persistence via SeaORM, with PostGIS for spatial.

### 2.2 Out of scope (today)

- FHIR R5 — Places are not a FHIR-resource concern.
- Production Fluvio publisher / consumers.
- PostGIS-backed spatial queries (currently fallback to Haversine +
  bounding-box in app code).
- Recursive CTEs for place-hierarchy depth queries.
- OSM bulk import pipeline.
- Tile-server integration.
- Reverse-geocoding endpoint.

## 3. Stakeholders and Users

| Stakeholder | Interest |
|---|---|
| GIS / mapping integrators | Stable place identity + geo search |
| CRM / scheduling integrators | Place lookup by name / address / hierarchy |
| Operations / DBA | PostgreSQL + PostGIS schema discipline |
| Compliance officer | GDPR export (private residences), consent |
| Other Main X Index crates | Cross-references via `place_id` |

## 4. Glossary

| Term | Meaning |
|---|---|
| **Place** | A geographic location with name, address, geo, type, identifiers |
| **GLN** | Global Location Number — 13-digit deterministic identifier with check digit |
| **GeoCoordinates** | `{ latitude, longitude, elevation? }` in WGS 84 decimal degrees |
| **Hierarchy** | `contained_in_place` (parent) + `contains_place` (children); acyclic |
| **Geo-radius search** | Haversine distance + bounding-box pre-filter |
| **Match quality** | Certain / Probable / Possible / Unlikely buckets |
| **Soft delete** | `is_deleted = true`; rows are never `DELETE`d |

## 5. Domain Model

Field-by-field reference: [`AGENTS/models.md`](AGENTS/models.md).

### 5.1 `Place`

Material aspects:

- **Identity** — UUID `id` + `identifiers: Vec<PlaceIdentifier>` (GLN
  13-digit, FIPS, GNIS, OSM ID, custom). `global_location_number` and
  `branch_code` are shortcuts.
- **Names** — `name` (primary), `alternate_name`, `description`.
- **Classification** — `place_type` (`LocalBusiness`,
  `CivicStructure`, `AdministrativeArea`, `Landform`, `Residence`,
  `TouristAttraction`, `EducationalOrganization`, `Park`,
  `BodyOfWater`, `Hospital`, `School`, `Library`, `Museum`,
  `Restaurant`, `Hotel`, `Airport`, `Other(String)`), `keywords` tags.
- **Address** — `PostalAddress` (`street_address`,
  `address_locality`, `address_region`, `address_country`,
  `postal_code`).
- **Geo** — `GeoCoordinates` (`latitude`, `longitude`, `elevation`).
- **Hierarchy** — `contained_in_place` (parent UUID) + transitive
  `contains_place` (children).
- **Contact** — `telephone`, `fax_number`, `url`.
- **Operational** — `opening_hours_specification`,
  `is_accessible_for_free`, `public_access`, `smoking_allowed`,
  `maximum_attendee_capacity`, `amenity_feature`.
- **External cross-refs** — `same_as` (URLs to authoritative sources,
  used in dedup).
- **Audit** — `is_deleted`, `deleted_at`, `created_at`, `updated_at`.

### 5.2 Supporting types

`PostalAddress`, `GeoCoordinates`, `PlaceType`, `PlaceIdentifier`
(GLN / FIPS / GNIS / OSM / Custom), `AmenityFeature`,
`OpeningHoursSpecification`, `Organization`, `MergeRequest` /
`MergeResponse` / `MergeRecord`, `ReviewQueueItem`,
`BatchDeduplicationRequest` / `Response`, `Consent`.

### 5.3 Invariants

The implementation MUST enforce:

- `name` is non-empty.
- `geo.latitude ∈ [-90, 90]`, `geo.longitude ∈ [-180, 180]` when
  present.
- `GLN` MUST be exactly 13 digits and pass the GLN check-digit
  algorithm.
- `url` MUST use the `http://` or `https://` scheme.
- `telephone` MUST be in international `+` format.
- `address`, when present, MUST have at least one of
  `address_locality`, `postal_code`, or `address_country`.
- A place MUST be in at most one `contained_in_place`; cycles are
  rejected.
- Soft delete is the only delete.

## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete place records.
- Multiple identifiers per place.
- Hierarchy management via `contained_in_place` / `contains_place`.
- Multiple amenity features and opening-hours specifications.
- Automatic event publish on every CRUD.

### 6.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).

Default component weights (sum to 1.0):

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.35 | Jaro-Winkler + Soundex phonetic bonus |
| Geo coordinates | 0.25 | Haversine distance with sigmoid decay |
| Address | 0.20 | Weighted postal / locality / street / region / country |
| Place type | 0.10 | Exact match |
| Identifier | 0.10 | Type + value exact |

Deterministic short-circuit: exact GLN match → 1.0.

Match quality (configurable thresholds):

| Quality | Score |
|---|---|
| Certain | ≥ 0.95 |
| Probable | ≥ 0.80 |
| Possible | ≥ 0.60 |
| Unlikely | < 0.60 |

#### Interoperability with `place-matcher`

The service embeds the sibling `place-matcher` crate (path dependency
declared in `Cargo.toml`) and re-exports it from
`src/matching/mod.rs` as `matcher_lib`. The matcher crate is the
**canonical reference algorithm** — it carries the full
`PlaceCategory` vocabulary (34 variants), `PlaceIdScheme` for
external IDs (Google, OSM nodes/ways/relations, GeoNames, Wikidata,
Foursquare, Here, Mapbox, …), Haversine + Gaussian-decay geo scoring,
weight renormalisation for missing fields, and three tuned config
presets (`strict` / `default` / `lenient`) that the in-service
matcher does not duplicate.

Bridge: [`src/matching/adapter.rs`](src/matching/adapter.rs) exposes
`to_matcher_place(&service::Place) -> place_matcher::Place`. The
projection lifts the service's schema.org-shaped record (`PostalAddress`,
`GeoCoordinates`, `PlaceType`, `Vec<PlaceIdentifier>`) into the
matcher's flat builder shape:

- `name` → `name`; `alternate_name` (Option) → first entry of `alternate_names`
- `place_type` → `category` (12-variant service enum → 34-variant matcher vocabulary, with `Other(s)` flowing through)
- `address.street_address` → `line1`; `address_locality` → `city`; `address_region` → `county`; `postal_code` → `postcode`; `address_country` → `country` (and `country_code_as_iso_3166_1_alpha_2` if 2-character)
- `geo.latitude` / `.longitude` / `.elevation` → bare `f64` slots + `elevation_as_metre`
- `telephone` → `phone`
- `global_location_number` → `add_place_id(Other("GLN"), value)`
- `branch_code` → `add_place_id(Other("BranchCode"), value)`
- `identifiers[]` routed to `PlaceIdScheme` via `map_identifier_scheme` (`OpenStreetMap` → `OsmNode`; `Fips` / `Gnis` / `Custom(s)` → `Other(name)`)
- `maximum_attendee_capacity` → `maximum_capacity_count`

Registry-only fields (`id`, `is_deleted`, `created_at`, `keywords`,
`amenity_features`, `opening_hours`, `description`, `fax_number`,
`url`, `public_access`, `smoking_allowed`, …) are dropped — they
have no matcher counterpart. See
[`AGENTS/matching.md`](AGENTS/matching.md) for the in-service
algorithm and the matcher crate's
[`spec.md §5–§7`](../place-matcher-rust-crate/spec.md) for the
canonical algorithm.

### 6.3 Search

Tantivy across `name`, `alternate_name`, `identifiers`, address
components, `place_type`. Full-text + fuzzy + boolean.

**Geo-radius search** at `GET /api/places/nearby?lat=&lon=&radius_km=`:
Haversine distance with bounding-box pre-filter for efficiency.
PostGIS-backed spatial queries are a roadmap item (today: app-side
fallback).

Pagination via `offset` + `limit`.

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/places` when an existing
  place is within proximity threshold + name match.
- Explicit `POST /api/places/check-duplicates`.
- Batch `POST /api/places/deduplicate`.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge transfers identifiers, alternate names, amenity features,
  opening-hours specifications, `same_as` URLs, hierarchy links;
  appends the duplicate's name as `alternate_name` on the survivor;
  adds a `Replaces` link; soft-deletes the duplicate; records a JSON
  snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

Required `name`; coordinate bounds; GLN check digit; URL protocol;
telephone format; address completeness; place hierarchy acyclicity.
Address normalised (title-case locality, uppercase region / country,
expand St. / Ave. / Rd. abbreviations). Coordinate normalisation
(decimal degrees, WGS 84). Failed validation → `422`.

### 6.6 Privacy

Per-field masking for sensitive contact fields: phone, fax, exact
coordinates rounded to 2 decimal places (~1 km precision). GDPR
Article 15 export at `GET /api/places/{id}/export`. Consent model
where the place represents a private residence.

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp.

## 7. Non-Functional Requirements

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
| Observability | OTLP traces / metrics / logs; `traceparent` per request; Prometheus text-exposition scrape at `GET /metrics.prom` (canonical `/metrics` serves the HTML dashboard) |
| Background jobs | Loco `BackgroundQueue` backed by **PostgreSQL** (`bg_pg`) — same database as application data; no external broker (no Redis, no SQLite) |

## 8. Architecture

### 8.1 Module layout

```
src/
├── lib.rs                 # Library root
├── main.rs                # Binary entry point (REST + gRPC API)
├── models/
│   ├── place.rs           # Place
│   ├── address.rs         # PostalAddress
│   ├── geo.rs             # GeoCoordinates + Haversine
│   ├── place_type.rs      # PlaceType enum
│   ├── identifier.rs      # PlaceIdentifier + IdentifierType (GLN, FIPS, GNIS, OSM)
│   ├── amenity.rs         # AmenityFeature
│   ├── opening_hours.rs   # OpeningHoursSpecification + DayOfWeek
│   └── consent.rs         # Consent (GDPR)
├── matching/
│   ├── name.rs            # Jaro-Winkler
│   ├── address.rs         # Weighted-field comparison
│   ├── geo.rs             # Haversine + sigmoid decay + within_radius
│   ├── identifier.rs      # Exact + has_gln_match
│   ├── phonetic.rs        # Soundex
│   └── scoring.rs         # compute_match, MatchWeights, MatchConfidence
├── validation/            # boundary validators + address normalisation
├── privacy/               # masking + GDPR export
├── api/                   # REST + gRPC (stub)
├── web/                   # Loco app + Tera views
└── bin/web.rs             # cargo run --bin web (binds 0.0.0.0:5150)
```

### 8.2 Layering rules

- `api/*` depends on `models`, `matching`, `validation`, `privacy`.
- `matching` MUST NOT depend on `api`.
- `models` are leaves.

### 8.3 Trait-based abstraction

| Trait | Implementations |
|---|---|
| (No `Matcher` trait yet — `compute_match` is a free function) | — |
| `EventProducer` | `InMemoryEventPublisher` (Fluvio planned) |

## 9. API Surface

Complete reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/places/*` + `/api/audit/*` + `/api/health` |
| Geo-radius | `GET /api/places/nearby?lat=&lon=&radius_km=` |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` |

This crate does **not** expose a FHIR R5 surface — Places are not a
FHIR-resource concern.

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

## 10. Persistence

PostgreSQL 18+ via SeaORM.

### 10.1 Tables (13)

`places`, `place_addresses`, `place_geo_coordinates`,
`place_identifiers`, `place_amenities`, `place_opening_hours`,
`place_same_as`, `place_hierarchy`, `place_links`,
`organizations`, `organization_addresses`,
`place_match_scores`, `audit_log`.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`, **`postgis`** (planned use: spatial indexing
and bounding-box pre-filter on geo-radius search).

## 11. Testing Strategy

Layered: [`AGENTS/testing.md`](AGENTS/testing.md).

- **Unit tests** — `#[cfg(test)]` modules; models (32), matching
  (45), validation (19), privacy (8). 104 tests.
- **Integration tests** — `tests/integration_*.rs`; end-to-end
  workflows + edge cases (unicode names, geo poles, date line, GLN
  deterministic override, address normalisation edge cases, GDPR
  field preservation). 67 tests.
- **Bridge integration tests** — `tests/duplicate_detection.rs`;
  drives service-side records through `adapter::to_matcher_place` and
  asserts on `MatchingEngine::match_places` end-to-end. Covers
  identical clones, name typos, deterministic identifier short-circuits,
  field-routing pinning, and config-preset invariants. 14 tests.
- **Benchmarks** — Criterion: 16 — matching, search, validation,
  privacy.
- **CI** — `test.yml`, `quality.yml`, `security.yml`.

## 12. Compliance

| Standard | Mechanism |
|---|---|
| GDPR Art. 15 | `GET /api/places/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| ISO/IEC 27001 | Operational controls (deployment-side) |
| schema.org/Place | Domain-model conformance |

Technology compliance:
[`agents/share/compliance-for-technology.md`](../agents/share/compliance-for-technology.md).

## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [ ] **T-1 — PostGIS-backed spatial queries.**
  - [ ] Add `geometry(Point, 4326)` column on `place_geo_coordinates`.
  - [ ] GiST index + `ST_DWithin` for geo-radius search.
  - **Acceptance:** geo-radius search ≤ 200 ms p50 at 1 M places.
- [ ] **T-2 — Recursive CTE for place hierarchy depth queries.**
  - [ ] Replace linear walk with `WITH RECURSIVE`.
  - **Acceptance:** "list all descendants of `place_id`" returns
    correctly for ≥ 5 levels deep, ≤ 100 ms p50.
- [ ] **T-3 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag.
  - **Acceptance:** integration test publishes a `PlaceCreated`
    record end-to-end.
- [ ] **T-4 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `PlaceService.GetPlace`
    round-trips a record.
- [ ] **T-5 — OSM import pipeline.**
  - [ ] Streaming PBF reader → Place upserts.
  - [ ] Idempotency via OSM ID.
  - **Acceptance:** import a small `.osm.pbf` extract; spot-checks
    pass against the canonical OSM web view.
- [ ] **T-6 — Reverse-geocoding endpoint.**
  - [ ] `GET /api/places/reverse-geocode?lat=&lon=`.
  - **Acceptance:** known NYC coords return the corresponding
    administrative-area place.
- [ ] **T-7 — GeoJSON export.**
  - [ ] `GET /api/places/{id}.geojson` (Feature with point geometry +
    properties).
  - [ ] `GET /api/places/search.geojson?bbox=` (FeatureCollection).
  - **Acceptance:** `jq -e '.type == "Feature"'` passes.
- [ ] **T-8 — Authentication / authorisation.**
  - [ ] JWT middleware on `/api/*` with editor / curator / read-only
    / service roles.
  - **Acceptance:** unauthenticated requests get `401`; valid token
    + role gets `2xx`.

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
| Web UI | Loco / Tera / HTMX / Alpine / Lily HTML Headless + United Kingdom National Health Service England theme |
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

## 15. Roadmap

- **Authentication & authorisation** — JWT, RBAC for editor /
  curator / read-only / service roles, rate limiting, security
  headers.
- **Observability** — Prometheus alongside OTLP, complete OTLP trace
  exporter, custom metrics (`place_created`,
  `geo_search_radius_km_histogram`), Grafana dashboards + alerting.
- **Performance** — PostGIS spatial indexing, recursive CTEs for
  hierarchy depth, batch fixes.
- **Infrastructure as code** — OpenTofu modules, multi-cloud, secrets,
  backup + DR.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index,
  ingress, probes.
- **Production readiness** — security audit + pen test, GDPR
  validation, DR runbook, backup / restore, CI/CD pipeline.
- **Feature enhancements** — complete gRPC; Fluvio production +
  consumers; **OSM import pipeline**; **GeoJSON export**;
  **tile-server integration for the web UI map view**; **map-tile
  clustering for high-density searches**; **reverse-geocoding
  endpoint**.

## 16. Open Questions

- **OQ-1 — Tile server.** Self-host a tile server (e.g. tegola,
  martin) or proxy to an external one for the web UI map view?
- **OQ-2 — Coordinate precision masking.** Today we round to 2 dp
  (~1 km). Is that the right default for a private-residence
  privacy view, or should we offer multiple buckets (`coarse` /
  `medium` / `precise`)?
- **OQ-3 — Hierarchy cycle detection.** Validation rejects on insert,
  but we have no online "no two paths from A to B" check. Acceptable?

## 17. References

- Sibling specs: [person-service](../person-service-rust-crate/spec.md),
  [event-service](../event-service-rust-crate/spec.md),
  [worker-service](../worker-service-rust-crate/spec.md),
  [thing-service](../thing-service-rust-crate/spec.md).
- AGENTS reference set: [`AGENTS/index.md`](AGENTS/index.md).
- Shared docs: [`agents/share/index.md`](../agents/share/index.md).
- SDD discipline: [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md).
- Project-root web spec: [`../spec.md`](../spec.md).

## 18. Change Control

Material changes to this spec — domain-model fields, match-quality
thresholds, API-surface shape, compliance scope — MUST land in the
same commit as the corresponding code change. The cross-crate
uniformity invariant in the project-root [`spec.md`](../spec.md)
applies to web tier files only; this per-crate spec is local to the
Place Service.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation.
