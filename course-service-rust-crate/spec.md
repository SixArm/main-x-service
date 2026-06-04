# Course Service — Living Specification

> **Source of truth.** When code and spec disagree, the spec wins —
> open a task in §13 to bring the code in line, do not silently
> rewrite the spec.
>
> **Three-part PRs.** A behavioural change is one PR: spec edit +
> code edit + test edit. See
> [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md).

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

The Course Service is a registry of **course identities**. It models
the abstract course (`schema.org/Course` — "CS101 Introduction to
Computer Science") separately from its specific offerings
(`schema.org/CourseInstance` — "CS101, Fall 2026, Prof. Smith,
in-person, Tue/Thu 09:00"). One course → many instances.

It sits in the Main X Index family between the more-abstract
[Thing Service](../thing-service-rust-crate/) (anything with an
identity) and the time-bounded
[Event Service](../event-service-rust-crate/) (occurrences with
locations and parties). A `Course` is a template; a `CourseInstance`
is closer to an `Event` and may eventually reference one.

### 1.2 Vision

A single course-identity surface that:

- Carries every property from `schema.org/Course` /
  `schema.org/CourseInstance` / `LearningResource` / `CreativeWork` /
  `Thing` that is relevant to interoperability with LMS, OER, and
  catalog systems.
- Matches probabilistically (name + course-code + provider +
  educational-level + topic / teaches) and deterministically (DOI,
  Wikidata, LMS course-id, OER id, URI, UUID).
- Detects duplicates in real time on create *and* in batch on
  demand, routing them through a review queue with auto-merge for
  high-confidence matches.
- Emits audit logs and event-streaming records suitable for HIPAA-
  / FERPA-grade trails.

### 1.3 Non-goals

- **Not** a learning-management system. We do not store enrollments,
  grades, submissions, or content. We point at LMSs via the
  `CourseInstance.location_id` / `instructor_ids` references and the
  external-identifier collection.
- **Not** a credential issuer. We model the **shape** of an
  `EducationalCredential` awarded by a course; we do not issue Open
  Badges or Verifiable Credentials.
- **Not** a marketplace. No payment, enrollment, or course-discovery
  ranking algorithms.
- **Not** an authentication / authorisation provider. JWT auth is
  planned (§15) but identity proofing is out of scope.

## 2. Scope

### 2.1 In scope

- Course identity CRUD with soft delete and audit trail.
- CourseInstance sub-resource (multiple instances per course; each
  with schedule, mode, instructors, location).
- Multiple identifiers per course (LMS id, course code, platform
  slug, DOI, Wikidata, ISCED, ROR, URI, UUID, custom).
- Educational-credential references (degree / diploma / certificate
  / micro-credential / badge / license).
- Syllabus sections (hierarchical table of contents).
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy + phonetic search.
- Real-time + batch duplicate detection with review queue +
  auto-merge.
- Record merging with link tracking and transferred-data snapshots.
- Data validation + normalisation at the boundary.
- Per-field privacy masking, GDPR Article 15 export.
- REST API (Axum) at `/api/courses` and `/api/courses/{id}/instances`.
- PostgreSQL persistence via SeaORM, with migrations.
- Observability (tracing + OpenTelemetry OTLP).

### 2.2 Out of scope (MVP)

- Authentication / authorisation middleware (planned — §15).
- gRPC API (stub only).
- FHIR resource mapping (no FHIR resource fits Course cleanly).
- ML-based match scoring.
- LMS round-trip integration (LTI / xAPI / SCORM).
- Course-discovery ranking.
- Enrollment / grade storage.

## 3. Stakeholders and Users

| Persona | Need |
|---|---|
| Catalog operator | Day-to-day course CRUD; instance management. |
| Data integration engineer | Bulk import from LMS / OER repositories with deduplication. |
| Academic affairs | Audit trail for course-catalog changes. |
| Front-end developer | Stable REST contract for [`course-front-end-with-svelte`](../course-front-end-with-svelte/). |
| Search consumer | Full-text and fuzzy lookup with stable ranking. |

## 4. Glossary

| Term | Meaning |
|---|---|
| **Course** | The abstract template (course code, name, topics taught). |
| **CourseInstance** | A specific offering of a Course at a particular time / place / mode. |
| **Provider** | The organisation that issues / owns the course. |
| **Deterministic identifier** | An identifier scheme whose values are unique by construction across providers — DOI, Wikidata, LMS id (when LMS-scoped), URI, UUID, OER id. |
| **Course code** | A provider-scoped identifier (`CS101`). NOT globally unique. |
| **Envelope** | `{ success, data, error }` wrapper applied to every REST response. |

## 5. Domain Model

The model surface mirrors `schema.org/Course` field-for-field where
sensible. The complete property table — schema.org → Rust mapping
— is in [`AGENTS/models.md`](AGENTS/models.md). High-level shape:

- `Course` (the template) — Thing + CreativeWork + LearningResource
  + Course-specific properties (course_code, number_of_credits,
  course_prerequisites, available_language, financial_aid_eligible,
  educational_credential_awarded, total_historical_enrollment,
  syllabus_sections, instances).
- `CourseInstance` — schedule, mode (online / onsite / blended /
  self-paced), instructors, location, capacity, enrollment window.
- `Provider` — the issuing organisation.
- `CourseIdentifier` — `{ property_id, value, name?, url? }`
  matching `schema.org/PropertyValue`. `property_id` enumerates the
  scheme; `is_deterministic()` exposes which schemes short-circuit
  matching.
- `Syllabus` — hierarchical table-of-contents node with `teaches`,
  `time_required`, and `sub_sections`.
- `EducationalCredential` — schema.org/EducationalOccupationalCredential.
- `MergeRequest` / `MergeResponse` / `ReviewQueueItem` — same shape
  as the sibling services.

## 6. Functional Requirements

| ID | Requirement |
|---|---|
| FR-1 | `POST /api/courses` MUST validate (FR-21..FR-25), MUST short-circuit duplicate-detection on a deterministic identifier match, MUST return `201` + `Course` on success, `409` + `MatchResult[]` on probable duplicate, `422` + field errors on validation failure. |
| FR-2 | `GET /api/courses/{id}` MUST return the full Course including its `instances` collection. |
| FR-3 | `PUT /api/courses/{id}` MUST replace the Course (excluding `instances`, which have their own endpoints). |
| FR-4 | `DELETE /api/courses/{id}` MUST soft-delete (sets `deleted_at`, hides from search). |
| FR-5 | `GET /api/courses/search` MUST support `q`, `limit`, `offset`, `fuzzy`, `phonetic`, `educational_level`, `language`, `provider_id`, `mask_sensitive`. |
| FR-6 | `POST /api/courses/match` MUST score the request against blocked candidates and return ranked `MatchResult[]`. |
| FR-7 | `POST /api/courses/check-duplicates` MUST run the same blocker + scorer as FR-1 but never write. |
| FR-8 | `POST /api/courses/merge` MUST fold the duplicate into the main course (transfer identifiers, instances, syllabus, links), then soft-delete the duplicate, record a `MergeRecord`. |
| FR-9 | `POST /api/courses/deduplicate` MUST scan the index in batch, queue uncertain matches, and auto-merge above `auto_merge_threshold`. |
| FR-10 | `GET /api/courses/{id}/instances` MUST list all instances ordered by `schedule.start_date DESC NULLS LAST`. |
| FR-11 | `POST /api/courses/{id}/instances` MUST create a new instance, validate FR-26..FR-28. |
| FR-12 | `PUT /api/courses/{id}/instances/{instance_id}` MUST replace the instance. |
| FR-13 | `DELETE /api/courses/{id}/instances/{instance_id}` MUST soft-delete. |
| FR-14 | `GET /api/courses/{id}/audit` MUST return audit log entries (newest first) for the Course AND its child instances/syllabus. |
| FR-15 | `GET /api/courses/{id}/export` MUST return the GDPR Article-15 portability JSON (full record). |
| FR-16 | `GET /api/courses/{id}/masked` MUST return the Course with `provider_id`, `instructor_ids`, `instructor_names`, and any `Personal Data` identifier values masked. |
| FR-17 | Every CRUD operation MUST emit an audit-log entry. |
| FR-18 | Every CRUD operation MUST emit a Course event (`CourseCreated`, `CourseUpdated`, `CourseDeleted`, `CourseMerged`, `CourseInstanceCreated`, `CourseInstanceUpdated`, `CourseInstanceDeleted`) on the streaming bus. |
| FR-19 | Search responses MUST normalise to `{ items, total }` (the front-end expects the `items` key). |
| FR-20 | A deterministic identifier match (DOI, Wikidata, LMS id, OER id, URI, UUID) MUST short-circuit scoring to `1.0` with `confidence = High`. |
| FR-21 | `name` is required, non-empty after trim. |
| FR-22 | `course_code`, when present, MUST be 1-100 chars. |
| FR-23 | `number_of_credits`, when present, MUST be a non-negative integer. |
| FR-24 | `in_language` entries MUST be valid BCP-47 codes (length check; full validation deferred). |
| FR-25 | `url`, `image[*]`, `same_as[*]`, identifier `url`s MUST start with `http://` or `https://`. |
| FR-26 | `CourseInstance.schedule.end_date` MUST be ≥ `schedule.start_date` when both set. |
| FR-27 | `CourseInstance.enrollment_closes` MUST be ≥ `enrollment_opens` when both set. |
| FR-28 | `CourseInstance.maximum_attendee_capacity` MUST be ≥ `enrolled_count` when both set. |

## 7. Non-Functional Requirements

- **Throughput:** ≥1000 req/s sustained on a single 4-core host for `GET /courses/{id}`.
- **Latency p95:** `GET /courses/{id}` ≤ 25 ms; `GET /courses/search` ≤ 100 ms; `POST /courses/match` ≤ 500 ms.
- **Bundle size (binary):** < 30 MB stripped.
- **Memory:** ≤ 256 MB resident for 1M courses + 5M instances indexed.
- **Search consistency:** a `POST /courses` MUST be observable via `GET /courses/search` immediately on subsequent requests (`SearchEngine::reload()` after every commit, matching person-service).

## 8. Architecture

### 8.1 Module layout

```
src/
├── main.rs                  # binary entry — Config → AppState → api::rest::serve
├── api/
│   ├── mod.rs               # ApiResponse, ApiError
│   └── rest/                # REST API (Axum)
├── models/                  # Course, CourseInstance, Provider, identifier, …
├── db/                      # SeaORM entities + repository trait + audit
├── matching/                # service-side adapter onto course_matcher::MatchingEngine
├── search/                  # Tantivy index + query
├── config/                  # env loading + Config struct
└── error.rs
```

### 8.2 Boot sequence

`Config::from_env` → `db::create_connection` → `SearchEngine::new` →
`matching::CourseMatcher::new` → `AppState::new` →
`api::rest::serve`. Identical shape to the
[person-service](../person-service-rust-crate/) binary.

### 8.3 Layering rules

1. `models/` MAY NOT depend on `db/`, `api/`, `search/`.
2. `api/` MAY depend on every other module; nothing depends on `api/`.
3. `matching/` is a thin adapter over the canonical
   [`course-matcher`](../course-matcher-rust-crate/) crate.
4. `search/` MAY depend on `models/` only.

### 8.4 Data flow

**Create:** HTTP POST → validate → duplicate-detection (search +
matcher) → on duplicate return `409 MatchResult[]`; on success
repository INSERT → search index → audit log → event publish →
response.

**Match:** HTTP POST → blocker (`search_by_name_and_provider`) →
load candidates → `CourseMatcher::find_matches` → renormalised
weighted score → response.

**Merge:** HTTP POST → fetch both → fold identifiers / instances /
syllabus / links into main → update main → soft-delete duplicate →
update index → audit log + `CourseMerged` event → response.

## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | endpoints under `/api/courses/*` + `/api/courses/{id}/instances/*` + `/api/audit/*` + `/api/health` |
| gRPC (Tonic) | Out of MVP scope. |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa). |

All REST endpoints return `{ "success": bool, "data": …, "error": … }`.
HTTP status codes follow REST conventions: `409` for duplicate
detection on create, `422` for validation failure, `501` for any
endpoint not yet implemented in the MVP scaffold.

## 10. Persistence

PostgreSQL via SeaORM. Migrations under `migrations/` (numbered SQL
`up.sql` / `down.sql`). Tables:

- `providers` — issuing organisations.
- `courses` — Course template (scalar fields + JSONB collections).
- `course_identifiers` — typed external identifiers.
- `course_links` — course-to-course cross-references.
- `course_instances` — specific offerings (FK to courses).
- `syllabus_sections` — hierarchical (parent_id self-FK).
- `course_match_scores` — historical match scores / review queue.
- `course_merge_records` — merge audit trail with transferred-data snapshot.
- `audit_log` — HIPAA / FERPA-style trail for who / what / when.

## 11. Testing Strategy

| Layer | Tool | Scope |
|---|---|---|
| Unit | `cargo test --lib` | model construction + serde round-trip; matcher adapter; validation rules. |
| Integration | `cargo test --test api_integration_test` | full HTTP cycle against a Postgres test container (docker-compose.test.yml). |
| Bridge | `cargo test --test duplicate_detection` | mirror the pattern used by sibling services: drive `course-matcher::MatchingEngine` through the service-side `Course` and pin scoring behaviour. |
| Benchmarks | `cargo bench` | matching + search + validation (Criterion). |

See [`AGENTS/testing.md`](AGENTS/testing.md) for the full layout.

## 12. Compliance

- **GDPR**: right of access via `GET /courses/{id}/export`; right to
  erasure via soft-delete + masked-view. Audit log records who
  accessed each record.
- **FERPA**: where instances carry instructor or student data, the
  masked view conceals personal identifiers; the audit log preserves
  access trail.
- **WCAG**: out of scope for the service (front-end concern).

## 13. Tasks

- [x] T-1: Scaffold skeleton (Cargo.toml, src/, migrations, Dockerfile, docker-compose, spec, AGENTS docs).
- [x] T-2: SeaORM entity modules in `db/models.rs` matching the migration schema.
- [x] T-3: `SeaOrmCourseRepository` CRUD + soft-delete (courses + identifiers + links round-trip; transactional). Audit-log writes still pending alongside T-9 event publisher.
- [ ] T-4: Tantivy `SearchEngine::index_course` + `search` + `search_by_name_and_provider` (reader-reload after every commit).
- [ ] T-5: Validation module enforcing FR-21..FR-28.
- [ ] T-6: Adapter `matching::adapter::to_matcher_course` + integration with `course_matcher::MatchingEngine`.
- [ ] T-7: REST handlers for FR-1..FR-13 (replacing the `501` stubs).
- [ ] T-8: Instance sub-resource handlers FR-10..FR-13 with transactional create / update / delete.
- [ ] T-9: Audit handlers + event-stream publisher (in-memory MVP; Fluvio adapter under feature flag).
- [ ] T-10: Privacy module (masking + GDPR export) and FR-15 / FR-16.
- [ ] T-11: Bridge test pinning matcher contract + per-field routing.
- [ ] T-12: Integration test exercising create / search / detail / edit / soft-delete / match / merge / dedup / audit.
- [ ] T-13: Benchmark suite (matching, search, validation).
- [ ] T-14: OpenAPI schema completion via utoipa derive annotations.
- [ ] T-15: Authentication middleware (JWT) — coordinated with the family-wide auth rollout.

## 14. Implementation Status

| Area | Status |
|---|---|
| Skeleton (compiles, REST routes return 501) | 🚧 in progress |
| SeaORM entities | ✅ |
| Repository CRUD | ✅ (courses + identifiers + links; instances + syllabus pending T-8) |
| Search engine | ❌ stub |
| Validation | ❌ |
| Matching adapter | ❌ stub |
| REST handlers | ❌ stubs return 501 |
| Audit / streaming | ❌ |
| Privacy | ❌ |
| Tests | ❌ |

## 15. Roadmap

- **v0.2**: T-2..T-7 land; full CRUD + search + matching + dedup wired up.
- **v0.3**: Instance sub-resource (T-8) + audit / streaming (T-9).
- **v0.4**: Privacy + bridge tests + benches (T-10..T-13).
- **v0.5**: JWT auth (T-15) coordinated with family-wide auth rollout.
- **v0.6**: LMS round-trip (LTI / xAPI import / export) — out of MVP, captured here so it stays on the radar.

## 16. Open Questions

- **OQ-1**: Should `CourseInstance` reference the
  [event-service](../event-service-rust-crate/) `Event` resource
  rather than carry its own `schedule` field? Decision deferred until
  we see a real cross-service integration requirement; for MVP we
  keep `Schedule` inline so the front-end works without joining
  services.
- **OQ-2**: Should `Provider` move into a separate
  `organization-service` shared by `course`, `event`, `worker`? The
  Person Service has an inline `Organization` model with the same
  pattern. Defer until two consumers ask for it.
- **OQ-3**: Should the matcher's deterministic-identifier set include
  `CourseCode`? Today: no, because `CS101` exists at many providers.
  Yes for the **same provider** — leaves room for `(provider_id,
  course_code)` short-circuit. To be decided in T-6.
- **OQ-4**: Internationalisation of `EducationalLevel`. The schema.org
  vocabulary doesn't fully cover non-English systems (e.g. UK A-levels,
  German Abitur, French Baccalauréat). The `Custom(String)` escape
  hatch handles them; a controlled vocabulary is a future task.

## 17. References

- Sibling service specs:
  [person-service](../person-service-rust-crate/spec.md),
  [worker-service](../worker-service-rust-crate/spec.md),
  [place-service](../place-service-rust-crate/spec.md),
  [thing-service](../thing-service-rust-crate/spec.md),
  [event-service](../event-service-rust-crate/spec.md).
- Sibling matcher spec: [course-matcher](../course-matcher-rust-crate/spec.md).
- Front-end consumer: [course-front-end-with-svelte](../course-front-end-with-svelte/spec.md) — SvelteKit + SVAR DataGrid + Lily Headless UI over this service's REST API.
- AGENTS reference set: [`AGENTS/index.md`](AGENTS/index.md).
- Shared docs: [`agents/share/index.md`](../agents/share/index.md).
- SDD discipline: [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md).
- External: [schema.org/Course](https://schema.org/Course), [schema.org/CourseInstance](https://schema.org/CourseInstance), [schema.org/LearningResource](https://schema.org/LearningResource), [schema.org/EducationalOccupationalCredential](https://schema.org/EducationalOccupationalCredential).

## 18. Change Control

Material changes to this spec — domain-model fields,
match-quality thresholds, API-surface shape, compliance scope —
MUST land in the same commit as the corresponding code change.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in under a minute. Long-form rationale belongs in
the commit message or a §16 open question.
