# Main Patient Index — Specification

Source of truth for the **Main Patient Index (MPI)** crate. This document
articulates what the system *does*, *guarantees*, and *targets*. When code
and this spec disagree, this spec wins — update one or the other with a
deliberate decision recorded here.

For shared infrastructure (web tier, technology stack, observability,
compliance), see the project-root [`spec.md`](../spec.md),
[`AGENTS.md`](../AGENTS.md), and [`agents/share/*`](../agents/share/).
For per-crate reference detail (architecture diagrams, model fields,
matching algorithm constants), see [`AGENTS/`](AGENTS/).

## 1. Purpose

The MPI is a healthcare-specific centralised registry of patient identities
across providers. It exists to:

- Give clinicians one trustworthy view of a patient regardless of how many
  EHRs, MRNs, or registration events the patient has accumulated.
- Detect duplicate patient records in real time on admission and in batch
  on demand, surfacing high-confidence merges and queuing the ambiguous
  ones for human review.
- Enforce HIPAA-style audit trails on every read, write, and merge.
- Expose patient identity over REST, FHIR R5, and (planned) gRPC for
  downstream EHRs, billing, analytics, and population-health systems.

Sibling crates ([person](../main-person-index-rust-crate/),
[worker](../main-worker-index-rust-crate/),
[place](../main-place-index-rust-crate/),
[thing](../main-thing-index-rust-crate/),
[event](../main-event-index-rust-crate/)) share the same architectural
chassis but cover different entity types.

## 2. Domain Model

### Patient

The central record. Field-by-field reference: [`AGENTS/models.md`](AGENTS/models.md).
Material aspects:

- **Identity**: UUID `id` + multiple typed `identifiers` (MRN, SSN, DL,
  NPI, PPN, TAX, Other) + optional `tax_id` shortcut.
- **Names**: primary `name: HumanName` + `additional_names` (aliases,
  former, maiden); each name carries `use_type`, family, given, prefix,
  suffix.
- **Contact**: `telecom: Vec<ContactPoint>` (phone / fax / email / pager /
  url / sms / other), `addresses: Vec<Address>` (home / work / temp / old
  / billing).
- **Identity documents**: passport, birth certificate, national ID,
  driver's licence, voter ID, military ID, residence/work permit; each
  with type + number + country + issuer + dates + verified flag.
- **Emergency contacts**: name, relationship, telecom, address,
  `is_primary` flag.
- **Demographics**: `gender` (Male/Female/Other/Unknown), `birth_date`,
  `marital_status`, `multiple_birth`, `deceased` + `deceased_datetime`,
  `photo`.
- **Organisation**: `managing_organization` reference + per-patient
  `links: Vec<PatientLink>` (ReplacedBy / Replaces / Refer / Seealso).
- **Audit**: `active` (soft-delete flag), `created_at`, `updated_at`.

### Supporting types

`Organization`, `MergeRequest` / `MergeResponse` / `MergeRecord`,
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`, `Consent`
(DataProcessing / DataSharing / Marketing / Research / EmergencyAccess —
Active / Revoked / Expired).

### Invariants

- `family` name must be non-empty.
- `birth_date`, when present, must not be in the future.
- An `Identifier` is keyed by `(identifier_type, system, value)` —
  duplicates are silently deduplicated on update.
- `IdentityDocument.expiry_date`, when present, must be on or after
  `issue_date`.
- Soft-delete is the only delete: `DELETE` flips `active = false` and
  writes an audit row; the row remains in the database.

## 3. Functional Capabilities

### 3.1 Identity management

- Create / read / update / soft-delete patient records.
- Manage multiple identifiers per record (typed, system-qualified).
- Manage identity documents with expiry tracking.
- Manage multiple addresses, telecom contacts, and emergency contacts.
- Publish a `PatientCreated` / `PatientUpdated` / `PatientDeleted` event
  on every CRUD operation. See [`agents/share/auditability.md`](../agents/share/auditability.md).

### 3.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).

| Strategy | Output | Use |
|---|---|---|
| Probabilistic | Weighted sum 0.00–1.00 across name / DOB / gender / address / identifier / tax-ID / document | Fuzzy real-world data |
| Deterministic | Rule-based; short-circuit to 1.0 on tax-ID, identifier, or document exact match | Hard guarantees |

Component weights: Name 0.30, DOB 0.25, Gender 0.10, Address 0.10,
Identifier 0.10, Tax ID 0.10, Document 0.05. Algorithms: Jaro-Winkler,
Levenshtein, Soundex phonetic bonus (+0.05 if codes match and score <
0.95), date proximity, weighted per-field address.

Match-quality classification: ≥ 0.95 Definite · ≥ 0.85 Probable · ≥ 0.50
Possible · < 0.50 Unlikely (thresholds configurable).

### 3.3 Search

Powered by Tantivy across 11 indexed fields (name, identifiers, DOB
year, addresses, …). Supports full-text + fuzzy + phonetic, boolean
syntax, pagination via `offset` + `limit`, and optional sensitive-field
masking on results. The search index is kept in sync with database
writes; bulk re-indexing is supported.

### 3.4 Duplicate detection & merging

- **Real-time** on `POST /api/patients` — runs duplicate-check before
  insert; returns `409 Conflict` with candidate matches when found.
- **Explicit** via `POST /api/patients/check-duplicates` — same
  algorithm, no insert.
- **Batch** via `POST /api/patients/deduplicate` — pairwise scan with
  configurable `threshold`, `max_candidates`, `auto_merge_threshold`.
- **Review queue** — `ReviewQueueItem` per candidate pair with status
  `Pending` / `Confirmed` / `Rejected` / `AutoMerged`.
- **Merge** — transfers identifiers, names, addresses, contacts,
  documents, tax-ID, and emergency contacts to the surviving record;
  appends the duplicate's primary name as a "former" alias on the
  survivor; creates a `Replaces` link from survivor → duplicate;
  soft-deletes the duplicate; records the merge with a JSON snapshot
  of transferred data; emits a `Merged` event.

### 3.5 Validation & normalisation

Required-field enforcement (family + given name), future-date guard on
birth date, tax-ID format check, email regex, phone digit count check,
address completeness (city ∨ postal ∨ country), document number
required + expiry guard, emergency-contact name+relationship required.
Phone is normalised E.164-like; address is standardised (title-case
city, uppercase state/country, expand St./Ave./Rd. abbreviations).
Failed validation returns `422`.

### 3.6 Privacy

- Per-field masking for sensitive values (SSN, tax ID, passport, phone,
  email, address); served at `GET /api/patients/{id}/masked`.
- GDPR Article 15 export at `GET /api/patients/{id}/export` returns the
  full patient record as JSON.
- Consent model with type + status + grant/expiry/revoke dates;
  `has_active_consent()` utility checks consent at use sites.
- See [`agents/share/privacy.md`](../agents/share/privacy.md).

### 3.7 Audit

Every CRUD / merge / link operation writes to the `audit_log` table
with old + new values as JSON, user ID, IP, user agent, and timestamp.
Audit queries: per-patient, recent system-wide, per-user. See
[`agents/share/auditability.md`](../agents/share/auditability.md).

## 4. Quality Attributes

| Attribute | Target |
|---|---|
| Scale | Millions of patients, thousands of clinics |
| Patient create latency (incl. dup-check + index + audit) | ≤ 50 ms p50 |
| Patient read | ≤ 5 ms p50 |
| Search query | ≤ 100 ms p50 |
| Match call | ≤ 500 ms p99 |
| Throughput | ≥ 1 000 req/sec single instance |
| Availability | High availability with disaster recovery (HADR); stateless app tier; PostgreSQL replication |
| Fault tolerance | Graceful shutdown; connection pooling; container health checks; non-root containers |
| Observability | OTLP traces / metrics / logs; per-request `traceparent`; structured JSON logs in prod |

## 5. Technology Stack

The crate sits on the project-wide stack ([`agents/share/stack-for-rust-loco.md`](../agents/share/stack-for-rust-loco.md)).
Crate-specific pinning:

- **Runtime**: Rust 1.93+ 2024 edition · Tokio 1.x
- **Web**: Axum 0.7 · Loco.rs 0.14 · Tera 1.20 · HTMX 2.0 · Alpine.js 3.14 · Lily HTML Headless (NHS UK theme)
- **Data**: PostgreSQL 18+ · SeaORM 1.1
- **Search**: Tantivy 0.22
- **API docs**: utoipa 5.x + Swagger UI
- **gRPC**: Tonic 0.12 (stub today)
- **Event streaming**: Fluvio 0.23 (in-memory publisher today)
- **Observability**: tracing + opentelemetry + opentelemetry-otlp
- **String matching**: strsim (Jaro-Winkler, Levenshtein)
- **Validation**: validator
- **Testing**: assertables, mockall, tempfile, tokio-test, criterion

## 6. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/patients/*` + `/api/audit/*` + `/api/health` |
| FHIR R5 (Axum) | `Patient` resource CRUD + search under `/fhir/Patient` |
| gRPC (Tonic) | Stubbed; not yet implemented |
| Web UI (Loco / Tera / HTMX / Alpine / Lily) | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

All REST endpoints return `{ "success": bool, "data": …, "error": … }`.
HTTP status codes follow REST conventions including `409` for duplicate
detection on create and `422` for validation failure.

## 7. Persistence

PostgreSQL 18+ via SeaORM. Schema overview: [`agents/share/postgresql.md`](../agents/share/postgresql.md).
Tables (12+):

`patients`, `patient_names`, `patient_identifiers`, `patient_addresses`,
`patient_contacts`, `patient_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `patient_match_scores`, `audit_log`.

Required PostgreSQL extensions: `pg_stat_statements`, `uuid-ossp`,
`pgcrypto`, `pg_trgm`, `citext`, `unaccent`. Optional: `pg_vector`,
`postgis`.

Connection pooling with configurable min/max; soft-delete is application
level (`active` flag); audit triggers retain history.

## 8. Testing & Quality

Strategy: [`AGENTS/testing.md`](AGENTS/testing.md).

- **Unit tests**: embedded in source under `#[cfg(test)]`; no external
  dependencies. Coverage targets: matching algorithms, phonetic,
  scoring, validation, privacy, model construction.
- **Integration tests**: under `tests/`; full HTTP request/response
  cycles against real PostgreSQL + Tantivy. Run via
  `docker-compose -f docker-compose.test.yml up` or
  `cargo test --test api_integration_test` against a live DB.
- **Benchmarks**: Criterion suites for matching, search, validation.
- **CI**: `test.yml` (unit + integration), `quality.yml` (fmt + clippy),
  `security.yml` (scanning).

## 9. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, access tracking, encryption-at-rest delegated to PostgreSQL, soft delete |
| GDPR Art. 15 | `GET /api/patients/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Patient resource (bidirectional conversion) |
| ISO/IEC 27001 | Operational controls (deployment-side) |

Healthcare-specific compliance frameworks tracked in
[`agents/share/compliance-for-healthcare.md`](../agents/share/compliance-for-healthcare.md);
technology compliance in
[`agents/share/compliance-for-technology.md`](../agents/share/compliance-for-technology.md).

## 10. Implementation Status

### Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture, 40+ dependencies |
| Database schema | 12+ tables, SeaORM entities, indexes, audit triggers |
| Matching | Probabilistic + deterministic; Jaro-Winkler + Levenshtein + Soundex; configurable weights |
| Search | Tantivy 11-field index; fuzzy + phonetic + bulk + blocking |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| FHIR R5 | Patient bidirectional conversion + search parameters + OperationOutcome |
| Repository | SeaORM CRUD with transactions, soft delete, paginated active retrieval |
| Event streaming | InMemoryEventPublisher (Created/Updated/Deleted/Merged/Linked/Unlinked) |
| Audit log | AuditLogRepository with old/new JSON snapshots + user context + query endpoints |
| Duplicate detection | Real-time (409 on create) + explicit endpoint + batch scan with review queue |
| Merging | Data transfer, alias creation, link, soft-delete, JSON snapshot, event emission |
| Validation | Required fields, format checks, phone normalisation, address standardisation, 422 on failure |
| Privacy | Field masking, GDPR export, consent model + active-consent utility |
| Docker | Multi-stage Dockerfile (~85 % image-size reduction), dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |
| Documentation | README, CLAUDE.md, full AGENTS/* set, architecture diagrams, deploy guide |

### Open gaps

| Gap | Where |
|---|---|
| FHIR capability statement | not yet emitted |
| FHIR bundle (full) | partial only |
| FHIR Organization resource | not yet mapped |
| Fluvio production publisher | in-memory stub only |
| Event consumers | stub |
| gRPC API | scaffolded, not implemented |
| Dedup / merge / privacy integration tests | not yet written |

## 11. Roadmap

Forward-looking phases. Order is suggestive, not contractual.

### Authentication & authorisation

- JWT-based authentication middleware on `/api/*`.
- Role-based access control (RBAC) for clinician / admin / service roles.
- Rate limiting and request throttling.
- User management endpoints.
- Security headers.

### Observability & monitoring

- Prometheus metrics exporter alongside OTLP.
- Complete the OTLP trace exporter wiring.
- Custom MPI metrics (`patient_created`, `match_score_histogram`, etc.).
- Grafana dashboard templates + alerting rules.

### Performance optimisation

- Database query caching (Redis or in-memory).
- Batch-load N+1 query offenders in the repository.
- Load test at realistic patient volumes.
- Profile and optimise matching hot paths.

### Infrastructure as code

- OpenTofu modules for PostgreSQL provisioning and app deployment.
- Multi-cloud configuration (GCP, AWS, Azure).
- Secrets management integration.
- Backup and disaster-recovery automation.

### Kubernetes

- Helm chart.
- Horizontal pod autoscaling.
- Persistent volume claims for the search index.
- Ingress controllers + Kubernetes health probes.

### Production readiness

- Security audit and penetration test.
- HIPAA and GDPR compliance validation.
- Disaster-recovery runbook and drills.
- Backup and restore procedures.
- Incident-response procedures.
- CI/CD pipeline.

### Feature enhancements

- Complete gRPC server implementation.
- Complete FHIR R5 (capability statement, bundles, Organization).
- Fluvio production event publisher + consumers.
- ML-based match scoring with A/B test framework.
- Patient photo storage and retrieval.
- Consent enforcement in the query layer.

## 12. Change control

Material changes to this spec — domain-model fields, match-quality
thresholds, API-surface shape, compliance scope — should land in the
same commit as the code change. The cross-crate uniformity invariant
documented in the project-root [`spec.md`](../spec.md) applies to web
tier files only; this per-crate spec is local to MPI.
