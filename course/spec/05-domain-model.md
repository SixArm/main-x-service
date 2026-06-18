## 5. Domain Model

The course entity has **one canonical domain model and three
representations**. The service's Rust model is canonical; the matcher
and front-end representations are projections of it.

### 5.1 Canonical `Course` + `CourseInstance` (service)

Defined in the service crate (`src/models/`); field-by-field
schema.org → Rust reference in
[`course-service-with-loco/AGENTS/models.md`](../course-service-with-loco/AGENTS/models.md).
Material aspects:

- **Course (the template)** — Thing + CreativeWork + LearningResource
  + Course-specific properties: `course_code`, `number_of_credits`,
  `course_prerequisites`, `available_language`,
  `financial_aid_eligible`, `educational_credential_awarded`,
  `total_historical_enrollment`, `syllabus_sections`, `instances`,
  `lessons`.
- **Tags** — `tags: Vec<String>`, a list of **short free-text labels**
  that operators attach to a record for grouping, filtering, triage, or
  workflow (e.g. `"vip"`, `"review"`, `"archived-2026"`,
  `"fast-track"`). **Any `Course` (the main concept) can carry tags.**
  Each tag is a short, trimmed, non-empty string; the list is
  **unordered**, **de-duplicated case-insensitively**, and **defaults to
  empty**. Distinct from `keywords`: `keywords` are descriptive /
  discovery terms about *what the record is* (the subject matter), while
  **tags are user-applied operational labels** for grouping and
  workflow — neither removes nor renames the other. Tags **are** a
  supporting match signal: the matcher scores them by plain set Jaccard
  over the case-insensitively normalised tag sets (matcher §5.2 / §13a),
  weighted `tags_weight` — a corroborating signal, never an identifying
  field on its own. As a canonical (upstream) field, tags propagate to
  the service model, matcher DTO, and front-end types in the same change
  cycle (§5.1–§5.4).
- **CourseInstance** — schedule, mode (online / onsite / blended /
  self-paced), instructors, location, capacity, enrollment window.
  Sub-resource: lives under its parent course, never standalone.
- **Lesson** — a unit of teaching / content **within a course**
  (schema.org `LearningResource`; the course `hasPart` the lesson). One
  course contains **0..many** lessons, ordered by `position`. Carries e.g.
  `title`, `position`, `description`, `teaches`, `time_required`.
  Sub-resource: lives under its parent course, never standalone. Distinct
  from a **CourseInstance** (a *scheduled offering* — when/where/who) and
  from the **Syllabus** (the table-of-contents that may organise the
  lessons).
- **Relationships** — typed course-to-course links:
  `relationships: Vec<CourseRelationship>`, each `{ relation, course_id }`
  **referencing another `Course` in the registry**. `relation` is a
  `RelationKind` enum, initially **`SimilarTo`**, **`HigherLevelThan`**,
  and **`LowerLevelThan`**:
  - `SimilarTo` is **symmetric** — A `SimilarTo` B ⇔ B `SimilarTo` A
    (the two courses cover comparable material).
  - `HigherLevelThan` / `LowerLevelThan` are **inverses** — A
    `HigherLevelThan` B (A is *more abstract* / advanced) ⇔ B
    `LowerLevelThan` A (B is *more concrete* / foundational).
  These typed links are distinct from the existing
  `course_prerequisites` field (free-form prerequisite text/refs, left
  unchanged). The enum is extensible (e.g. a future
  `PrerequisiteOf` / `HasPrerequisite` inverse pair that would formalise
  `course_prerequisites` as typed registry references).
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
teaches, identifiers, same_as, in_language, relationships, tags }`, with
`CourseIdentifier { scheme, value }`, `IdentifierScheme` (12
variants), `EducationalLevel` (12 variants + `Custom`),
`LearningResourceType` (11 variants + `Custom`), and
`RelationshipRef { relation, course_id }` over a `RelationKind` enum. The matcher models
**only** the properties that carry identity signal.

### 5.3 Service ↔ matcher DTO contract (the adapter)

The service embeds the matcher (path dependency) and bridges via
[`src/matching/adapter.rs`](../course-service-with-loco/src/matching/adapter.rs):
`to_matcher_course(&service::Course) -> course_matcher::Course`,
driven by `CourseMatcher` over `course_matcher::MatchingEngine`.

Routing rules (normative; pinned by
[`tests/duplicate_detection.rs`](../course-service-with-loco/tests/duplicate_detection.rs),
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
- `tags` → matcher `tags`; routed 1:1, **not dropped**. Scored by plain
  set Jaccard over the case-insensitively normalised tag sets (matcher
  §5.2 / §13a), weighted `tags_weight` — a supporting signal, not an
  identifying field on its own.
- `relationships[]` → matcher `relationships` (typed `(relation,
  course_id)` refs); routed 1:1, **not dropped**. Scored by typed-set
  Jaccard (matcher §13a), weighted `relationships_weight` — a supporting
  signal, not an identifying field on its own. (`course_prerequisites`
  stays registry-only, dropped — it is free-form prerequisite content,
  not a typed course reference today.)

The projection is **lossy by design**: registry-only fields (`id`,
`instances`, `syllabus_sections`, `lessons`, `course_prerequisites`,
timestamps, audit plumbing, …)
are dropped — they carry no identity signal. (`relationships` and
`tags` are the exceptions — each carries a supporting identity signal
and is routed, not dropped.) The adapter is the pinch
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
- A `CourseRelationship` references an **existing** `Course` in the
  registry; **no course relates to itself** (not its own
  similar / higher-level / lower-level course). `HigherLevelThan` /
  `LowerLevelThan` must stay **acyclic** (no course is, directly or
  transitively, of a higher level than itself) and, where both
  directions are stored, mutually consistent (A `HigherLevelThan` B ⇔ B
  `LowerLevelThan` A); `SimilarTo` is **symmetric** (A `SimilarTo` B ⇔ B
  `SimilarTo` A).
- A **lesson** belongs to exactly one parent course (a sub-resource,
  never standalone); a course contains **0..many** ordered lessons.
  Lessons are registry-only content (no identity signal) — the matcher
  drops them, like `instances` and `syllabus_sections`.
- `tags` are **short, trimmed, non-empty** strings, **unordered**, and
  **de-duplicated case-insensitively**; they are operational labels, not
  the descriptive `keywords` field, and are a **supporting match
  signal** — scored by plain set Jaccard in the matcher (§5.2 / §13a),
  weighted `tags_weight`, never identifying on their own.
- Soft delete (`deleted_at`) is the only delete, end to end: the
  service never row-deletes, and the front-end never offers hard
  delete.
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown.
- Search responses normalise to `{ items, total }` (service FR-19;
  the front-end depends on the `items` key).
