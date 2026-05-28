# Main Thing Index — Specification

Source of truth for the **Main Thing Index** crate. This document
articulates what the system *does*, *guarantees*, and *targets*. When
code and this spec disagree, this spec wins — update one or the other
with a deliberate decision recorded here.

For shared infrastructure (web tier, technology stack, observability,
compliance), see the project-root [`spec.md`](../spec.md),
[`AGENTS.md`](../AGENTS.md), and [`agents/share/*`](../agents/share/).
For per-crate reference detail, see [`AGENTS/`](AGENTS/).

## 1. Purpose

The Main Thing Index is a **generic asset / object registry** —
the lowest-common-denominator entity in the Main X Index family.
It exists to:

- Catalogue arbitrary discrete objects (devices, vehicles, equipment,
  inventory items, products, instances of physical or digital assets)
  with a stable identity, identifiers, hierarchy, and audit trail —
  without forcing a healthcare, workforce, place, or event ontology
  onto the record.
- Match thing records probabilistically and deterministically by name,
  identifier (SKU, serial number, asset tag, RFID, barcode), category,
  manufacturer, and location.
- Detect duplicate things (real-time on create, batch on demand) with
  a review queue and auto-merge for high-confidence cases.
- Provide a stable cross-system identifier so downstream procurement,
  inventory, maintenance, and analytics systems can refer to one thing
  ID per real-world item.
- Emit audit logs and event-streaming records for every CRUD / merge /
  link operation.

This crate is the right home for anything that doesn't fit one of the
more opinionated sibling crates:
[person](../main-person-index-rust-crate/),
[patient](../main-patient-index-rust-crate/),
[worker](../main-worker-index-rust-crate/),
[place](../main-place-index-rust-crate/),
[event](../main-event-index-rust-crate/).

## 2. Domain Model

### Thing

Field-by-field reference: [`AGENTS/models.md`](AGENTS/models.md).
Material aspects:

- **Identity**: UUID `id` + multiple typed `identifiers` (SKU, GTIN,
  serial number, asset tag, RFID, barcode, MAC address, IMEI, custom).
- **Names**: `name` (primary), `alternate_name` (aliases, model names,
  product variants), `description`.
- **Classification**: `thing_type` (Device, Vehicle, Equipment,
  Inventory, Product, DigitalAsset, Container, Component, Other),
  `category` tags, `manufacturer`, `model`, `version`.
- **Hierarchy**: `parent_thing` (parent in assembly / container
  hierarchy) + `child_things` (sub-components).
- **Location**: optional link to a `place_id` (where the thing
  currently sits) and `assigned_to` (worker / person / organisation).
- **Lifecycle**: `acquisition_date`, `decommission_date`,
  `warranty_expiry`, `last_serviced_at`.
- **Operational state**: `status` (active / in-maintenance /
  in-transit / decommissioned / lost / disposed).
- **External cross-refs**: `same_as` URLs.
- **Audit**: `active` (soft-delete flag), `created_at`, `updated_at`.

### Supporting types

`ThingType`, `ThingIdentifier`, `Organization` (for manufacturer /
owner / custodian links), `MergeRequest` / `Response` / `Record`,
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`,
`Consent` (for things subject to data-protection regimes — for
example a personal device).

### Invariants

- `name` must be non-empty.
- An `Identifier` is keyed by `(identifier_type, system, value)`;
  duplicates within a single record are silently deduplicated.
- `decommission_date`, when present, must be on or after
  `acquisition_date`.
- `warranty_expiry`, when present, must be on or after
  `acquisition_date`.
- A thing can have at most one `parent_thing`; cycles are rejected.
- A `status = "disposed"` thing is treated as effectively soft-deleted
  for matching purposes (excluded from live duplicate detection).
- Soft-delete is the only delete.

## 3. Functional Capabilities

### 3.1 Identity management

- Create / read / update / soft-delete thing records.
- Multiple identifiers per thing (SKU, serial, asset tag, barcode,
  RFID, …).
- Assembly hierarchy (`parent_thing` / `child_things`).
- Location and assignment tracking (link to place / worker / person).
- Lifecycle dates and operational status transitions.
- Event publish on every CRUD.

### 3.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).
Component weights tuned for things:

| Component | Weight | Algorithm |
|---|---|---|
| Name | 0.30 | Jaro-Winkler + Levenshtein |
| Identifier | 0.30 | Exact `(type, system, value)` |
| Manufacturer + model | 0.15 | Jaro-Winkler on combined string |
| Type / category | 0.10 | Exact match |
| Location | 0.10 | Place-ID exact or address fuzzy |
| Acquisition date | 0.05 | Date proximity |

Deterministic short-circuits: exact serial number, GTIN, or asset
tag match → 1.0.

Match quality: Certain / Probable / Possible / Unlikely (configurable
thresholds).

### 3.3 Search

Tantivy across indexed fields (name, alternate_name, identifiers,
manufacturer, model, type, category, location). Full-text + fuzzy +
boolean. Pagination via `offset` + `limit`. Optional masking for
things subject to consent constraints.

### 3.4 Duplicate detection & merging

- Real-time `409 Conflict` on `POST /api/things` when an existing
  thing matches on identifier or name + manufacturer + model.
- Explicit `POST /api/things/check-duplicates`.
- Batch `POST /api/things/deduplicate`.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge transfers identifiers, alternate names, hierarchy links,
  location, assignment, lifecycle dates, `same_as` URLs; appends
  duplicate's name as `alternate_name` on the survivor; adds
  `Replaces` link; soft-deletes duplicate; records JSON snapshot;
  emits `Merged` event.

### 3.5 Validation & normalisation

Required `name`, identifier formats per type (GTIN check digit,
MAC-address format, IMEI Luhn check, …), date ordering
(`decommission_date ≥ acquisition_date`,
`warranty_expiry ≥ acquisition_date`), hierarchy acyclicity, URL
protocol on `same_as`. Failed validation → `422`.

### 3.6 Privacy

Per-field masking for sensitive identifiers (MAC, IMEI, asset
tag where ownership is sensitive). GDPR Article 15 export at
`GET /api/things/{id}/export`. Consent model where the thing is
attached to a person (for example a personal medical device or
phone).

### 3.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp.

## 4. Quality Attributes

| Attribute | Target |
|---|---|
| Scale | Millions of things, thousands of data sources |
| Create latency | ≤ 50 ms p50 |
| Read | ≤ 5 ms p50 |
| Search | ≤ 100 ms p50 |
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
| REST (Axum) | 15 endpoints under `/api/things/*` + `/api/audit/*` + `/api/health` |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` |

Note: this crate does **not** expose a FHIR R5 surface — things are
not a FHIR-resource concern. (A medical-device-flavoured subset could
later map to FHIR `Device` if required.)

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

## 7. Persistence

PostgreSQL 18+ via SeaORM. Tables:

`things`, `thing_identifiers`, `thing_hierarchy`, `thing_locations`,
`thing_assignments`, `thing_same_as`, `thing_links`,
`organizations`, `organization_addresses`, `organization_contacts`,
`organization_identifiers`, `thing_match_scores`, `audit_log`.

Required PostgreSQL extensions: `pg_stat_statements`, `uuid-ossp`,
`pgcrypto`, `pg_trgm`, `citext`, `unaccent`.

## 8. Testing & Quality

Strategy: [`AGENTS/testing.md`](AGENTS/testing.md).

Current coverage (Phase 14–15):

- **Unit tests**: ~100 — models, matching, validation, privacy.
- **Integration tests**: full HTTP cycles against real PostgreSQL +
  Tantivy.
- **Criterion benchmarks**: matching, search, validation.

CI: `test.yml`, `quality.yml` (fmt + clippy), `security.yml`.

## 9. Compliance

| Standard | Mechanism |
|---|---|
| GDPR Art. 15 | `GET /api/things/{id}/export` (for personal things) |
| GDPR Art. 17 | Soft delete + consent revocation |
| ISO/IEC 27001 | Operational controls (deployment-side) |

Technology compliance: [`agents/share/compliance-for-technology.md`](../agents/share/compliance-for-technology.md).

## 10. Implementation Status

### Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Database schema | Tables, SeaORM entities, indexes, audit triggers |
| Domain model | Generic thing with identifiers, hierarchy, location, assignment, lifecycle |
| Matching | Probabilistic (name + manufacturer + model + type + location + date) + deterministic (serial / GTIN / asset tag) |
| Search | Tantivy index |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher |
| Audit log | AuditLogRepository with old/new JSON |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alternate-name + link + soft-delete + snapshot + event |
| Validation | Identifier formats, date ordering, hierarchy acyclicity, 422 |
| Privacy | Sensitive-identifier masking, GDPR export, consent model |
| Docker | Multi-stage Dockerfile, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |
| Documentation | README, CLAUDE.md, AGENTS/* set, architecture, deploy guide |

### Open gaps

| Gap | Where |
|---|---|
| Fluvio production publisher | in-memory stub only |
| Event consumers | stub |
| gRPC API | scaffolded, not implemented |
| Lifecycle-event timeline | per-thing service / maintenance log not yet a first-class capability |
| Inventory-level aggregates | "how many of SKU X exist where" requires app-side roll-up today |

## 11. Roadmap

### Authentication & authorisation

JWT middleware, RBAC for editor / inventory-clerk / auditor /
read-only / service roles, rate limiting, security headers.

### Observability & monitoring

Prometheus metrics alongside OTLP, complete OTLP trace exporter,
custom metrics (`thing_created`, `things_in_maintenance`,
`identifier_collisions`), Grafana dashboards + alerting.

### Performance optimisation

Identifier-collision indexes, recursive CTEs for hierarchy queries,
N+1 batch fixes, load test at realistic thing volumes, profile and
optimise matching hot paths.

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

Complete gRPC, Fluvio production publisher + consumers, **FHIR Device
mapping** (for the medical-device subset), **barcode / RFID scan
endpoint** (POST a scanned identifier, get the thing back), **service
/ maintenance timeline** as a first-class capability, **inventory
aggregates** (SKU → count by location), **image / photo storage**
for visual identification.

## 12. Change control

Material changes to this spec — domain-model fields, match-quality
thresholds, API-surface shape, compliance scope — should land in the
same commit as the code change. The cross-crate uniformity invariant
documented in the project-root [`spec.md`](../spec.md) applies to web
tier files only; this per-crate spec is local to the Main Thing Index.
