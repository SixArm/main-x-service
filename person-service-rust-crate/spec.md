# Person Service — Living Specification

> **Source of truth.** This document is the canonical artefact for the
> Person Service crate. When code and spec disagree, the spec wins —
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

The Person Service is a general-purpose centralised registry of
**person identities**. It sits alongside the more domain-specific
[Worker](../worker-service-rust-crate/) index and gives callers one
canonical record per real-world person regardless of how many source
systems hold a shard of that identity. It is healthcare-aware (tax ID,
identity documents, emergency contacts) so it can stand in as a patient
registry where a dedicated clinical index is not warranted.

### 1.2 Vision

A single, healthcare-aware person identity surface that:

- Carries every field the patient index does (tax ID, identity documents,
  emergency contacts) so it can stand in as a patient index where a full
  healthcare deployment is not warranted.
- Matches probabilistically and deterministically against arbitrary
  input, returning ranked candidates with per-component score breakdowns.
- Detects duplicates in real time on create *and* in batch on demand,
  routing them through a review queue with auto-merge for high-confidence
  matches.
- Emits HIPAA-grade audit logs and event-streaming records for every
  CRUD / merge / link operation.

### 1.3 Non-goals

- **Not** a system of record for clinical encounters, observations, or
  conditions — link out to the EHR or to the patient index.
- **Not** a workforce credentialing system — use the Worker Service.
- **Not** an authentication / authorisation provider — JWT middleware
  is planned (§15) but identity proofing is out of scope.

## 2. Scope

### 2.1 In scope

- Person identity CRUD with soft delete and full audit trail.
- Multiple identifiers per record (MRN, SSN, DL, NPI, PPN, TAX, Other).
- Identity documents (passport, driver's licence, national ID, …).
- Multiple addresses, telecom contacts, emergency contacts.
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy + phonetic search.
- Real-time + batch duplicate detection with review queue + auto-merge.
- Record merging with link tracking and transferred-data snapshots.
- Data validation + phone / address normalisation at the boundary.
- Per-field privacy masking, GDPR Article 15 export, consent model.
- REST API (Axum) + FHIR R5 Person + gRPC stub.
- Server-rendered web UI (Loco / Tera / HTMX / Alpine / Lily HTML
  Headless + NHS UK theme).
- PostgreSQL persistence via SeaORM, with migrations.
- Observability (tracing + OpenTelemetry OTLP).

### 2.2 Out of scope (today)

- Authentication / authorisation middleware (planned — §15).
- Production Fluvio publisher / consumers (today: in-memory stub).
- Complete FHIR bundle handling (Person resource ✔; bundles partial).
- ML-based match scoring.
- Person photo storage and retrieval.

## 3. Stakeholders and Users

| Stakeholder | Interest |
|---|---|
| API integrators | Stable REST + FHIR surface for person CRUD, match, merge |
| Operations / DBA | PostgreSQL schema + migration discipline; backups |
| Compliance officer | HIPAA audit trail, GDPR export, consent records |
| Frontend / portal teams | Server-rendered UI + JSON API |
| Other Main X Index crates | Cross-references via `person_id` |

## 4. Glossary

| Term | Meaning |
|---|---|
| **Person** | The canonical record for an individual, modelled with HumanName, identifiers, addresses, documents, emergency contacts |
| **Identifier** | Typed external reference (`(identifier_type, system, value)`) e.g. MRN / SSN / NPI |
| **Tax ID** | Effective tax identifier (`tax_id` field or TAX-type entry in `identifiers`) |
| **Match** | A comparison between two persons yielding a 0.00–1.00 score plus per-component breakdown |
| **Match quality** | Definite / Probable / Possible / Unlikely buckets keyed off configurable thresholds |
| **Merge** | An operation that transfers a duplicate's data onto a surviving record, soft-deletes the duplicate, and writes a `Replaces` link |
| **Review queue** | A persisted set of candidate duplicate pairs, each `Pending` / `Confirmed` / `Rejected` / `AutoMerged` |
| **Soft delete** | Persistence-level retention with `active = false`; never `DELETE FROM` |

## 5. Domain Model

Field-by-field reference: [`AGENTS/models.md`](AGENTS/models.md).

### 5.1 `Person`

Material aspects:

- **Identity** — UUID `id` + `identifiers: Vec<Identifier>` + optional
  `tax_id` shortcut.
- **Names** — primary `name: HumanName` + `additional_names`; each name
  carries `use_type`, family, given, prefix, suffix.
- **Contact** — `telecom: Vec<ContactPoint>`, `addresses: Vec<Address>`.
- **Identity documents** — passport, birth certificate, national ID,
  driver's licence, voter ID, military ID, residence / work permit.
- **Emergency contacts** — name, relationship, telecom, address,
  `is_primary` flag.
- **Demographics** — `gender`, `birth_date`, `marital_status`,
  `multiple_birth`, `deceased` + `deceased_datetime`, `photo`.
- **Organisation** — `managing_organization` reference + per-person
  `links: Vec<PersonLink>` (`ReplacedBy` / `Replaces` / `Refer` /
  `Seealso`).
- **Audit** — `active`, `created_at`, `updated_at`.

### 5.2 Supporting types

`Organization`, `MergeRequest` / `MergeResponse` / `MergeRecord`,
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`, `Consent`.

### 5.3 Invariants

The implementation MUST enforce:

- `name.family` is non-empty.
- `birth_date`, when present, is not in the future.
- An `Identifier` is unique within `(person_id, identifier_type, system, value)`.
- `IdentityDocument.expiry_date`, when present, is on or after `issue_date`.
- Soft delete (`active = false`) is the only delete; rows MUST NOT be
  removed.

## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete person records.
- Multiple identifiers (typed, system-qualified).
- Identity documents with expiry tracking.
- Multiple addresses, telecom, emergency contacts.
- Automatic event publish on every CRUD. See
  [`agents/share/auditability.md`](../agents/share/auditability.md).

### 6.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).

| Strategy | Output | Use |
|---|---|---|
| Probabilistic | Weighted sum 0.00–1.00 | Fuzzy input |
| Deterministic | Rule-based; short-circuits on tax-ID, identifier, or document exact match | Hard guarantees |

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

### 6.3 Search

Powered by Tantivy across 11 indexed fields. Full-text + fuzzy +
phonetic, boolean syntax, pagination (`offset` + `limit`), optional
sensitive-field masking. Index stays synchronised with database writes;
bulk re-index supported.

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/persons` when matches exceed
  the configured threshold.
- Explicit `POST /api/persons/check-duplicates`.
- Batch `POST /api/persons/deduplicate` with configurable `threshold`,
  `max_candidates`, `auto_merge_threshold`.
- Review queue with `Pending` / `Confirmed` / `Rejected` / `AutoMerged`.
- Merge transfers identifiers, names, addresses, contacts, documents,
  tax ID, emergency contacts; appends the duplicate's primary name as
  a "former" alias on the survivor; adds `Replaces` link; soft-deletes
  the duplicate; records a JSON snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

Required `family` + first `given` name; future-date guard on birth date;
tax-ID format; email regex; phone digit count; address completeness;
document number required + expiry guard; emergency-contact name +
relationship required. Phone normalised E.164-like; addresses
standardised. Failed validation → `422`.

### 6.6 Privacy

Per-field masking, GDPR Article 15 export at
`GET /api/persons/{id}/export`, masked view at
`GET /api/persons/{id}/masked`, consent model with type + status +
dates, `has_active_consent()` utility. See
[`agents/share/privacy.md`](../agents/share/privacy.md).

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp. Queries: per-person, recent
system-wide, per-user. See
[`agents/share/auditability.md`](../agents/share/auditability.md).

### 6.8 FHIR R5

Bidirectional Person resource conversion under `/fhir/Person`. Search
parameters: `name`, `family`, `given`, `identifier`, `birthdate`,
`gender`, `_count`. OperationOutcome on error.

## 7. Non-Functional Requirements

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
| Observability | OTLP traces / metrics / logs; `traceparent` per request; JSON logs in production |
| Security | Argon2 password hashing (when auth lands); JWT (planned); TLS at the edge |

## 8. Architecture

### 8.1 Module layout

```
src/
├── api/
│   ├── mod.rs               # ApiResponse, ApiError
│   ├── rest/                # REST API (Axum) — 15 endpoints
│   ├── fhir/                # FHIR R5 Person + bundle stubs
│   └── grpc/                # Tonic stub
├── models/                  # Person, HumanName, Identifier, …
├── db/                      # SeaORM entities + repositories + audit
├── matching/                # algorithms + scoring + phonetic
├── search/                  # Tantivy index + query
├── streaming/               # EventProducer trait + InMemoryEventPublisher
├── validation/              # boundary validators + normalisers
├── privacy/                 # masking + GDPR export + consent
├── config/                  # env loading + Config struct
├── observability/           # OTLP setup
├── web/                     # Loco app + Tera views + Axum web router
├── bin/
│   └── web.rs               # cargo run --bin web (binds 0.0.0.0:5150)
├── error.rs
└── lib.rs
```

### 8.2 Layering rules

- `api/*` depends on `db`, `matching`, `search`, `streaming`,
  `validation`, `privacy`.
- `matching` and `search` MUST NOT depend on `api` or `db`
  repositories — they take values, not connections.
- `db` MUST NOT depend on `api`.
- `models` are leaves — they depend on `serde`, `chrono`, `uuid` only.

### 8.3 Trait-based abstraction

| Trait | Implementations |
|---|---|
| `PersonRepository` | `SeaOrmPersonRepository` |
| `PersonMatcher` | `ProbabilisticMatcher`, `DeterministicMatcher` |
| `EventProducer` | `InMemoryEventPublisher` (Fluvio planned) |
| `EventConsumer` | stub |

### 8.4 Application state

`AppState` (`src/api/rest/state.rs`) holds:
`db`, `person_repository: Arc<dyn PersonRepository>`,
`event_publisher: Arc<dyn EventProducer>`,
`audit_log: Arc<AuditLogRepository>`,
`search_engine: Arc<SearchEngine>`,
`matcher: Arc<dyn PersonMatcher>`,
`config: Arc<Config>`.

### 8.5 Data flow

**Create:** HTTP POST → Validation → Duplicate detection → Repository
INSERT → Search Index → Event Publish → Audit Log → Response.

**Match:** HTTP POST → Search engine (blocking candidates) → Repository
GET → `Matcher::find_matches` → score + classify → Response.

**Merge:** HTTP POST → fetch both → transfer data → update main →
soft-delete duplicate → update index → publish `Merged` → Response.

## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/persons/*` + `/api/audit/*` + `/api/health` |
| FHIR R5 (Axum) | Person CRUD + search under `/fhir/Person` |
| gRPC (Tonic) | Stubbed; not yet implemented |
| Web UI | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

All REST endpoints return `{ "success": bool, "data": …, "error": … }`.
HTTP status codes follow REST conventions: `409` for duplicate
detection on create, `422` for validation failure.

## 10. Persistence

PostgreSQL 18+ via SeaORM. Schema overview:
[`agents/share/postgresql.md`](../agents/share/postgresql.md).

### 10.1 Tables (12+)

`persons`, `person_names`, `person_identifiers`, `person_addresses`,
`person_contacts`, `person_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `person_match_scores`, `audit_log`.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`. Optional: `pg_vector`, `postgis`.

### 10.3 Connection pooling

Configurable min / max via env (`DATABASE_MIN_CONNECTIONS` /
`DATABASE_MAX_CONNECTIONS`). Soft delete is application-level (`active`
flag); audit triggers retain history.

## 11. Testing Strategy

Layered: [`AGENTS/testing.md`](AGENTS/testing.md).

- **Unit tests** — embedded `#[cfg(test)]` modules; matching, phonetic,
  scoring, validation, privacy, models. ~100 tests.
- **Integration tests** — `tests/`; full HTTP request/response cycles
  against real PostgreSQL + Tantivy.
- **Benchmarks** — Criterion suites for matching, search, validation.
- **CI** — `test.yml`, `quality.yml` (`fmt --check` + `clippy`),
  `security.yml`.

## 12. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, access tracking, encryption-at-rest (DB), soft delete |
| GDPR Art. 15 | `GET /api/persons/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Person resource bidirectional conversion |
| ISO/IEC 27001 | Operational controls (deployment-side) |
| ISO/IEC 42001:2023 | AIMS controls (where matcher tuning is ML-driven) |

Healthcare-specific:
[`agents/share/compliance-for-healthcare.md`](../agents/share/compliance-for-healthcare.md).
Technology compliance:
[`agents/share/compliance-for-technology.md`](../agents/share/compliance-for-technology.md).

## 13. Tasks

Spec-driven work breakdown. Each task has an acceptance criterion;
tick the box when an automated test or clearly described manual check
confirms the criterion is met. Tasks small enough to land in a single
PR; split larger tasks (`T-12a`, `T-12b`).

- [ ] **T-1 — Wire JWT middleware on `/api/*`.**
  - [ ] Add `jsonwebtoken` validator extractor.
  - [ ] Reject unauthenticated requests with `401`.
  - **Acceptance:** integration test posts without a token → `401`;
    posts with a valid signed token → `2xx`.
- [ ] **T-2 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag `fluvio`.
  - [ ] Document failover behaviour when the broker is unreachable.
  - **Acceptance:** integration test against a local Fluvio broker
    publishes a `PersonCreated` event end-to-end.
- [ ] **T-3 — Complete FHIR bundle handling.**
  - [ ] `Bundle` GET / POST / search wrapping.
  - [ ] OperationOutcome on malformed bundles.
  - **Acceptance:** Touchstone FHIR validator passes on a sample
    bundle round-trip.
- [ ] **T-4 — FHIR capability statement endpoint.**
  - [ ] `GET /fhir/metadata` returns a CapabilityStatement listing
    supported resources + interactions.
  - **Acceptance:** schema check against R5 CapabilityStatement.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [ ] **T-6 — gRPC implementation.**
  - [ ] Promote stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `PersonService.GetPerson`
    round-trips a record.
- [ ] **T-7 — Spec-drift CI check.**
  - [ ] Fail PR if `src/matching/**` or `src/models/person.rs`
    changes without a `spec.md` edit (allowlist in `.spec-allow`).
  - **Acceptance:** `bash scripts/spec-drift-check.sh main HEAD`
    exits non-zero on a code-only PR.
- [ ] **T-8 — `db::audit` rename clean-up.**
  - [ ] Verify no `patient`-era symbols remain in `src/db/audit.rs`.
  - **Acceptance:** `cargo check --lib` passes clean; `grep -ri
    'patient' src/db/` returns no matches.

## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture, 40+ dependencies |
| Database schema | 12+ tables, SeaORM entities, indexes, audit triggers |
| Matching | Probabilistic + deterministic; Jaro-Winkler + Levenshtein + Soundex; configurable weights |
| Search | Tantivy 11-field index; fuzzy + phonetic + bulk + blocking |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| FHIR R5 | Person bidirectional conversion + search parameters + OperationOutcome |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher (Created / Updated / Deleted / Merged / Linked / Unlinked) |
| Audit log | AuditLogRepository with old / new JSON + user context |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, phone normalisation, address standardisation, `422` |
| Privacy | Field masking, GDPR export, consent model |
| Web UI | Loco / Tera / HTMX / Alpine / Lily HTML Headless + NHS UK theme |
| Docker | Multi-stage Dockerfile, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |
| Documentation | README, CLAUDE.md, AGENTS/* set, architecture, deploy guide, this spec |

### 14.2 Open gaps

Open gaps drive tasks in §13. Live gap list:

| Gap | Task |
|---|---|
| FHIR capability statement | T-4 |
| FHIR bundle (full) | T-3 |
| FHIR Organization resource | (no task yet — open in §16) |
| Fluvio production publisher | T-2 |
| Event consumers | (no task yet) |
| gRPC API | T-6 |
| Dedup / merge / privacy integration tests | T-5 |
| Authentication / authorisation | T-1 |
| Spec-drift CI guard | T-7 |
| `db::audit` rename clean-up | T-8 |

## 15. Roadmap

Roadmap items become §13 tasks when they are concrete enough to size
and accept.

- **Authentication & authorisation** — JWT, RBAC, rate limiting, user
  endpoints, security headers.
- **Observability** — Prometheus alongside OTLP, complete OTLP trace
  exporter, custom metrics (`person_created`, `match_score_histogram`,
  …), Grafana dashboards + alerting.
- **Performance** — query caching (Redis or in-memory), N+1 batch
  fixes, load test at realistic person volumes, profile matching hot
  paths.
- **Infrastructure as code** — OpenTofu modules (PostgreSQL + app
  deploy), multi-cloud (GCP, AWS, Azure), secrets management, backup
  and DR automation.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index, ingress
  controllers, Kubernetes health probes.
- **Production readiness** — security audit + pen test, HIPAA + GDPR
  validation, DR runbook + drills, backup / restore, incident
  response, CI/CD pipeline.
- **Feature enhancements** — complete gRPC, complete FHIR (capability
  statement, bundles, Organization), Fluvio production publisher +
  consumers, ML-based match scoring with A/B test framework, person
  photo storage and retrieval, consent enforcement in the query layer.

## 16. Open Questions

- **OQ-1 — FHIR Organization resource.** Do we expose `/fhir/Organization`
  here, or push it into a separate `organization-service` crate? Spec
  does not yet decide.
- **OQ-2 — Tax-ID short-circuit threshold.** Currently any TAX-type
  identifier match short-circuits to 1.0. Should we require both records
  to also share birth-year before the short-circuit fires, to avoid
  false positives on shared corporate tax IDs?
- **OQ-3 — Consent enforcement.** Should the query layer hide records
  lacking active `DataProcessing` consent, or surface them with a
  `consent_required: true` flag and leave the filtering to the caller?

Open questions resolve into §13 tasks or §5–§9 amendments when
decisions are made.

## 17. References

- Sibling specs: [event-service](../event-service-rust-crate/spec.md),
  [worker-service](../worker-service-rust-crate/spec.md),
  [place-service](../place-service-rust-crate/spec.md),
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
Person Service.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation. Avoid re-flowing surrounding paragraphs
in the same PR as a content change — keep stylistic churn out of
behavioural diffs.
