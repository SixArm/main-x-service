## 6. Functional Requirements

Entity-level requirements, each mapped to its owning subproject.
Detail lives in the owner's spec: the entry here is the contract that
the trio composes correctly. Owners: **S** = course-service, **M** =
course-matcher, **F** = course-front-end.

| ID | Requirement | Owner | Detail |
|---|---|---|---|
| FR-1 | Course CRUD with soft delete and full audit trail | S | [service FR-1..FR-4](../course-service-rust-crate/spec/06-functional-requirements.md) |
| FR-2 | `CourseInstance` sub-resource CRUD under `/api/courses/{id}/instances/*`, ordered `schedule.start_date DESC NULLS LAST` | S | [service FR-10..FR-13](../course-service-rust-crate/spec/06-functional-requirements.md) |
| FR-3 | Multiple identifiers per course (`PropertyValue`-shaped: DOI, Wikidata, LMS id, OER id, ISCED, ROR, platform slug, URI, UUID, custom) | S | [service §2](../course-service-rust-crate/spec/02-scope.md) |
| FR-4 | Probabilistic matching — weighted fuzzy scoring with per-component breakdown: name (Jaro-Winkler + Levenshtein + Soundex bonus, weight 0.35), provider-scoped course code (0.15), provider (0.15), educational level (0.10), keywords Jaccard (0.10), teaches Jaccard (0.15) | S + M | [matcher §5](../course-matcher-rust-crate/spec/05-algorithm-overview.md), [§7](../course-matcher-rust-crate/spec/07-configuration.md) |
| FR-5 | Deterministic matching — short-circuit to score 1.0 on DOI / Wikidata / LOM / OER / URI / UUID identifier match, same-provider course code (R-1), or `same_as` URL overlap (R-2) | S + M | [matcher §15](../course-matcher-rust-crate/spec/15-identifier-short-circuits.md), [§16](../course-matcher-rust-crate/spec/16-same-as-url-short-circuit.md) |
| FR-6 | The service MUST expose the matcher's canonical algorithm through the adapter (§5.3); routing rules are normative and test-pinned | S | [adapter.rs](../course-service-rust-crate/src/matching/adapter.rs) |
| FR-7 | Full-text + fuzzy search (Tantivy) over name, alternate names, keywords, teaches, identifier values; filters for `educational_level`, `language`, `provider_id`; pagination | S | [service FR-5](../course-service-rust-crate/spec/06-functional-requirements.md) |
| FR-8 | Duplicate detection: real-time `409` + `MatchResult[]` on create, explicit `check-duplicates`, batch `deduplicate` with auto-merge above `auto_merge_threshold` | S | [service FR-1, FR-7, FR-9](../course-service-rust-crate/spec/06-functional-requirements.md) |
| FR-9 | Merge: fold identifiers / instances / syllabus / links into the main course, soft-delete the duplicate, record a `MergeRecord` snapshot | S | [service FR-8](../course-service-rust-crate/spec/06-functional-requirements.md) |
| FR-10 | Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`) persisted in `course_match_scores` | S | [service §10](../course-service-rust-crate/spec/10-persistence.md) |
| FR-11 | Validation at the boundary (name required, course-code length, credits non-negative, BCP-47 languages, URL shapes, instance schedule / enrollment-window / capacity ordering; `422` on failure) | S | [service FR-21..FR-28](../course-service-rust-crate/spec/06-functional-requirements.md) |
| FR-12 | Privacy: masked view (provider / instructor fields) and GDPR Article 15 export per course | S | [service FR-15, FR-16](../course-service-rust-crate/spec/06-functional-requirements.md) |
| FR-13 | Audit: every CRUD on Course **and** its child instances / syllabus writes an audit-log entry; per-course + recent queries | S | [service FR-14, FR-17](../course-service-rust-crate/spec/06-functional-requirements.md) |
| FR-14 | Event streaming: publish on every CRUD / merge (`CourseCreated`, `CourseUpdated`, `CourseDeleted`, `CourseMerged`, `CourseInstance*`) | S | [service FR-18](../course-service-rust-crate/spec/06-functional-requirements.md) |
| FR-15 | Operator UI: list/search grid, create with inline 409-duplicate candidates, detail, edit, soft delete | F | [front-end §6](../course-front-end-with-svelte/spec/06-functional-requirements.md) |
| FR-16 | Operator UI: match check (score a hypothetical record), merge with preview, per-course audit view | F | [front-end §6](../course-front-end-with-svelte/spec/06-functional-requirements.md) |

### 6.1 Composition requirements

- **FR-17** — The front-end MUST consume only the service's public
  REST API (`/api/*`), never the database, search index, or matcher
  directly.
- **FR-18** — The matcher MUST remain a pure library (no IO, no
  async runtime, deterministic); the service is its only in-entity
  embedder.
- **FR-19** — Duplicate candidates returned in a `409` create
  response MUST render inline in the front-end create flow, so the
  operator can divert to match/merge without losing input.
- **FR-20** — A new deterministic identifier scheme lands in three
  parts in one change cycle: matcher spec + code, service bridge
  test, and (where exposed) front-end identifier-scheme option.
- **FR-21** — Confidence classification is family-conventional:
  Definite ≥ 0.95, Probable ≥ threshold (default 0.85), Possible
  ≥ 0.50, Unlikely below (matcher
  [spec §18](../course-matcher-rust-crate/spec/18-confidence-classification.md)).
