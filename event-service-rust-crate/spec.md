# Event Service — Living Specification

> **Source of truth.** This document is the canonical artefact for the
> Event Service crate. When code and spec disagree, the spec wins —
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

The Event Service is a centralised registry of **time-bounded events**:
appointments, encounters, shifts, sessions, deliveries, incidents,
scheduled tasks — anything that can be canonicalised as "a thing
happening, between a start time and an end time, involving parties and
a place." The domain model is aligned with
[schema.org/Event](https://schema.org/Event).

### 1.2 Vision

A single trustworthy view of each event regardless of how many
scheduling, EHR, CRM, calendar, or operational systems hold a shard
of that event:

- Match probabilistically and deterministically against arbitrary input
  (party + approximate time, identifier + organisation, partial title +
  venue) and return ranked candidates with per-component score breakdowns.
- Detect duplicates in real time on create *and* in batch on demand —
  for example, the same appointment created by both the patient portal
  and the front-desk EHR.
- Expose a stable cross-system identifier so downstream analytics,
  billing, and notifications refer to one event ID per real-world
  occurrence.
- Emit audit logs and event-streaming records for every CRUD / merge /
  link operation. ("Event streaming" here is the Fluvio pipe for
  *index-level* changes, not the modelled domain events themselves.)

### 1.3 Non-goals

- **Not** a calendaring engine — RFC 5545 recurrence is a roadmap item
  (§15), not a current capability.
- **Not** a scheduler — events are recorded, not allocated against
  resources.
- **Not** a notification / reminder system — downstream consumers may
  build that on top of the event stream.

## 2. Scope

### 2.1 In scope

- Event identity CRUD with soft delete and full audit trail.
- schema.org/Thing properties (`name`, `description`, `alternateName`,
  `url`, `image`, `sameAs`, `keywords`).
- Time-window fields (`start_date` required, `end_date`, `door_time`,
  `duration` ISO 8601, `previous_start_date`, `time_zone`, `all_day`).
- Status / mode / type taxonomies aligned with schema.org/Event.
- Capacity fields (total / physical / virtual / remaining).
- `Location` as a union of `Place` / `PostalAddress` / `VirtualLocation`
  / `Text`.
- Parties (organizers, performers, attendees, sponsors, funders,
  contributors).
- Offers (price, currency, availability, validity window).
- Multiple identifiers (`BookingNumber`, `ConfirmationCode`,
  `TicketNumber`, `EncounterId`, `TransactionId`, `ExternalRef`,
  `Tax`, `Other`).
- `super_event` / `sub_events` hierarchy.
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy search with date-range filter.
- Real-time + batch duplicate detection + review queue.
- Record merging with link tracking and JSON snapshots.
- Per-field privacy masking, GDPR Article 15 export, consent records.
- REST API (Axum) + gRPC stub.
- Server-rendered web UI.
- PostgreSQL persistence via SeaORM.

### 2.2 Out of scope (today)

- FHIR R5 surface — stubbed `501` until the Event → Encounter /
  Appointment mapping is fixed.
- Recurrence (RFC 5545 RRULE).
- Time-zone-aware fuzzy matching (uses naive UTC offsets today).
- Production Fluvio publisher / consumers.
- ML-based match scoring.

## 3. Stakeholders and Users

| Stakeholder | Interest |
|---|---|
| Scheduling / EHR integrators | Stable REST surface for create / read / search |
| Operations / DBA | PostgreSQL schema + migration discipline |
| Compliance officer | Audit trail, GDPR export, consent records |
| Frontend / portal teams | Server-rendered UI + JSON API |
| Other Main X Index crates | Cross-references via `event_id` |

## 4. Glossary

| Term | Meaning |
|---|---|
| **Event** | A time-bounded occurrence with parties + (optional) location + offers |
| **Strong identifier** | `BookingNumber`, `ConfirmationCode`, `TicketNumber`, `EncounterId`, `TransactionId` — match short-circuits to 1.0 |
| **Location** | Union of `Place`, `PostalAddress`, `VirtualLocation`, `Text` |
| **Party** | Typed reference (`Person` or `Organization`) with name, optional external ID, email, URL |
| **Match quality** | Definite / Probable / Possible / Unlikely buckets keyed off configurable thresholds |
| **Window overlap** | Jaccard ratio of two `[start, end)` intervals |
| **Soft delete** | `active = false`; rows are never `DELETE`d |

## 5. Domain Model

Field-by-field reference: [`AGENTS/models.md`](AGENTS/models.md).

### 5.1 `Event`

Material aspects:

- **Identity** — UUID `id` + `identifiers: Vec<Identifier>` with the
  typed enum above; multiple per event.
- **Thing properties** — `name` (required), `alternate_names`,
  `description`, `disambiguating_description`, `url`, `image`,
  `same_as`, `keywords`.
- **Time window** — `start_date` (required, UTC), `end_date`,
  `door_time`, `duration` (ISO 8601), `previous_start_date`,
  `time_zone` (IANA, display-only), `all_day`.
- **Status / mode / type** — `event_status`, `event_attendance_mode`,
  `event_type` (29 variants).
- **Audience & accessibility** — `typical_age_range`, `in_language`
  (ISO 639-1), `is_accessible_for_free`, capacity totals.
- **Location** — `Vec<Location>` (Place / PostalAddress /
  VirtualLocation / Text).
- **Parties** — `organizers`, `performers`, `attendees`, `sponsors`,
  `funders`, `contributors`.
- **Hierarchy** — `super_event`, `sub_events`.
- **Works** — `about`, `works`.
- **Offers** — `Vec<Offer>` with price + currency + availability.
- **Audit** — `active`, `created_at`, `updated_at`.

### 5.2 Supporting types

`Location`, `Place`, `VirtualLocation`, `Address`, `Party`,
`Reference`, `Offer`, `EventLink`, `Organization`, `MergeRequest` /
`MergeResponse` / `MergeRecord`, `ReviewQueueItem`,
`BatchDeduplicationRequest` / `Response`, `Consent`.

### 5.3 Invariants

The implementation MUST enforce:

- `name` is non-empty.
- `start_date` is present.
- `end_date ≥ start_date` when both are present.
- `door_time ≤ start_date` when present.
- `maximum_physical + maximum_virtual ≤ maximum_total` when all three
  are set.
- `remaining ≤ maximum_total` when both are set.
- `EventAttendanceMode::Online` requires at least one
  `Location::Virtual`.
- `EventAttendanceMode::Mixed` requires at least one physical and one
  virtual location.
- An `Identifier` is unique within `(event_id, identifier_type, system, value)`.
- Cancelled events are not deleted — `event_status` changes;
  `active` remains `true` until an explicit soft-delete.
- Soft delete is the only delete.

## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete event records.
- Multiple identifiers per event (typed, system-qualified).
- Status transitions tracked via the audit log.
- Automatic event publish on every CRUD.

### 6.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).

Default component weights (sum to 1.0):

| Component | Weight | Algorithm |
|---|---:|---|
| Name (title + alternates) | 0.20 | Jaro-Winkler + Levenshtein + Soundex floor |
| Start date | 0.20 | Exponential decay (1 h half-life) |
| End date | 0.10 | Exponential decay or window overlap |
| Location | 0.15 | Place-id exact / address fuzzy / virtual URL / text |
| Organizer | 0.10 | Party id exact / name fuzzy / email exact |
| Performer | 0.10 | Same |
| Attendee | 0.05 | Same |
| Identifier | 0.10 | Type + system + value match |

Deterministic short-circuit: exact value match on a **strong**
identifier (`BookingNumber`, `ConfirmationCode`, `TicketNumber`,
`EncounterId`, `TransactionId`) → 1.0.

Match quality:

| Quality | Score |
|---|---|
| Definite | ≥ 0.95 |
| Probable | ≥ 0.85 |
| Possible | ≥ 0.50 |
| Unlikely | < 0.50 |

### 6.3 Search

Tantivy across `name`, `alternate_names`, `description`, `keywords`,
`organizer_name`, `performer_name`, `identifier_value`, plus faceted
fields for `event_status`, `event_attendance_mode`, `event_type`,
`in_language`, location city / country / URL, and `start_date`
(yyyy-mm-dd for range queries). Full-text + fuzzy + boolean.
Pagination (`offset` + `limit`). Optional masking for events with
sensitive identifiers or party emails.

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/v1/events` when the blocking
  step (name + start-date) yields a probable match above the
  threshold.
- Explicit `POST /api/v1/events/check-duplicates`.
- Batch `POST /api/v1/events/deduplicate`.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge picks the surviving record; transfers identifiers, alternate
  names, keywords, locations, parties, and `same_as` URLs; appends the
  duplicate's primary name as an `alternate_name` on the survivor;
  adds a `Replaces` link; soft-deletes the duplicate; records a JSON
  snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

`name` required; `start_date` required; `end_date ≥ start_date` when
both present; `door_time ≤ start_date`; `duration` parsed as ISO 8601;
`in_language` entries 2-letter ISO 639-1; `Online` attendance mode
expects a `Virtual` location; `Mixed` expects one physical + one
virtual; capacities non-negative and consistent; per-location checks
(place name, lat/lon range, URL scheme, address completeness);
per-offer checks (3-letter ISO 4217 currency, parseable price,
`valid_from ≤ valid_through`). Failed validation → `422`.

### 6.6 Privacy

Per-field masking of identifier values (often double as access tokens)
and party emails; external party IDs stripped from the masked view.
GDPR Article 15 export at `GET /api/v1/events/{id}/export`. Consent
records (`Consent` model) let callers grant / revoke processing /
sharing / marketing / research consent per event. See
[`agents/share/privacy.md`](../agents/share/privacy.md).

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp.

### 6.8 FHIR R5

**Stubbed.** `/fhir/Event/*` returns `501 Not Implemented` with an
`OperationOutcome` body until the schema.org/Event → FHIR R5 mapping
is fixed. See OQ-1.

## 7. Non-Functional Requirements

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

## 8. Architecture

### 8.1 Module layout

```
src/
├── api/
│   ├── mod.rs               # ApiResponse, ApiError
│   ├── rest/                # /api/v1/* — 15 endpoints
│   ├── fhir/                # 501 stub + OperationOutcome
│   └── grpc/                # Tonic stub
├── models/                  # Event, Location, Party, Offer, …
├── db/                      # SeaORM entities + repositories + audit
├── matching/                # algorithms + scoring + phonetic
├── search/                  # Tantivy index + query
├── streaming/               # EventProducer trait + InMemoryEventPublisher
├── validation/              # boundary validators + normalisers
├── privacy/                 # masking + GDPR export + consent
├── config/                  # env loading + Config struct
├── observability/           # OTLP setup
├── web/                     # Loco app + Tera views + Axum web router
├── bin/web.rs               # cargo run --bin web (binds 0.0.0.0:5150)
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
| `EventRepository` | `SeaOrmEventRepository` |
| `EventMatcher` | `ProbabilisticMatcher`, `DeterministicMatcher` |
| `EventProducer` | `InMemoryEventPublisher` (Fluvio planned) |
| `EventConsumer` | stub |

### 8.4 Application state

`AppState` (`src/api/rest/state.rs`) holds `db`, `event_repository`,
`event_publisher`, `audit_log`, `search_engine`, `matcher`, `config`.

### 8.5 Data flow

**Create:** HTTP POST → Validation → Duplicate detection (blocking on
name + start-date) → Repository INSERT → Search Index → Event Publish
→ Audit Log → Response.

**Match:** HTTP POST → Search engine (date-window candidates) →
Repository GET → `Matcher::find_matches` → score + classify → Response.

**Merge:** HTTP POST → fetch both → transfer data → update survivor →
soft-delete duplicate → update index → publish `Merged` → Response.

## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/v1/events/*` + `/api/v1/audit/*` + `/api/v1/health` |
| FHIR R5 (Axum) | `501 Not Implemented` stub (see §6.8) |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` |

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

## 10. Persistence

PostgreSQL 18+ via SeaORM.

### 10.1 Tables

- `events` — scalar columns (name, description, time window,
  status / mode / type, capacities, audit cols) plus JSONB arrays for
  `alternate_names`, `image`, `same_as`, `keywords`, `in_language`.
- `event_identifiers`, `event_locations`, `event_parties`,
  `event_offers`, `event_links`, `event_sub_events`.
- `organizations`, `organization_addresses`,
  `organization_contacts`, `organization_identifiers`.
- `audit_log`.

### 10.2 Extensions

Required: `pgcrypto`, `pg_trgm`.
Optional: `citext`, `unaccent`, `btree_gist` (for "no overlapping
events per resource" exclusion constraints).

## 11. Testing Strategy

Layered: [`AGENTS/testing.md`](AGENTS/testing.md).

- **Unit tests** — `#[cfg(test)]` modules; matching, scoring,
  validation, privacy, models, time-interval algebra. 62+ tests.
- **Integration tests** — `tests/`; full HTTP request/response
  cycles against real PostgreSQL + Tantivy.
- **Benchmarks** — Criterion for matching, search, validation.
- **CI** — `test.yml`, `quality.yml`, `security.yml`.

## 12. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA (when used clinically) | Audit log, access tracking, encryption-at-rest, soft delete |
| GDPR Art. 15 | `GET /api/v1/events/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Stub; mapping pending (§6.8, OQ-1) |
| ISO/IEC 27001 | Operational controls (deployment-side) |

## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [ ] **T-1 — FHIR R5 mapping decision + implementation.**
  - [ ] Decide Encounter vs Appointment vs other event-pattern (OQ-1).
  - [ ] Implement bidirectional conversion for the chosen resource.
  - **Acceptance:** `POST /fhir/Event` round-trips through the chosen
    resource; OperationOutcome on errors.
- [ ] **T-2 — Time-zone-aware fuzzy matching.**
  - [ ] Replace naive UTC offsets with `chrono-tz` conversions in the
    date-proximity scorer.
  - **Acceptance:** unit test where one event in `America/New_York`
    matches another in `UTC` at the same wall-clock instant.
- [ ] **T-3 — RFC 5545 RRULE recurrence support.**
  - [ ] Add `recurrence_rule: Option<String>` to `Event`.
  - [ ] Implement expansion for search + dedup.
  - **Acceptance:** weekly RRULE expanded into 52 occurrences for
    range queries.
- [ ] **T-4 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag.
  - **Acceptance:** integration test publishes an `EventCreated`
    record end-to-end.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [ ] **T-6 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `EventService.GetEvent`
    round-trips a record.
- [ ] **T-7 — iCalendar import / export.**
  - [ ] `POST /api/v1/events/import.ics`, `GET /api/v1/events/{id}.ics`.
  - **Acceptance:** Apple Calendar imports the exported `.ics`
    without warnings.
- [ ] **T-8 — Authentication / authorisation.**
  - [ ] JWT middleware on `/api/v1/*` with scheduler / admin /
    read-only / service roles.
  - **Acceptance:** unauthenticated requests get `401`; valid token
    + role gets `2xx`.

## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Database schema | Tables + SeaORM entities + indexes + audit triggers |
| Matching | Probabilistic + deterministic; configurable weights |
| Search | Tantivy index; fuzzy + bulk + name+date blocking |
| REST API | Core endpoints + OpenAPI/Swagger + CORS + structured errors |
| Repository | SeaORM CRUD with transactions, soft delete |
| Event streaming | InMemoryEventPublisher (index-level events) |
| Audit log | AuditLogRepository with old / new JSON |
| Duplicate detection | Real-time + explicit + batch with review queue |
| Merging | Transfer + alias + link + soft-delete + snapshot + event |
| Validation | Required fields, format checks, time-window guards, `422` |
| Privacy | Field masking, GDPR export, consent model |
| Web UI | Loco / Tera / HTMX / Alpine / Lily HTML Headless + NHS UK theme |
| Docker | Multi-stage Dockerfile, dev + test Compose |
| Tests | Unit + integration + Criterion benchmarks; CI workflows |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| FHIR Event mapping | T-1 (open question OQ-1) |
| Time-zone-aware fuzzy matching | T-2 |
| Recurrence / RRULE | T-3 |
| Fluvio production publisher | T-4 |
| Event consumers | (no task yet) |
| Dedup / merge / privacy integration tests | T-5 |
| gRPC API | T-6 |
| iCalendar I/O | T-7 |
| Authentication / authorisation | T-8 |

## 15. Roadmap

- **Authentication & authorisation** — JWT, RBAC for scheduler /
  admin / read-only / service, rate limiting, security headers.
- **Observability** — Prometheus alongside OTLP, complete OTLP trace
  exporter, custom metrics (`event_created`, `event_duration_seconds`,
  `match_score`), Grafana dashboards + alerting.
- **Performance** — time-range query caching, btree_gist exclusion
  constraints for no-overlap policies, load test at realistic event
  volumes.
- **Infrastructure as code** — OpenTofu modules, multi-cloud, secrets,
  backup + DR.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index,
  ingress, probes.
- **Production readiness** — security audit + pen test, GDPR
  validation, DR runbook, backup / restore, CI/CD pipeline.
- **Feature enhancements** — complete gRPC; complete FHIR (capability
  statement, bundles, Encounter / Appointment); Fluvio production +
  consumers; ML-based match scoring; iCalendar import / export; RFC
  5545 RRULE recurrence; time-zone-aware fuzzy matching; consent
  enforcement in the query layer.

## 16. Open Questions

- **OQ-1 — FHIR mapping.** Encounter or Appointment (or both)?
  Encounter fits clinical visits; Appointment fits scheduling. A
  "best-fit by `event_type`" dispatch is a third option.
- **OQ-2 — Capacity invariant strictness.** Should we reject events
  where `remaining > maximum_total` outright (422), or accept and
  warn? Today: reject.
- **OQ-3 — `previous_start_date` semantics.** Required when
  `event_status == Rescheduled`? Today: not required, but consumers
  expect it.

## 17. References

- Sibling specs: [person-service](../person-service-rust-crate/spec.md),
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
Event Service.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation.
