# Main Event Service — Specification

Source of truth for the **Main Event Service** crate. This document
articulates what the system *does*, *guarantees*, and *targets*. When
code and this spec disagree, this spec wins — update one or the other
with a deliberate decision recorded here.

For shared infrastructure (web tier, technology stack, observability,
compliance), see the project-root [`spec.md`](../spec.md),
[`AGENTS.md`](../AGENTS.md), and [`agents/share/*`](../agents/share/).
For per-crate reference detail, see [`AGENTS/`](AGENTS/).

## 1. Purpose

The Main Event Service is a centralised registry of **time-bounded
events**: appointments, encounters, shifts, sessions, deliveries,
incidents, scheduled tasks — anything that can be canonicalised as
"a thing happening, between a start time and an end time, involving
parties and a place." It exists to:

- Give callers one trustworthy view of each event regardless of how
  many scheduling, EHR, CRM, calendar, or operational systems hold a
  shard of that event.
- Match event records probabilistically and deterministically against
  arbitrary input (party + approximate time, identifier + organisation,
  partial title + venue) and return ranked candidates with per-component
  score breakdowns.
- Detect duplicate events (real-time on create, batch on demand) — for
  example the same appointment created by both the patient portal and
  the front-desk EHR.
- Provide a stable cross-system identifier surface so downstream
  analytics, billing, and notifications can refer to one event ID per
  real-world occurrence.
- Emit audit logs and event-streaming records for every CRUD / merge /
  link operation on event records (note: "event streaming" here is the
  Fluvio pipe for *index-level* changes, not the modelled domain
  events themselves).

Sibling crates: [person](../main-person-service-rust-crate/),
[patient](../main-patient-index-rust-crate/),
[worker](../main-worker-service-rust-crate/),
[place](../main-place-service-rust-crate/),
[thing](../main-thing-service-rust-crate/).

## 2. Domain Model

### Event

Field-by-field reference: [`AGENTS/models.md`](AGENTS/models.md).
Material aspects:

- **Identity**: UUID `id` + multiple typed `identifiers` (booking
  number, encounter ID, ticket number, external system IDs, TAX where
  the event is a billable encounter).
- **Title + description**: human-readable label + optional long-form
  description.
- **Time window**: `start_time` (required), `end_time` (optional;
  unbounded = open-ended), `time_zone`, `all_day` flag.
- **Status / lifecycle**: planned / in-progress / completed / cancelled
  / no-show (configurable; mapped to FHIR Encounter status where
  applicable).
- **Parties**: links to subject (person / patient) and performer
  (worker / organisation). Optional `links` to other indices.
- **Location**: link to a `place_id` or inline `Address`.
- **Categorisation**: `event_type` (appointment / shift / encounter /
  session / incident / delivery / other), `category` tags.
- **Audit**: `active` (soft-delete flag), `created_at`, `updated_at`.

### Supporting types

`Organization`, `MergeRequest` / `MergeResponse` / `MergeRecord`,
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`,
`Consent` (for events involving consented data sharing).

### Invariants

- `start_time` must be present and non-null.
- `end_time`, when present, must be ≥ `start_time`.
- An `Identifier` is keyed by `(identifier_type, system, value)`.
- Cancelled events are not deleted — their status changes; `active`
  remains `true` until soft-deleted.
- Soft-delete is the only delete.

## 3. Functional Capabilities

### 3.1 Identity management

- Create / read / update / soft-delete event records.
- Multiple identifiers per event (typed, system-qualified).
- Status transitions tracked through the audit log.
- Event publish on every CRUD.

### 3.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).
Component weights are tuned for time-bounded entities:

| Component | Weight | Algorithm |
|---|---|---|
| Title | 0.20 | Jaro-Winkler + Levenshtein |
| Time window | 0.30 | Interval overlap + start-time proximity |
| Subject (person/patient) ID | 0.20 | Exact match |
| Performer / org ID | 0.10 | Exact match |
| Location / place | 0.10 | Place ID exact or address fuzzy |
| Identifier | 0.10 | Type + system + value exact |

Deterministic short-circuit: exact `(identifier_type, system, value)`
match → 1.0.

Match quality: ≥ 0.95 Definite · ≥ 0.85 Probable · ≥ 0.50 Possible · <
0.50 Unlikely (configurable).

### 3.3 Search

Tantivy across indexed fields (title, identifiers, subject, performer,
place, start-time year/month). Full-text + fuzzy + boolean. Pagination
via `offset` + `limit`. Optional masking for events with sensitive
subjects.

### 3.4 Duplicate detection & merging

- Real-time `409 Conflict` on `POST /api/events` when the time window
  overlaps an existing event with the same subject + performer.
- Explicit `POST /api/events/check-duplicates`.
- Batch `POST /api/events/deduplicate`.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge picks the surviving record; transfers identifiers,
  performer/subject links, and title aliases; adds a `Replaces` link;
  soft-deletes the duplicate; records a JSON snapshot of transferred
  data; emits a `Merged` event.

### 3.5 Validation & normalisation

`start_time` required; `end_time ≥ start_time` when both present;
`time_zone` must be a valid IANA name when supplied. Identifier
formats validated per type. Failed validation → `422`.

### 3.6 Privacy

Per-field masking (subject IDs, free-text descriptions). GDPR Article
15 export at `GET /api/events/{id}/export`. Consent enforcement is
applied at the query layer when the subject of the event has revoked
or restricted consent.

### 3.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp.

## 4. Quality Attributes

| Attribute | Target |
|---|---|
| Scale | Millions of events |
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
- **Time / interval**: chrono + chrono-tz
- **Validation**: validator
- **Testing**: assertables, mockall, tempfile, tokio-test, criterion

## 6. API Surface

Complete reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/events/*` + `/api/audit/*` + `/api/health` |
| FHIR R5 (Axum) | Maps to `Encounter` / `Appointment` resources where applicable |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` |

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

## 7. Persistence

PostgreSQL 18+ via SeaORM. Tables (12+):

`events`, `event_identifiers`, `event_subjects`, `event_performers`,
`event_locations`, `event_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `event_match_scores`, `audit_log`.

Required PostgreSQL extensions: `pg_stat_statements`, `uuid-ossp`,
`pgcrypto`, `pg_trgm`, `citext`, `unaccent`. Time-window queries
benefit from a `btree_gist` exclusion-constraint extension where the
deployment wants to enforce "no overlapping events per resource".

## 8. Testing & Quality

Strategy: [`AGENTS/testing.md`](AGENTS/testing.md).

- Unit tests under `#[cfg(test)]`: matching, scoring, validation,
  privacy, models, time-interval algebra.
- Integration tests under `tests/`: full HTTP cycles against real
  PostgreSQL + Tantivy.
- Criterion benchmarks: matching, search, validation.
- CI: `test.yml`, `quality.yml`, `security.yml`.

## 9. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, access tracking, encryption-at-rest (DB), soft delete |
| GDPR Art. 15 | `GET /api/events/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Encounter / Appointment bidirectional conversion |
| ISO/IEC 27001 | Operational controls (deployment-side) |

## 10. Implementation Status

### Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Database schema | Tables + SeaORM entities + indexes + audit triggers |
| Matching | Probabilistic + deterministic; configurable weights |
| Search | Tantivy index; fuzzy + bulk |
| REST API | Core endpoints + OpenAPI/Swagger + CORS + structured errors |
| FHIR R5 | Initial Encounter/Appointment conversion |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher (index-level events) |
| Audit log | AuditLogRepository with old/new JSON |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, time-window guards, 422 |
| Privacy | Field masking, GDPR export, consent model |
| Docker | Multi-stage Dockerfile, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |

### Open gaps

| Gap | Where |
|---|---|
| FHIR capability statement | not yet emitted |
| FHIR bundle (full) | partial only |
| Recurrence / RRULE support | not yet a domain capability |
| Time-zone-aware fuzzy matching | uses naive UTC offsets today |
| Fluvio production publisher | in-memory stub only |
| Event consumers | stub |
| gRPC API | scaffolded, not implemented |
| Dedup / merge / privacy integration tests | not yet written |

## 11. Roadmap

### Authentication & authorisation

JWT middleware, RBAC for scheduler / admin / read-only / service
roles, rate limiting, security headers.

### Observability & monitoring

Prometheus metrics alongside OTLP, complete OTLP trace exporter, custom
metrics (`event_created`, `event_duration_seconds`, `match_score`),
Grafana dashboards + alerting.

### Performance optimisation

Time-range query caching, btree_gist exclusion constraints for
no-overlap policies, load test at realistic event volumes, profile and
optimise matching hot paths.

### Infrastructure as code

OpenTofu modules, multi-cloud (GCP, AWS, Azure), secrets management,
backup and DR automation.

### Kubernetes

Helm chart, HPA, PVCs for the search index, ingress controllers,
Kubernetes health probes.

### Production readiness

Security audit + pen test, HIPAA + GDPR compliance validation, DR
runbook + drills, backup and restore procedures, incident-response
procedures, CI/CD pipeline.

### Feature enhancements

Complete gRPC, complete FHIR (capability statement, bundles,
Encounter/Appointment full coverage), Fluvio production publisher +
consumers, ML-based match scoring, **iCalendar import/export**,
**RFC 5545 RRULE recurrence support**, **time-zone-aware fuzzy
matching**, consent enforcement in the query layer.

## 12. Change control

Material changes to this spec should land in the same commit as the
code change. The cross-crate uniformity invariant documented in the
project-root [`spec.md`](../spec.md) applies to web tier files only;
this per-crate spec is local to the Main Event Service.
