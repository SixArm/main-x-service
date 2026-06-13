## 5. Domain Model

The course entity has **one canonical domain model and three
representations**. The service's Rust model is canonical; the matcher
and front-end representations are projections of it.

### 5.1 Canonical `Course` + `CourseInstance` (service)

Defined in the service crate (`src/models/`); field-by-field
schema.org → Rust reference in
[`course-service-rust-crate/AGENTS/models.md`](../course-service-rust-crate/AGENTS/models.md).
Material aspects:

- **Course (the template)** — Thing + CreativeWork + LearningResource
  + Course-specific properties: `course_code`, `number_of_credits`,
  `course_prerequisites`, `available_language`,
  `financial_aid_eligible`, `educational_credential_awarded`,
  `total_historical_enrollment`, `syllabus_sections`, `instances`.
- **CourseInstance** — schedule, mode (online / onsite / blended /
  self-paced), instructors, location, capacity, enrollment window.
  Sub-resource: lives under its parent course, never standalone.
- **Provider** — the issuing organisation.
- **CourseIdentifier** — `{ property_id, value, name?, url? }`
  matching `schema.org/PropertyValue`; `property_id` enumerates the
  scheme; `is_deterministic()` exposes which schemes short-circuit.
- **Syllabus** — hierarchical table-of-contents node with `teaches`,
  `time_required`, and `sub_sections`.
- **EducationalCredential** — `schema.org/EducationalOccupationalCredential`.
- **MergeRequest / MergeResponse / ReviewQueueItem** — same shape as
  the sibling services.

### 5.2 Matcher `Course` (slim identity-signal shape)

Defined in the matcher crate
([spec §6](../course-matcher-rust-crate/spec/06-domain-model.md)):
`Course { name, alternate_names, course_code, provider_id,
provider_name, educational_level, learning_resource_type, keywords,
teaches, identifiers, same_as, in_language }`, with
`CourseIdentifier { scheme, value }`, `IdentifierScheme` (12
variants), `EducationalLevel` (12 variants + `Custom`), and
`LearningResourceType` (11 variants + `Custom`). The matcher models
**only** the properties that carry identity signal.

### 5.3 Service ↔ matcher DTO contract (the adapter)

The service embeds the matcher (path dependency) and bridges via
[`src/matching/adapter.rs`](../course-service-rust-crate/src/matching/adapter.rs):
`to_matcher_course(&service::Course) -> course_matcher::Course`,
driven by `CourseMatcher` over `course_matcher::MatchingEngine`.

Routing rules (normative; pinned by
[`tests/duplicate_detection.rs`](../course-service-rust-crate/tests/duplicate_detection.rs),
14 tests):

- `Course.name` → matcher `name`; `alternate_names` → `alternate_names`.
- `course_code` + `provider_id` → the matcher's provider-scoped
  course-code field (never scored across providers).
- `identifiers[]` → matcher `Vec<CourseIdentifier>`, with
  `property_id` mapped 1:1 to the matcher's `IdentifierScheme` enum
  (the matcher mirrors but does not re-derive the deterministic set).
- `same_as[]` → matcher `Vec<String>` of authoritative URLs (R-2
  short-circuit input).
- `educational_level` + `learning_resource_type` → matcher enums,
  routed 1:1.
- `keywords` / `teaches` / `assesses` → matcher set fields (Jaccard).

The projection is **lossy by design**: registry-only fields (`id`,
`instances`, `syllabus_sections`, timestamps, audit plumbing, …) are
dropped — they carry no identity signal. The adapter is the pinch
point for any field-name drift between the two crates.

### 5.4 Front-end TypeScript types

The front-end mirrors the service's wire format in
`src/lib/api/types.ts` (`Course`, `MatchResult`, …) and unwraps the
shared envelope in `src/lib/api/client.ts`; `CourseRepository`
(`src/lib/api/courses.ts`) assumes the `/api` base path. The service
model is upstream: if a field changes in the service, the front-end
types MUST be fixed in the same change cycle (front-end
[`AGENTS.md`](../course-front-end-with-svelte/AGENTS.md)).

### 5.5 Shared invariants

All subprojects MUST uphold:

- `name` is required and non-empty after trim (service FR-21).
- Course codes are **provider-scoped**: no subproject may treat
  `course_code` equality across providers as an identity signal.
- The deterministic identifier set is exactly DOI / Wikidata / LOM /
  OER / URI / UUID (matcher [spec §15](../course-matcher-rust-crate/spec/15-identifier-short-circuits.md));
  adding a scheme requires a matcher spec edit **and** a service
  bridge test in the same change.
- Soft delete (`deleted_at`) is the only delete, end to end: the
  service never row-deletes, and the front-end never offers hard
  delete.
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown.
- Search responses normalise to `{ items, total }` (service FR-19;
  the front-end depends on the `items` key).
