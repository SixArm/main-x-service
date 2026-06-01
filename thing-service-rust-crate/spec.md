# Thing Service — Living Specification

> **Source of truth.** This document is the canonical artefact for the
> Thing Service crate. When code and spec disagree, the spec wins —
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

The Thing Service is a **generic registry** for arbitrary discrete
objects — books, papers, software, digital assets, devices, products,
instances of any physical or virtual object. The domain model is
faithful to [schema.org/Thing](https://schema.org/Thing) and is the
most general entity in the Main X Index family: anything that does not
fit one of the more opinionated sibling crates (`person`, `worker`,
`event`, `place`) belongs here.

### 1.2 Vision

A stable identity for any "thing" with:

- Typed identifiers (DOI, ISBN, ISSN, GTIN, SKU, MPN, SerialNumber,
  URI, UUID, Custom) drawn from
  [schema.org/PropertyValue](https://schema.org/PropertyValue).
- Probabilistic + deterministic matching by name, identifier,
  description, URL, and `sameAs` cross-reference.
- Real-time and batch duplicate detection with auto-merge for
  high-confidence cases.
- Stable cross-system Thing IDs so downstream systems converge on a
  single ID per real-world object.
- Audit logs and an event stream covering every CRUD / merge / link.

### 1.3 Non-goals

- **Not** an inventory system — we record identity, not stock or location.
- **Not** a catalogue manager — we hold canonical properties, not
  marketing copy or pricing.
- **Not** a recommendation engine — `same_as` and `additional_type`
  give downstream systems the hooks they need.

## 2. Scope

### 2.1 In scope

- Thing identity CRUD with soft delete and full audit trail.
- schema.org/Thing canonical properties (`name`, `alternateName`,
  `description`, `disambiguatingDescription`, `additionalType`, `url`,
  `identifier`, `image`, `mainEntityOfPage`, `owner`, `sameAs`,
  `subjectOf`, `potentialAction`).
- Typed identifiers via `PropertyValue` shape.
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy + boolean search.
- Real-time + batch duplicate detection with review queue +
  auto-merge.
- Record merging with link tracking and JSON snapshots.
- Per-field privacy masking, GDPR Article 15 export, consent records.
- REST API (Axum) + gRPC stub.
- Server-rendered web UI.
- PostgreSQL persistence via SeaORM.

### 2.2 Out of scope (today)

- FHIR R5 — Things are not a FHIR-resource concern.
- Production Fluvio publisher / consumers.
- ML-based match scoring.
- File / blob storage for image URLs (`image[]` holds URLs, not bytes).

## 3. Stakeholders and Users

| Stakeholder | Interest |
|---|---|
| Catalogue / metadata teams | Stable identifiers + dedup |
| Open-data integrators | DOI / ISBN / GTIN deterministic match |
| Operations / DBA | PostgreSQL schema + migration discipline |
| Compliance officer | GDPR export + consent records (for personal Things) |
| Other Main X Index crates | Cross-references via `thing_id` |

## 4. Glossary

| Term | Meaning |
|---|---|
| **Thing** | A discrete object — book, paper, software, device, product, asset |
| **Deterministic identifier** | DOI / ISBN / ISSN / GTIN / MPN / SerialNumber / UUID — globally unique by construction; match short-circuits to 1.0 |
| **Non-deterministic identifier** | SKU / URI / Custom — used as evidence, not as a hard pin |
| **PropertyValue** | The schema.org shape `{ propertyID, value, name?, url? }` |
| **Match quality** | Certain / Probable / Possible / Unlikely buckets keyed off configurable thresholds |
| **Soft delete** | `is_deleted = true`; rows are never `DELETE`d |

## 5. Domain Model

Field-by-field reference: [`AGENTS/models.md`](AGENTS/models.md).

### 5.1 `Thing`

Material aspects:

- **Schema.org/Thing properties** — `name` (required),
  `alternate_names`, `description`, `disambiguating_description`,
  `additional_type` (URL of a schema.org sub-type),
  `url`, `images`, `main_entity_of_page`, `owner`, `same_as`,
  `subject_of`, `potential_action`.
- **Identifiers** — `Vec<ThingIdentifier>` (the `PropertyValue` shape).
- **Registry-internal** — UUID `id`, `created_at`, `updated_at`,
  `is_deleted`, `deleted_at`.

### 5.2 `ThingIdentifier`

`{ property_id: IdentifierType, value: String, name?: String, url?: String }`

`IdentifierType` variants:

- **Deterministic** (globally unique): `Doi`, `Isbn`, `Issn`, `Gtin`,
  `Mpn`, `SerialNumber`, `Uuid`.
- **Non-deterministic**: `Sku`, `Uri`, `Custom(String)`.

### 5.3 Supporting types

`MergeRequest` / `MergeResponse` / `MergeRecord`, `ReviewQueueItem`,
`BatchDeduplicationRequest` / `Response`, `Consent` (for Things
subject to data-protection regimes — e.g. a personally-owned record).

### 5.4 Invariants

The implementation MUST enforce:

- `name` is non-empty.
- An `Identifier` is keyed by `(property_id, value)`; duplicates within
  a single record are silently deduplicated.
- All URL-valued properties (`url`, `additional_type`,
  `main_entity_of_page`, `subject_of`, each `image`, each `same_as`)
  MUST use the `http://` or `https://` scheme.
- `Isbn` is 10 or 13 digits (dashes / spaces tolerated; trailing `X`
  allowed for ISBN-10).
- `Issn` is 8 chars (trailing `X` allowed).
- `Doi` MUST start with `10.` and contain `/`.
- `Gtin` is 8 / 12 / 13 / 14 digits.
- `Uuid` MUST parse per RFC 4122.
- `Uri` MUST contain a scheme separator (`:`).
- Soft delete is the only delete.

## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete Thing records.
- Multiple typed identifiers per Thing (`PropertyValue` list).
- Multiple `alternate_names`, `images`, `same_as` URLs.
- Automatic event publish on every CRUD.

### 6.2 Matching

Algorithm reference: [`AGENTS/matching.md`](AGENTS/matching.md).

Default component weights (sum to 1.0):

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.40 | Jaro-Winkler (case-insensitive) |
| Identifier | 0.30 | Exact `(property_id, value)` |
| Description | 0.10 | Jaro-Winkler |
| URL | 0.10 | Scheme/case-normalized host + path |
| Same-as | 0.10 | Best URL pair across `sameAs` lists |

Deterministic short-circuit: any matching DOI / ISBN / ISSN / GTIN /
MPN / SerialNumber / UUID → 1.0.

Phonetic bonus: +0.05 when name Soundex matches and base score < 0.95.

Match quality (configurable thresholds):

| Quality | Score |
|---|---|
| Certain | ≥ 0.95 |
| Probable | ≥ 0.80 |
| Possible | ≥ 0.60 |
| Unlikely | < 0.60 |

#### Interoperability with `thing-matcher`

The service embeds the sibling `thing-matcher` crate (declared in
`Cargo.toml`) and re-exports it from `src/matching/mod.rs` as
`matcher_lib`. The matcher crate is the **canonical reference
algorithm** — it scores 10 schema.org/Thing components (the service
scores 5), exposes 12 tunable config knobs including three presets
(`strict` / `default` / `lenient`), and uses an opaque `(property_id,
value)` identifier shape that accepts any vocabulary the service can
emit.

Bridge: [`src/matching/adapter.rs`](src/matching/adapter.rs) exposes
`to_matcher_thing(&service::Thing) -> thing_matcher::Thing`. The
projection lifts the service's `Thing` (schema.org-shaped with the
`PropertyValue` identifier wrapper) into the matcher's builder:

- `name`, `description`, `disambiguating_description`, `url`,
  `main_entity_of_page`, `owner` map 1:1
- `alternate_names: Vec<String>` → `alternate_names`
- `additional_type` (singular) → first entry of `additional_types`
- `subject_of` (singular) → first entry of `subject_of`
- `images: Vec<String>` → first entry of `image` (matcher takes one)
- `same_as: Vec<String>` → `same_as`
- `identifiers[]` mapped via `map_identifier_property`: schema.org
  canonical tokens (`doi`, `isbn`, `issn`, `gtin`, `sku`, `mpn`,
  `serialNumber`, `uri`, `uuid`); `Custom(s)` passes the carried
  label through verbatim. Identifier `name` / `url` metadata is
  dropped (the matcher does not consume it).

Registry-only fields (`id`, `is_deleted`, `created_at`,
`updated_at`, `potential_action`) are dropped — they have no
matcher counterpart. See [`AGENTS/matching.md`](AGENTS/matching.md)
for the in-service algorithm and the matcher crate's
[`spec.md §5–§6`](../thing-matcher-rust-crate/spec.md) for the
canonical algorithm.

### 6.3 Search

Tantivy across `name`, `alternate_names`, `description`,
`identifier.value`, `url`, `same_as`. Full-text + fuzzy + boolean.
Pagination (`offset` + `limit`). Optional masking for Things subject
to consent constraints.

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/things` when an existing
  Thing matches on a deterministic identifier or on name + URL.
- Explicit `POST /api/things/check-duplicates`.
- Batch `POST /api/things/deduplicate`.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge transfers identifiers, `alternate_names`, `same_as`,
  `images`; appends the duplicate's name as `alternate_name` on the
  survivor; adds a `Replaces` link; soft-deletes the duplicate;
  records a JSON snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

Required `name`; URL formats on every URL-valued property; identifier
formats per type (see §5.4). Normalisation trims text, lowercases URL
schemes (host / path preserved), dedupes `alternate_names`,
`same_as`, `images`. Failed validation → `422`.

### 6.6 Privacy

Per-field masking: `owner` → `"[owner withheld]"`; identifier `value`
→ `"****<last 4 chars>"`; per-identifier `url` cleared on mask;
`property_id` preserved. GDPR Article 15 export at
`GET /api/things/{id}/export`. Consent model when a Thing is attached
to a person.

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp.

## 7. Non-Functional Requirements

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

## 8. Architecture

### 8.1 Module layout

```
src/
├── lib.rs                 # Library root
├── models/
│   ├── thing.rs           # Thing
│   ├── identifier.rs      # ThingIdentifier + IdentifierType
│   └── consent.rs         # Consent
├── matching/
│   ├── name.rs            # Jaro-Winkler
│   ├── description.rs     # Jaro-Winkler
│   ├── url.rs             # scheme/case-normalized comparison
│   ├── identifier.rs      # PropertyValue exact match + deterministic detection
│   ├── phonetic.rs        # Soundex
│   └── scoring.rs         # compute_match, MatchWeights, MatchConfidence
├── validation/            # boundary validators + normalisers
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

A `ThingMatcher` trait is an open question (OQ-2).

## 9. API Surface

Complete reference: [`AGENTS/restful.md`](AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/things/*` + `/api/audit/*` + `/api/health` |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../spec.md) |
| Docs | Swagger UI at `/swagger-ui` |

This crate does **not** expose a FHIR R5 surface — Things are not a
FHIR-resource concern.

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

## 10. Persistence

PostgreSQL 18+ via SeaORM.

### 10.1 Tables

`things`, `thing_identifiers`, `thing_alternate_names`,
`thing_images`, `thing_same_as`, `thing_links`, `thing_match_scores`,
`audit_log`.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`.

## 11. Testing Strategy

Layered: [`AGENTS/testing.md`](AGENTS/testing.md).

- **Unit tests** — `#[cfg(test)]` modules; models, matching,
  validation, privacy. ~100 tests.
- **Integration tests** — `tests/integration_*.rs`; end-to-end
  workflows.
- **Benchmarks** — Criterion: matching, search, validation, privacy.
- **CI** — `test.yml`, `quality.yml`, `security.yml`.

## 12. Compliance

| Standard | Mechanism |
|---|---|
| GDPR Art. 15 | `GET /api/things/{id}/export` (for personal Things) |
| GDPR Art. 17 | Soft delete + consent revocation |
| ISO/IEC 27001 | Operational controls (deployment-side) |

Technology compliance:
[`agents/share/compliance-for-technology.md`](../agents/share/compliance-for-technology.md).

## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [ ] **T-1 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag.
  - **Acceptance:** integration test publishes a `ThingCreated`
    record end-to-end.
- [ ] **T-2 — Introduce `ThingMatcher` trait.**
  - [ ] Promote `compute_match` to a trait so alternative scorers
    (ML-based, embedding-based) can plug in.
  - **Acceptance:** `ProbabilisticMatcher : ThingMatcher` compiles
    and behaves identically to today's free function.
- [ ] **T-3 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `ThingService.GetThing`
    round-trips a record.
- [ ] **T-4 — Authentication / authorisation.**
  - [ ] JWT middleware on `/api/*` with editor / read-only / service
    roles.
  - **Acceptance:** unauthenticated requests get `401`; valid token
    + role gets `2xx`.
- [ ] **T-5 — Embedding-based similarity (optional / experimental).**
  - [ ] Vector index via `pg_vector`.
  - [ ] `compute_match` augmented with cosine-similarity score.
  - **Acceptance:** A/B harness shows ≥ 2 % uplift on a labelled
    duplicate set.
- [ ] **T-6 — Spec-drift CI guard.**
  - [ ] Fail PR if `src/matching/**` or `src/models/thing.rs`
    changes without a `spec.md` edit.
  - **Acceptance:** `bash scripts/spec-drift-check.sh main HEAD`
    exits non-zero on a code-only PR.

## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Domain model | schema.org/Thing canonical properties + PropertyValue identifiers |
| Matching | Probabilistic (name / identifier / description / URL / sameAs) + deterministic (DOI / ISBN / ISSN / GTIN / MPN / SerialNumber / UUID short-circuit) + Soundex bonus |
| Search | Tantivy index on name / alternate_names / description / identifier value / URL / same_as |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| Validation | Required `name`, URL formats, per-type identifier formats, normalisation |
| Privacy | Per-field masking (`owner`, identifier `value`), GDPR export, consent model |
| Web UI | Loco / Tera / HTMX / Alpine / Lily HTML Headless + NHS UK theme |
| Tests | ~100 unit + integration_* + Criterion benchmarks |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| Fluvio production publisher | T-1 |
| `Matcher` trait abstraction | T-2 |
| gRPC API | T-3 |
| Authentication / authorisation | T-4 |
| Embedding-based similarity | T-5 |
| Spec-drift CI guard | T-6 |

## 15. Roadmap

- **Authentication & authorisation** — JWT, RBAC, rate limiting,
  user endpoints, security headers.
- **Observability** — Prometheus alongside OTLP, complete OTLP trace
  exporter, custom metrics (`thing_created`, `match_score_histogram`),
  Grafana dashboards + alerting.
- **Performance** — query caching, batch fixes, profile matching hot
  paths.
- **Infrastructure as code** — OpenTofu modules, multi-cloud, secrets,
  backup + DR.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index,
  ingress, probes.
- **Production readiness** — security audit, GDPR validation, DR
  runbook, backup / restore, CI/CD pipeline.
- **Feature enhancements** — complete gRPC, Fluvio production +
  consumers, ML / embedding-based match scoring, image storage and
  retrieval, schema.org sub-type registry (`additional_type` lookup),
  Wikidata + OpenLibrary import pipelines.

## 16. Open Questions

- **OQ-1 — Image storage.** Today `image[]` is URLs only. Should the
  service offer a blob store endpoint, or hand off to a separate
  asset service?
- **OQ-2 — Matcher trait abstraction.** Promote `compute_match` to a
  `ThingMatcher` trait now (paving for ML), or defer until T-5 actually
  needs it?
- **OQ-3 — `additional_type` validation.** Should we reject values
  outside a curated allowlist (schema.org sub-types only), or accept
  any URL and warn?

## 17. References

- Sibling specs: [person-service](../person-service-rust-crate/spec.md),
  [event-service](../event-service-rust-crate/spec.md),
  [worker-service](../worker-service-rust-crate/spec.md),
  [place-service](../place-service-rust-crate/spec.md).
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
Thing Service.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation.
