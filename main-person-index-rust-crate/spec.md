# Main Person Index — Specification

Source of truth for the **Main Person Index** crate. This document
articulates what the system *does*, *guarantees*, and *targets*. When
code and this spec disagree, this spec wins — update one or the other
with a deliberate decision recorded here.

For shared infrastructure (web tier, technology stack, observability,
compliance), see the project-root [`spec.md`](../spec.md),
[`AGENTS.md`](../AGENTS.md), and [`agents/share/*`](../agents/share/).
For per-crate reference detail (architecture diagrams, model fields,
matching algorithm constants), see [`AGENTS/`](AGENTS/).

## 1. Purpose

The Main Person Index is a general-purpose centralised registry of
person identities. It sits alongside the more domain-specific
[Patient](../main-patient-index-rust-crate/) and
[Worker](../main-worker-index-rust-crate/) indices and exists to:

- Provide one canonical person identity that other crates and external
  systems can refer to without forcing a healthcare or workforce
  ontology on the record.
- Match person records probabilistically and deterministically against
  arbitrary input, returning ranked candidates with per-component score
  breakdowns.
- Detect duplicate person records (real-time on create, batch on demand)
  and route them through a review queue with auto-merge for high-confidence
  matches.
- Carry the same healthcare-aware fields as the patient index (tax ID,
  identity documents, emergency contacts) so it can stand in as a
  patient index where a full healthcare deployment is not warranted.
- Emit HIPAA-grade audit logs and event-streaming records for every
  CRUD / merge / link operation.

## 2. Domain Model

### Person

Field-by-field reference: [`AGENTS/models.md`](AGENTS/models.md).
Material aspects:

- **Identity**: UUID `id` + multiple typed `identifiers` (MRN, SSN, DL,
  NPI, PPN, TAX, Other) + optional `tax_id` shortcut.
- **Names**: primary `name: HumanName` + `additional_names`; each name
  carries `use_type`, family, given, prefix, suffix.
- **Contact**: `telecom: Vec<ContactPoint>` (phone / fax / email / pager
  / url / sms / other), `addresses: Vec<Address>`.
- **Identity documents**: passport, birth certificate, national ID,
  driver's licence, voter ID, military ID, residence/work permit.
- **Emergency contacts**: name, relationship, telecom, address,
  `is_primary` flag.
- **Demographics**: `gender`, `birth_date`, `marital_status`,
  `multiple_birth`, `deceased` + `deceased_datetime`, `photo`.
- **Organisation**: `managing_organization` reference + per-person
  `links: Vec<PersonLink>` (ReplacedBy / Replaces / Refer / Seealso).
- **Audit**: `active` (soft-delete flag), `created_at`, `updated_at`.

### Supporting types

`Organization`, `MergeRequest` / `MergeResponse` / `MergeRecord`,
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`, `Consent`.

### Invariants

- `family` name must be non-empty.
- `birth_date`, when present, must not be in the future.
- An `Identifier` is keyed by `(identifier_type, system, value)`.
- `IdentityDocument.expiry_date`, when present, must be on or after
  `issue_date`.
- Soft-delete is the only delete.

## 3. Functional Capabilities

### 3.1 Identity management

- Create / read / update / soft-delete person records.
- Multiple identifiers per record (typed, system-qualified).
- Identity documents with expiry tracking.
- Multiple addresses, telecom contacts, and emergency contacts.
- Automatic event publish on every CRUD. See
  [`agents/share/auditability.md`](../agents/share/auditability.md).

### 3.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).

| Strategy | Output | Use |
|---|---|---|
| Probabilistic | Weighted sum 0.00–1.00 | Fuzzy input |
| Deterministic | Rule-based; short-circuits on tax-ID / identifier / document exact match | Hard guarantees |

Component weights: Name 0.30, DOB 0.25, Gender 0.10, Address 0.10,
Identifier 0.10, Tax ID 0.10, Document 0.05. Algorithms: Jaro-Winkler,
Levenshtein, Soundex phonetic bonus (+0.05 if codes match and score <
0.95), date proximity, weighted per-field address.

Match quality: ≥ 0.95 Definite · ≥ 0.85 Probable · ≥ 0.50 Possible · <
0.50 Unlikely (configurable).

### 3.3 Search

Powered by Tantivy across 11 indexed fields. Full-text + fuzzy +
phonetic, boolean syntax, pagination, optional sensitive-field masking.
Index stays synchronised with database writes; bulk re-index supported.

### 3.4 Duplicate detection & merging

- Real-time `409 Conflict` on `POST /api/persons` when matches exceed
  threshold.
- Explicit endpoint `POST /api/persons/check-duplicates`.
- Batch scan `POST /api/persons/deduplicate` with configurable
  `threshold`, `max_candidates`, `auto_merge_threshold`.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge transfers identifiers, names, addresses, contacts, documents,
  tax-ID, emergency contacts; appends duplicate's primary name as
  "former" alias on the survivor; adds `Replaces` link; soft-deletes
  duplicate; records JSON snapshot; emits `Merged` event.

### 3.5 Validation & normalisation

Required-field enforcement (family + given name), future-date guard on
birth date, tax-ID format, email regex, phone digit count, address
completeness, document number required + expiry guard, emergency-contact
name+relationship required. Phone normalised E.164-like; addresses
standardised. Failed validation → `422`.

### 3.6 Privacy

Per-field masking, GDPR Article 15 export at `GET
/api/persons/{id}/export`, masked view at `GET /api/persons/{id}/masked`,
consent model with type + status + dates, `has_active_consent()`
utility. See [`agents/share/privacy.md`](../agents/share/privacy.md).

### 3.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp. Queries: per-person, recent
system-wide, per-user. See
[`agents/share/auditability.md`](../agents/share/auditability.md).

## 4. Quality Attributes

| Attribute | Target |
|---|---|
| Scale | Millions of persons |
| Create latency (incl. dup-check + index + audit) | ≤ 50 ms p50 |
| Read | ≤ 5 ms p50 |
| Search | ≤ 100 ms p50 |
| Match | ≤ 500 ms p99 |
| Throughput | ≥ 1 000 req/sec single instance |
| Availability | HADR; stateless app tier; PostgreSQL replication |
| Fault tolerance | Graceful shutdown; connection pooling; health checks; non-root containers |
| Observability | OTLP traces / metrics / logs; `traceparent` per request; JSON logs in prod |

## 5. Technology Stack

Project-wide stack: [`agents/share/stack-for-rust-loco.md`](../agents/share/stack-for-rust-loco.md).
Crate-specific pinning:

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

Complete endpoint reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/persons/*` + `/api/audit/*` + `/api/health` |
| FHIR R5 (Axum) | `Person` resource CRUD + search under `/fhir/Person` |
| gRPC (Tonic) | Stubbed; not yet implemented |
| Web UI (Loco / Tera / HTMX / Alpine / Lily) | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

All REST endpoints return `{ "success": bool, "data": …, "error": … }`.
HTTP status codes follow REST conventions including `409` for duplicate
detection on create and `422` for validation failure.

## 7. Persistence

PostgreSQL 18+ via SeaORM. Schema overview: [`agents/share/postgresql.md`](../agents/share/postgresql.md).
Tables (12+):

`persons`, `person_names`, `person_identifiers`, `person_addresses`,
`person_contacts`, `person_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `person_match_scores`, `audit_log`.

Required PostgreSQL extensions: `pg_stat_statements`, `uuid-ossp`,
`pgcrypto`, `pg_trgm`, `citext`, `unaccent`. Optional: `pg_vector`,
`postgis`.

Connection pooling with configurable min/max; soft-delete is application
level (`active` flag); audit triggers retain history.

## 8. Testing & Quality

Strategy: [`AGENTS/testing.md`](AGENTS/testing.md).

- **Unit tests**: embedded in source under `#[cfg(test)]`; matching,
  phonetic, scoring, validation, privacy, models.
- **Integration tests**: `tests/`; full HTTP request/response cycles
  against real PostgreSQL + Tantivy.
- **Benchmarks**: Criterion suites for matching, search, validation.
- **CI**: `test.yml`, `quality.yml` (fmt + clippy), `security.yml`.

## 9. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, access tracking, encryption-at-rest (DB), soft delete |
| GDPR Art. 15 | `GET /api/persons/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Person resource bidirectional conversion |
| ISO/IEC 27001 | Operational controls (deployment-side) |

Healthcare frameworks: [`agents/share/compliance-for-healthcare.md`](../agents/share/compliance-for-healthcare.md).
Technology frameworks: [`agents/share/compliance-for-technology.md`](../agents/share/compliance-for-technology.md).

## 10. Implementation Status

### Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture, 40+ dependencies |
| Database schema | 12+ tables, SeaORM entities, indexes, audit triggers |
| Matching | Probabilistic + deterministic; Jaro-Winkler + Levenshtein + Soundex; configurable weights |
| Search | Tantivy 11-field index; fuzzy + phonetic + bulk + blocking |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| FHIR R5 | Person bidirectional conversion + search parameters + OperationOutcome |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher (Created/Updated/Deleted/Merged/Linked/Unlinked) |
| Audit log | AuditLogRepository with old/new JSON + user context |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, phone normalisation, address standardisation, 422 |
| Privacy | Field masking, GDPR export, consent model |
| Docker | Multi-stage Dockerfile, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |
| Documentation | README, CLAUDE.md, AGENTS/* set, architecture, deploy guide |

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
| Patient/person rename (in progress) | some lib paths still reference old `patient` symbols (see `src/db/audit.rs`); the web binary builds independently |

## 11. Roadmap

### Authentication & authorisation

JWT middleware on `/api/*`, RBAC, rate limiting, user-management
endpoints, security headers.

### Observability & monitoring

Prometheus metrics alongside OTLP, complete the OTLP trace exporter,
custom metrics (`person_created`, `match_score_histogram`, etc.),
Grafana dashboards + alerting.

### Performance optimisation

Database query caching (Redis or in-memory), N+1 batch fixes in the
repository, load test at realistic person volumes, profile and
optimise matching hot paths.

### Infrastructure as code

OpenTofu modules (PostgreSQL + app deploy), multi-cloud configuration
(GCP, AWS, Azure), secrets management, backup and DR automation.

### Kubernetes

Helm chart, HPA, persistent volume claims for the search index,
ingress controllers, Kubernetes health probes.

### Production readiness

Security audit and penetration test, HIPAA + GDPR compliance
validation, DR runbook + drills, backup and restore procedures,
incident-response procedures, CI/CD pipeline.

### Feature enhancements

Complete gRPC server, complete FHIR R5 (capability statement, bundles,
Organization), Fluvio production publisher + consumers, ML-based match
scoring with A/B test framework, person photo storage and retrieval,
consent enforcement in the query layer.

### Rename clean-up

Complete the patient→person rename in `src/db/audit.rs` and remaining
lib paths so `cargo check --lib` passes clean.

## 12. Change control

Material changes to this spec — domain-model fields, match-quality
thresholds, API-surface shape, compliance scope — should land in the
same commit as the code change. The cross-crate uniformity invariant
documented in the project-root [`spec.md`](../spec.md) applies to web
tier files only; this per-crate spec is local to the Main Person Index.
