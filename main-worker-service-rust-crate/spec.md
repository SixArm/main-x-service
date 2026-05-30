# Main Worker Service — Specification

Source of truth for the **Main Worker Service** crate. This document
articulates what the system *does*, *guarantees*, and *targets*. When
code and this spec disagree, this spec wins — update one or the other
with a deliberate decision recorded here.

For shared infrastructure (web tier, technology stack, observability,
compliance), see the project-root [`spec.md`](../spec.md),
[`AGENTS.md`](../AGENTS.md), and [`agents/share/*`](../agents/share/).
For per-crate reference detail, see [`AGENTS/`](AGENTS/).

## 1. Purpose

The Main Worker Service is a centralised registry of **workforce and
professional identities**: clinicians, contractors, drivers, hospital
staff, field engineers, anyone whose role + credentials matter to the
caller. It exists to:

- Give an organisation one trustworthy record per worker regardless
  of how many HR, scheduling, credentialing, and payroll systems hold
  shards of that identity.
- Match worker records probabilistically and deterministically against
  arbitrary input (typed name, partial NPI, credential number, …) and
  return ranked candidates with per-component score breakdowns.
- Detect duplicate worker records on real-time create and in batch on
  demand, with a review queue and auto-merge for high-confidence cases.
- Carry credential / licence / professional-identifier fields
  (NPI, DEA, board licence, employee number) alongside the same
  healthcare-aware fields the patient and person indices use.
- Emit HIPAA-grade audit logs and event-streaming records for every
  CRUD / merge / link operation, in support of compliance for
  credentialed workforce data.

Sibling crates: [person](../main-person-service-rust-crate/),
[patient](../main-patient-index-rust-crate/),
[place](../main-place-service-rust-crate/),
[thing](../main-thing-service-rust-crate/),
[event](../main-event-service-rust-crate/).

## 2. Domain Model

### Worker

Field-by-field reference: [`AGENTS/models.md`](AGENTS/models.md).
Material aspects:

- **Identity**: UUID `id` + multiple typed `identifiers` (NPI, DEA,
  professional licence, MRN-style employee number, SSN, DL, TAX, Other)
  + optional `tax_id` shortcut.
- **Names**: primary `name: HumanName` + `additional_names` (former
  names, name at credential issuance, married/maiden forms).
- **Contact**: `telecom: Vec<ContactPoint>`, `addresses: Vec<Address>`.
- **Identity documents**: passport, driver's licence, professional
  credentials, certificates with type + number + issuing authority +
  issue/expiry dates + verified flag.
- **Emergency contacts**: name, relationship, telecom, address.
- **Demographics**: `gender`, `birth_date`, `marital_status`,
  `multiple_birth`, `deceased`, `photo`.
- **Organisation**: `managing_organization` reference + per-worker
  `links: Vec<WorkerLink>` (ReplacedBy / Replaces / Refer / Seealso).
- **Audit**: `active` (soft-delete flag), `created_at`, `updated_at`.

### Supporting types

`Organization`, `MergeRequest` / `MergeResponse` / `MergeRecord`,
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`, `Consent`.

### Invariants

- `family` name must be non-empty.
- `birth_date`, when present, must not be in the future.
- An `Identifier` is keyed by `(identifier_type, system, value)`.
- `IdentityDocument.expiry_date`, when present, must be on or after
  `issue_date` — credentials with no expiry are non-expiring; an
  expiry in the past flags an expired credential but does not refuse
  the record.
- Soft-delete is the only delete.

## 3. Functional Capabilities

### 3.1 Identity management

- Create / read / update / soft-delete worker records.
- Multiple professional identifiers per worker.
- Credential documents with expiry tracking.
- Multiple addresses, telecom contacts, emergency contacts.
- Event publish on every CRUD.

### 3.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).

| Strategy | Output | Use |
|---|---|---|
| Probabilistic | Weighted sum 0.00–1.00 across name / DOB / gender / address / identifier / tax-ID / document | Fuzzy input |
| Deterministic | Rule-based; short-circuits on identifier (NPI, DEA, employee #), tax-ID, or document exact match | Hard guarantees |

Component weights: Name 0.30, DOB 0.25, Gender 0.10, Address 0.10,
Identifier 0.10, Tax ID 0.10, Document 0.05. Algorithms: Jaro-Winkler,
Levenshtein, Soundex phonetic bonus, date proximity, weighted per-field
address.

Match quality: ≥ 0.95 Definite · ≥ 0.85 Probable · ≥ 0.50 Possible · <
0.50 Unlikely (configurable).

### 3.3 Search

Tantivy across 11 indexed fields (name, identifiers including NPI/DEA,
DOB year, addresses). Full-text + fuzzy + phonetic, boolean syntax,
pagination, optional sensitive-field masking. Index stays synchronised
with database writes.

### 3.4 Duplicate detection & merging

- Real-time `409 Conflict` on `POST /api/workers`.
- Explicit `POST /api/workers/check-duplicates`.
- Batch `POST /api/workers/deduplicate` with configurable thresholds.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge transfers identifiers (credentials!), names, addresses,
  contacts, documents, tax-ID, emergency contacts; appends duplicate's
  primary name as "former" alias; adds `Replaces` link; soft-deletes
  duplicate; records JSON snapshot; emits `Merged` event.

### 3.5 Validation & normalisation

Required-field enforcement (family + given name), future-date guard on
birth date, tax-ID format, email regex, phone digit count, address
completeness, document number required + expiry guard, emergency-contact
name+relationship required. Phone normalised E.164-like; addresses
standardised. Failed validation → `422`.

### 3.6 Privacy

Per-field masking, GDPR Article 15 export at
`GET /api/workers/{id}/export`, masked view at
`GET /api/workers/{id}/masked`, consent model with type + status +
dates, `has_active_consent()` utility. Sensitive fields specific to
workforce data (SSN, tax ID, DEA, home address) are masked by default
in the masked view.

### 3.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp. Queries: per-worker, recent
system-wide, per-user.

## 4. Quality Attributes

| Attribute | Target |
|---|---|
| Scale | Millions of workers, thousands of organisations |
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
- **Testing**: assertables, mockall, tempfile, tokio-test, criterion

## 6. API Surface

Complete reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/workers/*` + `/api/audit/*` + `/api/health` |
| FHIR R5 (Axum) | `Practitioner` resource CRUD + search under `/fhir/Practitioner` |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

Responses use the standard envelope `{ "success", "data", "error" }`.
`409` on duplicate-detected create; `422` on validation failure.

## 7. Persistence

PostgreSQL 18+ via SeaORM. Tables (12+):

`workers`, `worker_names`, `worker_identifiers`, `worker_addresses`,
`worker_contacts`, `worker_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `worker_match_scores`, `audit_log`.

Required PostgreSQL extensions: `pg_stat_statements`, `uuid-ossp`,
`pgcrypto`, `pg_trgm`, `citext`, `unaccent`.

## 8. Testing & Quality

Strategy: [`AGENTS/testing.md`](AGENTS/testing.md).

- Unit tests under `#[cfg(test)]`: matching, phonetic, scoring,
  validation, privacy, models.
- Integration tests under `tests/`: full HTTP cycles against real
  PostgreSQL + Tantivy.
- Criterion benchmarks: matching, search, validation.
- CI: `test.yml`, `quality.yml`, `security.yml`.

## 9. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, access tracking, encryption-at-rest (DB), soft delete |
| GDPR Art. 15 | `GET /api/workers/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Practitioner resource (bidirectional conversion) |
| ISO/IEC 27001 | Operational controls (deployment-side) |

Healthcare-specific: [`agents/share/compliance-for-healthcare.md`](../agents/share/compliance-for-healthcare.md).
Technology compliance: [`agents/share/compliance-for-technology.md`](../agents/share/compliance-for-technology.md).

## 10. Implementation Status

### Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture, 40+ dependencies |
| Database schema | 12+ tables, SeaORM entities, indexes, audit triggers |
| Matching | Probabilistic + deterministic; Jaro-Winkler + Levenshtein + Soundex; configurable weights |
| Search | Tantivy 11-field index; fuzzy + phonetic + bulk + blocking |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| FHIR R5 | Practitioner bidirectional conversion + search parameters |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher (Created/Updated/Deleted/Merged/Linked/Unlinked) |
| Audit log | AuditLogRepository with old/new JSON + user context |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, phone normalisation, address standardisation, 422 |
| Privacy | Field masking, GDPR export, consent model |
| Docker | Multi-stage Dockerfile, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |

### Open gaps

| Gap | Where |
|---|---|
| FHIR capability statement | not yet emitted |
| FHIR bundle (full) | partial only |
| FHIR Organization resource | not yet mapped |
| Fluvio production publisher | in-memory stub only |
| Event consumers | stub |
| gRPC API | scaffolded, not implemented |
| Credential-expiry alerts | not yet a domain capability |
| Dedup / merge / privacy integration tests | not yet written |

## 11. Roadmap

### Authentication & authorisation

JWT middleware, RBAC for HR-admin / credentialing-officer / service
roles, rate limiting, user-management endpoints, security headers.

### Observability & monitoring

Prometheus metrics alongside OTLP, complete OTLP trace exporter, custom
metrics (`worker_created`, `credential_expiry_within_30d`, etc.),
Grafana dashboards + alerting.

### Performance optimisation

Database query caching, N+1 batch fixes, load test at realistic
workforce volumes, profile and optimise matching hot paths.

### Infrastructure as code

OpenTofu modules (PostgreSQL + app deploy), multi-cloud (GCP, AWS,
Azure), secrets management, backup and DR automation.

### Kubernetes

Helm chart, HPA, PVCs for the search index, ingress controllers,
Kubernetes health probes.

### Production readiness

Security audit + pen test, HIPAA + GDPR compliance validation, DR
runbook + drills, backup and restore procedures, incident-response
procedures, CI/CD pipeline.

### Feature enhancements

Complete gRPC server, complete FHIR R5 (capability statement, bundles,
Organization), Fluvio production publisher + consumers, ML-based match
scoring with A/B test framework, worker photo storage, consent
enforcement in the query layer, **credential-expiry-warning workflow**
(domain-specific to workforce data), **role + assignment history**
captured per worker as a timeline.

## 12. Change control

Material changes to this spec — domain-model fields, match-quality
thresholds, API-surface shape, compliance scope — should land in the
same commit as the code change. The cross-crate uniformity invariant
documented in the project-root [`spec.md`](../spec.md) applies to web
tier files only; this per-crate spec is local to the Main Worker Service.
