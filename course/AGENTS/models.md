# Domain models — Course Entity

Orientation only. The full property tables live with their owners:

- Service field-by-field schema.org → Rust mapping:
  [`course-service/AGENTS/models.md`](../course-service-rust-crate/AGENTS/models.md)
- Matcher slim shape: [matcher spec §6](../course-matcher-rust-crate/spec/06-domain-model.md)
- Front-end mirror: `course-front-end/src/lib/api/types.ts`

## One model, three representations

| Representation | Shape | Purpose |
|---|---|---|
| **Service `Course`** (canonical) | Full schema.org/Course: Thing + CreativeWork + LearningResource + Course properties, plus `instances`, `syllabus_sections`, `identifiers`, `educational_credential_awarded` | What the REST API serves and PostgreSQL stores |
| **Matcher `Course`** (projection) | Slim identity-signal shape: `name`, `alternate_names`, `course_code` + `provider_id`/`provider_name`, `educational_level`, `learning_resource_type`, `keywords`, `teaches`, `identifiers`, `same_as`, `in_language` | What `MatchingEngine` scores |
| **Front-end `Course`** (mirror) | TypeScript copy of the service wire format | What the operator UI renders and posts |

## Template vs instance

- `Course` is the durable **template** ("CS101 Introduction to
  Computer Science").
- `CourseInstance` is a specific **offering** (term, schedule, mode,
  instructors, location, capacity, enrollment window) — always a
  sub-resource of its course, served under
  `/api/courses/{id}/instances/*`.
- Supporting types: `Provider`, `CourseIdentifier`
  (`PropertyValue`-shaped, with `is_deterministic()`), `Syllabus`
  (hierarchical), `EducationalCredential`, merge / review-queue types.

## The adapter (the contract that matters)

[`course-service/src/matching/adapter.rs`](../course-service-rust-crate/src/matching/adapter.rs)
projects the canonical model down to the matcher shape
(`to_matcher_course`). The projection is lossy by design (registry
plumbing dropped); the routing rules are normative in entity spec
[§5.3](../spec/05-domain-model.md) and pinned by the bridge tests.
Any field rename in either crate surfaces here first.

## Shared invariants (entity spec §5.5, abridged)

- `name` required, non-empty after trim.
- Course codes are provider-scoped — never an identity signal across
  providers.
- Deterministic identifier set is exactly DOI / Wikidata / LOM / OER
  / URI / UUID.
- Soft delete only, end to end.
- Scores in `[0.00, 1.00]`, always with a per-component breakdown.
- Search responses are `{ items, total }`.
