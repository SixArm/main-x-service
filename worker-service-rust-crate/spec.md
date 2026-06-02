# Worker Service — Living Specification

> **Source of truth.** This document is the canonical artefact for the
> Worker Service crate. When code and spec disagree, the spec wins —
> open a task in §13 to bring the code in line, do not silently rewrite
> the spec.
>
> **Three-part PRs.** A behavioural change is one PR: spec edit + code
> edit + test edit. See [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md).

For shared infrastructure (technology stack, observability,
compliance), see the project-root [`AGENTS.md`](../AGENTS.md) and
[`agents/share/*`](../agents/share/).
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

The Worker Service is a centralised registry of **workforce and
professional identities**: operators, contractors, drivers, site
staff, field engineers — anyone whose role + credentials matter to
the caller.

### 1.2 Vision

One trustworthy record per worker, regardless of how many HR,
scheduling, credentialing, and payroll systems hold shards of that
identity:

- Carry credential / licence / professional-identifier fields (NPI,
  DEA, board licence, employee number) alongside the same
  fields the person index uses.
- Match probabilistically and deterministically against arbitrary
  input (typed name, partial NPI, credential number, …) and return
  ranked candidates with per-component score breakdowns.
- Detect duplicates in real time on create *and* in batch on demand.
- Emit HIPAA-grade audit logs and event-streaming records for every
  CRUD / merge / link operation.

### 1.3 Non-goals

- **Not** a credentialing / licensing source — link to the issuing
  authority; we record the credential, we do not validate it.
- **Not** a payroll system.
- **Not** an authentication / authorisation provider — JWT middleware
  is planned (§15) but identity proofing is out of scope.

## 2. Scope

### 2.1 In scope

- Worker identity CRUD with soft delete and full audit trail.
- Multiple identifiers per record (NPI, DEA, professional licence,
  MRN-style employee number, SSN, DL, TAX, Other).
- Identity / credential documents with type, number, issuing
  authority, issue / expiry dates, verified flag.
- Multiple addresses, telecom contacts, emergency contacts.
- Demographics (gender, birth date, marital status, multiple birth,
  deceased, photo).
- Managing organisation + per-worker links.
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy + phonetic search.
- Real-time + batch duplicate detection with review queue +
  auto-merge.
- Record merging with link tracking and JSON snapshots.
- Per-field privacy masking, GDPR Article 15 export, consent records.
- REST API (Axum) + FHIR R5 Practitioner + gRPC stub.
- PostgreSQL persistence via SeaORM.

### 2.2 Out of scope (today)

- Authentication / authorisation middleware (planned — §15).
- Production Fluvio publisher / consumers (today: in-memory stub).
- FHIR Organization resource and capability statement / bundles
  (Practitioner ✔; supporting resources partial).
- ML-based match scoring.
- Credential-expiry workflow / alerting (roadmap, §15).
- Role + assignment history timeline.

## 3. Stakeholders and Users

| Stakeholder | Interest |
|---|---|
| HR / credentialing officers | Authoritative worker record + credential history |
| API integrators | Stable REST + FHIR surface for worker CRUD, match, merge |
| Operations / DBA | PostgreSQL schema + migration discipline; backups |
| Compliance officer | HIPAA audit trail, GDPR export, consent records |
| Other Main X Index crates | Cross-references via `worker_id` |

## 4. Glossary

| Term | Meaning |
|---|---|
| **Worker** | The canonical record for an employee / contractor / professional |
| **Credential** | An `IdentityDocument` entry with type, number, issuing authority, expiry |
| **NPI** | National Provider Identifier (US); 10-digit identifier |
| **DEA** | Drug Enforcement Administration registration number |
| **Match quality** | Definite / Probable / Possible / Unlikely buckets |
| **Soft delete** | `active = false`; rows are never `DELETE`d |

## 5. Domain Model

Field-by-field reference: [`AGENTS/models.md`](AGENTS/models.md).

### 5.1 `Worker`

Material aspects:

- **Identity** — UUID `id` + `identifiers: Vec<Identifier>` + optional
  `tax_id` shortcut.
- **Names** — primary `name: HumanName` + `additional_names`
  (former names, name at credential issuance, married / maiden forms).
- **Contact** — `telecom: Vec<ContactPoint>`, `addresses: Vec<Address>`.
- **Identity / credential documents** — passport, driver's licence,
  professional credentials, certificates with type + number +
  issuing authority + issue / expiry dates + verified flag.
- **Emergency contacts** — name, relationship, telecom, address.
- **Demographics** — `gender`, `birth_date`, `marital_status`,
  `multiple_birth`, `deceased`, `photo`.
- **Organisation** — `managing_organization` reference + per-worker
  `links: Vec<WorkerLink>` (`ReplacedBy` / `Replaces` / `Refer` /
  `Seealso`).
- **Audit** — `active`, `created_at`, `updated_at`.

### 5.2 Supporting types

`Organization`, `MergeRequest` / `MergeResponse` / `MergeRecord`,
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`, `Consent`.

### 5.3 Invariants

The implementation MUST enforce:

- `name.family` is non-empty.
- `birth_date`, when present, is not in the future.
- An `Identifier` is unique within `(worker_id, identifier_type, system, value)`.
- `IdentityDocument.expiry_date`, when present, is on or after
  `issue_date`. Credentials with no expiry are non-expiring; an
  expiry in the past flags an expired credential but does not refuse
  the record.
- Soft delete is the only delete.

## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete worker records.
- Multiple professional identifiers per worker.
- Credential documents with expiry tracking.
- Multiple addresses, telecom contacts, emergency contacts.
- Automatic event publish on every CRUD.

### 6.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).

| Strategy | Output | Use |
|---|---|---|
| Probabilistic | Weighted sum 0.00–1.00 | Fuzzy input |
| Deterministic | Rule-based; short-circuits on identifier (NPI, DEA, employee #), tax-ID, or document exact match | Hard guarantees |

Default component weights:

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.30 | Jaro-Winkler + Levenshtein + Soundex bonus |
| Birth date | 0.25 | Date proximity |
| Gender | 0.10 | Exact / unknown handling |
| Address | 0.10 | Weighted postal / city / state / street |
| Identifier | 0.10 | Type + system + value match |
| Tax ID | 0.10 | Exact match (deterministic short-circuit to 1.0) |
| Document | 0.05 | Type + number match |

Match quality (configurable thresholds):

| Quality | Score |
|---|---|
| Definite | ≥ 0.95 |
| Probable | ≥ 0.85 |
| Possible | ≥ 0.50 |
| Unlikely | < 0.50 |

#### Interoperability with `worker-matcher`

The service embeds the sibling `worker-matcher` crate (declared in
`Cargo.toml`) and re-exports it from `src/matching/mod.rs` as
`matcher_lib`. The matcher crate is the **canonical reference
algorithm** — it carries 40+ national-identifier parsers (UK NHS,
FR NIR, US SSN, BR CPF, IN Aadhaar, …), passport-book matching,
blood-type signals, nickname tables, and three tuned config presets
(`strict` / `default` / `lenient`) that the in-service matcher does
not duplicate.

Bridge: [`src/matching/adapter.rs`](src/matching/adapter.rs) exposes
`to_matcher_worker(&service::Worker) -> worker_matcher::Worker`. The
projection lifts the service's FHIR-shaped record into the matcher's
flat builder shape using the same routing rules as the person bridge
(name flattening, telecom sampling by `ContactPointSystem`, address
field renames, identifier routing by `system` URI with type-based
fallbacks, passport documents → `passport_books`). Service-only
fields (`id`, `active`, `worker_type`, `deceased_datetime`,
`managing_organization`, `links`, `created_at`, …) are dropped.

The matcher's `uk_nhs_number` slot is the per-worker equivalent of
the person matcher's `uk_nhs_number` (both crates settled on the
shorter method name once published to crates.io). The service's
worker-specific
`IdentifierType::ODS` (NHS Organisation Data Service code) has no
country-slot counterpart and falls through unmapped; surface it on
the matcher side only if a future matcher release adds an ODS
parser. See [`AGENTS/matching.md`](AGENTS/matching.md) for the
in-service algorithm and the matcher crate's
[`spec.md §12`](../worker-matcher-rust-crate/spec.md) for the
canonical algorithm.

### 6.3 Search

Tantivy across 11 indexed fields (name, identifiers — including NPI
/ DEA — DOB year, addresses). Full-text + fuzzy + phonetic, boolean
syntax, pagination (`offset` + `limit`), optional sensitive-field
masking. Index stays synchronised with database writes.

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/workers`.
- Explicit `POST /api/workers/check-duplicates`.
- Batch `POST /api/workers/deduplicate` with configurable thresholds.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge transfers identifiers (credentials!), names, addresses,
  contacts, documents, tax-ID, emergency contacts; appends the
  duplicate's primary name as a "former" alias on the survivor;
  adds a `Replaces` link; soft-deletes the duplicate; records a JSON
  snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

Required-field enforcement (family + given name), future-date guard
on birth date, tax-ID format, email regex, phone digit count, address
completeness, document number required + expiry guard,
emergency-contact name + relationship required. Phone normalised
E.164-like; addresses standardised. Failed validation → `422`.

### 6.6 Privacy

Per-field masking, GDPR Article 15 export at
`GET /api/workers/{id}/export`, masked view at
`GET /api/workers/{id}/masked`, consent model with type + status +
dates, `has_active_consent()` utility. Sensitive fields specific to
workforce data (SSN, tax ID, DEA, home address) are masked by default
in the masked view. See
[`agents/share/privacy.md`](../agents/share/privacy.md).

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp. Queries: per-worker, recent
system-wide, per-user.

### 6.8 FHIR R5

Bidirectional Practitioner resource conversion under
`/fhir/Practitioner`. Search parameters: `name`, `family`, `given`,
`identifier`, `birthdate`, `gender`, `_count`.

## 7. Non-Functional Requirements

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
| Background jobs | Loco `BackgroundQueue` backed by **PostgreSQL** (`bg_pg`) — same database as application data; no external broker (no Redis, no SQLite) |

## 8. Architecture

### 8.1 Module layout

```
src/
├── api/
│   ├── mod.rs               # ApiResponse, ApiError
│   ├── rest/                # REST API (Axum) — 15 endpoints
│   ├── fhir/                # FHIR R5 Practitioner
│   └── grpc/                # Tonic stub
├── models/                  # Worker, HumanName, Identifier, …
├── db/                      # SeaORM entities + repositories + audit
├── matching/                # algorithms + scoring + phonetic
├── search/                  # Tantivy index + query
├── streaming/               # EventProducer trait + InMemoryEventPublisher
├── validation/              # boundary validators + normalisers
├── privacy/                 # masking + GDPR export + consent
├── config/                  # env loading + Config struct
├── observability/           # OTLP setup
├── error.rs
└── lib.rs
```

### 8.2 Layering rules

- `api/*` depends on `db`, `matching`, `search`, `streaming`,
  `validation`, `privacy`.
- `matching` and `search` MUST NOT depend on `api` or `db`
  repositories.
- `db` MUST NOT depend on `api`.
- `models` are leaves.

### 8.3 Trait-based abstraction

| Trait | Implementations |
|---|---|
| `WorkerRepository` | `SeaOrmWorkerRepository` |
| `WorkerMatcher` | `ProbabilisticMatcher`, `DeterministicMatcher` |
| `EventProducer` | `InMemoryEventPublisher` (Fluvio planned) |
| `EventConsumer` | stub |

### 8.4 Application state

`AppState` (`src/api/rest/state.rs`) holds `db`, `worker_repository`,
`event_publisher`, `audit_log`, `search_engine`, `matcher`, `config`.

### 8.5 Data flow

**Create:** HTTP POST → Validation → Duplicate detection → Repository
INSERT → Search Index → Event Publish → Audit Log → Response.

**Match:** HTTP POST → Search engine (blocking candidates) →
Repository GET → `Matcher::find_matches` → score + classify → Response.

**Merge:** HTTP POST → fetch both → transfer data → update survivor →
soft-delete duplicate → update index → publish `Merged` → Response.

## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/workers/*` + `/api/audit/*` + `/api/health` |
| FHIR R5 (Axum) | Practitioner CRUD + search under `/fhir/Practitioner` |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

## 10. Persistence

PostgreSQL 18+ via SeaORM.

### 10.1 Tables

`workers`, `worker_names`, `worker_identifiers`, `worker_addresses`,
`worker_contacts`, `worker_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `worker_match_scores`, `audit_log`.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`.

## 11. Testing Strategy

Layered: [`AGENTS/testing.md`](AGENTS/testing.md).

- **Unit tests** — `#[cfg(test)]` modules; matching, phonetic,
  scoring, validation, privacy, models. ~99 tests.
- **Integration tests** — `tests/api_integration_test.rs`; full HTTP
  request/response cycles against real PostgreSQL + Tantivy. 7+ tests.
- **Bridge integration tests** — `tests/duplicate_detection.rs`;
  drives service-side records through `adapter::to_matcher_worker` and
  asserts on `MatchingEngine::match_workers` end-to-end. Covers
  identical clones, name typos, deterministic identifier short-circuits,
  field-routing pinning, and config-preset invariants. 13 tests.
- **Benchmarks** — Criterion: matching, search, validation.
- **CI** — `test.yml`, `quality.yml`, `security.yml`.

## 12. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, access tracking, encryption-at-rest, soft delete |
| GDPR Art. 15 | `GET /api/workers/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Practitioner resource bidirectional conversion |
| ISO/IEC 27001 | Operational controls (deployment-side) |

Domain-specific compliance:
[`agents/share/compliance-for-healthcare.md`](../agents/share/compliance-for-healthcare.md).

## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [ ] **T-1 — JWT middleware on `/api/*`.**
  - [ ] Add `jsonwebtoken` validator extractor with HR-admin /
    credentialing-officer / read-only / service roles.
  - **Acceptance:** unauthenticated requests get `401`; valid signed
    token with sufficient role gets `2xx`.
- [ ] **T-2 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag `fluvio`.
  - **Acceptance:** integration test publishes a `WorkerCreated`
    record end-to-end against a local Fluvio broker.
- [ ] **T-3 — FHIR capability statement + bundle handling.**
  - [ ] `GET /fhir/metadata` returns a CapabilityStatement listing
    Practitioner.
  - [ ] `Bundle` GET / POST / search wrapping.
  - **Acceptance:** Touchstone FHIR validator passes on a sample
    bundle round-trip.
- [ ] **T-4 — FHIR Organization resource.**
  - [ ] Bidirectional Organization mapping.
  - **Acceptance:** `POST /fhir/Organization` round-trips a record.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [ ] **T-6 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `WorkerService.GetWorker`
    round-trips a record.
- [ ] **T-7 — Credential-expiry warning workflow.**
  - [ ] Background scan: `IdentityDocument.expiry_date` within 30
    days → publish `CredentialExpiringSoon` event.
  - [ ] Custom metric `credential_expiry_within_30d`.
  - **Acceptance:** integration test seeding a credential with
    `expiry_date = today + 25d` produces the event + metric.
- [ ] **T-8 — Role + assignment history timeline.**
  - [ ] Per-worker timeline of role / organisation assignments.
  - **Acceptance:** new assignment creates a timeline entry visible
    via `GET /api/workers/{id}/timeline`.

## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture, 40+ dependencies |
| Database schema | 12+ tables, SeaORM entities, indexes, audit triggers |
| Matching | Probabilistic + deterministic; Jaro-Winkler + Levenshtein + Soundex; configurable weights |
| Search | Tantivy 11-field index; fuzzy + phonetic + bulk + blocking |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| FHIR R5 | Practitioner bidirectional conversion + search parameters |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher (Created / Updated / Deleted / Merged / Linked / Unlinked) |
| Audit log | AuditLogRepository with old / new JSON + user context |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, phone normalisation, address standardisation, `422` |
| Privacy | Field masking, GDPR export, consent model |
| Docker | Multi-stage Dockerfile, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| Authentication / authorisation | T-1 |
| Fluvio production publisher | T-2 |
| FHIR capability statement | T-3 |
| FHIR bundle (full) | T-3 |
| FHIR Organization resource | T-4 |
| Event consumers | (no task yet) |
| Dedup / merge / privacy integration tests | T-5 |
| gRPC API | T-6 |
| Credential-expiry workflow | T-7 |
| Role / assignment history | T-8 |

## 15. Roadmap

- **Authentication & authorisation** — JWT, RBAC for HR-admin /
  credentialing-officer / service roles, rate limiting, user
  endpoints, security headers.
- **Observability** — Prometheus alongside OTLP, complete OTLP trace
  exporter, custom metrics (`worker_created`,
  `credential_expiry_within_30d`, …), Grafana dashboards + alerting.
- **Performance** — query caching, N+1 batch fixes, load test at
  realistic workforce volumes.
- **Infrastructure as code** — OpenTofu modules, multi-cloud, secrets,
  backup + DR.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index,
  ingress, probes.
- **Production readiness** — security audit + pen test, HIPAA + GDPR
  validation, DR runbook, backup / restore, CI/CD pipeline.
- **Feature enhancements** — complete gRPC, complete FHIR (capability
  statement, bundles, Organization), Fluvio production + consumers,
  ML-based match scoring, worker photo storage, consent enforcement,
  **credential-expiry-warning workflow**, **role + assignment history
  timeline**, NPI / DEA registry import pipelines.

## 16. Open Questions

- **OQ-1 — Credential validation.** Should we call out to an
  NPI / DEA / board licence registry to verify a credential at create
  time, or accept-and-flag for later verification?
- **OQ-2 — Cross-organisation merge.** Two workers with the same NPI
  registered under different `managing_organization` records — auto-
  merge, or always review-queue?
- **OQ-3 — Soft-delete vs deactivation.** A worker leaving an
  organisation is a different state from a duplicate being merged.
  Today both set `active = false`. Do we need a distinct
  `employment_status` field?

## 17. References

- Sibling specs: [person-service](../person-service-rust-crate/spec.md),
  [event-service](../event-service-rust-crate/spec.md),
  [place-service](../place-service-rust-crate/spec.md),
  [thing-service](../thing-service-rust-crate/spec.md).
- AGENTS reference set: [`AGENTS/index.md`](AGENTS/index.md).
- Shared docs: [`agents/share/index.md`](../agents/share/index.md).
- SDD discipline: [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md).

## 18. Change Control

Material changes to this spec — domain-model fields, match-quality
thresholds, API-surface shape, compliance scope — MUST land in the
same commit as the corresponding code change. This per-crate spec is local to the
Worker Service.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation.
