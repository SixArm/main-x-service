# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- **SeaORM entities** (T-2). One module per migration table in
  `src/db/models.rs`: `providers`, `courses`, `course_identifiers`,
  `course_links`, `course_instances`, `syllabus_sections`,
  `audit_log`, `course_match_scores`, `course_merge_records`. JSONB
  columns typed as `serde_json::Value` and rehydrated by the
  repository.
- **`SeaOrmCourseRepository` CRUD** (T-3). `create` / `get_by_id` /
  `update` / `soft_delete` / `list` round-trip courses + identifiers
  + links transactionally. Status / link-type / interactivity-type
  enums map to lowercase / kebab-case strings via a small
  serde-backed helper; collection fields ride on JSONB columns.
  `instances` + `syllabus_sections` deferred to T-8 (sub-resource).
- **Tantivy `SearchEngine`** (T-4). `CourseIndex` + `CourseIndexSchema`
  in `src/search/index.rs` carry `id` (STRING / stored) and TEXT
  fields for `name`, `alternate_names`, `course_code` (STRING),
  `provider_id` (STRING), `provider_name`, `keywords`, `teaches`,
  `identifiers`. `SearchEngine::index_course` / `search` /
  `fuzzy_search` / `search_by_name_and_provider` / `delete_course`
  follow the family pattern (reader reload after every commit;
  multi-token fuzzy via `BooleanQuery` of `FuzzyTermQuery` per
  alphanumeric run).
- **Matching adapter** (T-6). `matching::adapter::to_matcher_course`
  projects the rich service `Course` down to the slim
  `course_matcher::Course` shape, with 1:1 routing of
  `IdentifierType → IdentifierScheme`, `EducationalLevel`, and
  `LearningResourceType`. `CourseMatcher::match_courses` and
  `find_matches` now drive `course_matcher::MatchingEngine` for real,
  no longer stubbed.
- **Validation module** (T-5). `src/validation/` enforces FR-21..FR-28:
  required non-blank `name`, `course_code` 1..=100 chars, sane
  credits cap, plausible BCP-47 codes on `in_language` and
  `available_language`, `http(s)://` scheme check on every URL field
  (course `url`, `image[*]`, `same_as[*]`, identifier `url`),
  `schedule.end_date ≥ start_date`, ordered enrollment window, and
  `maximum_attendee_capacity ≥ enrolled_count`. Nested-instance
  errors carry an `instances[i].` path prefix so the `422` body
  points the caller at the exact field.
- **REST handlers FR-1..FR-5 + FR-7** (T-7, partial). `POST /api/courses`
  validates → blocks via `search_by_name_and_provider` → scores
  candidates → returns `201` + `Course` on success, `409` +
  ranked `ScoredCandidate[]` on duplicate, `422` + field errors on
  validation failure. `GET /api/courses/{id}`, `PUT`, `DELETE`
  (soft-delete) wired against the SeaORM repository. `GET
  /api/courses/search` runs `search` or `fuzzy_search` (per
  `?fuzzy=true`), falls back to `list` for empty queries, returns
  the `{items, total}` envelope FR-19 mandates. `POST
  /api/courses/check-duplicates` runs the same blocker + scorer as
  the create handler without writing. `ApiResponse::error_with_details`
  surfaces validation errors and ranked candidates under `details`.
  FR-6 (match-against-existing), FR-8 (merge), FR-9 (batch dedup),
  FR-14..FR-16 (audit / privacy) continue to return 501.
- **Bridge test** (T-11). `tests/duplicate_detection.rs` drives
  service-side `Course` records through `to_matcher_course` and the
  canonical `MatchingEngine`, pinning identical-clone scoring,
  Jaro-Winkler typo tolerance, all three deterministic short-circuits
  (DOI / Wikidata / `same_as` URL / shared-provider + course-code),
  negative cases (LMS-id alone, same code at different providers,
  wholly unrelated titles), per-enum field routing
  (`provider_id`, `EducationalLevel`, `LearningResourceType`,
  `Custom` identifier scheme labels), and the strict-⊆-default
  config-preset invariant. 14/14 pass. `matching::matcher_lib` is now
  a plain `pub use`, not test-gated, so the bridge can import the
  matcher types without re-declaring the dependency.
- **Tests.** 27/27 unit + 14/14 bridge — enum round-trip,
  active-model carrying, index lifecycle, exact / fuzzy / provider-
  scoped search, index deletion, identical-records-score-one, DOI
  deterministic short-circuit, find-matches ordering, adapter
  routing rules, every FR-21..FR-28 validation branch, and the full
  matcher-contract bridge suite.

## [0.1.0] — 2026-06-04

Initial scaffold for the Course Service. Models, REST routing, and
docs land; handlers are 501-stubs that the next milestones flesh out
per `spec.md §13` task queue.

### Added

- **Domain model.** `Course` (schema.org/Course-aligned, with full
  Thing + CreativeWork + LearningResource inheritance flattened),
  `CourseInstance` (schema.org/CourseInstance with `Schedule`,
  `CourseMode`, `CourseInstanceStatus`), `Provider` (issuing
  organisation), `CourseIdentifier` (schema.org/PropertyValue shape
  with 12 schemes + `is_deterministic()` discriminator), `Syllabus`
  (hierarchical), `EducationalCredential`, `MergeRequest` /
  `MergeRecord` / `MergeResponse`, `ReviewQueueItem`.
- **REST scaffold.** Axum router under `/api` with route table for
  every FR-1..FR-16 endpoint. Handlers return `501 Not Implemented`
  via the standard `ApiResponse` envelope so the front-end can
  develop against the actual error shape today.
- **Service binary.** `src/main.rs` mirrors person-service:
  `Config::from_env → DB → SearchEngine → CourseMatcher → AppState
  → serve`. Honours `RUST_LOG` via `EnvFilter`, masks DB credentials
  in startup logs.
- **Config.** `Config::from_env` reads `DATABASE_URL`,
  `DATABASE_MAX_CONNECTIONS`, `DATABASE_MIN_CONNECTIONS`,
  `SERVER_HOST`, `SERVER_PORT`, `GRPC_PORT`, `SEARCH_INDEX_PATH`,
  `MATCHING_THRESHOLD`, `OTLP_SERVICE_NAME`, `OTLP_ENDPOINT`,
  `RUST_LOG`.
- **Migrations.** Four numbered SQL pairs:
  `2026060400000001_create_providers`,
  `2026060400000002_create_courses` (courses, course_identifiers,
  course_links), `2026060400000003_create_course_instances`
  (course_instances, syllabus_sections),
  `2026060400000004_create_audit_and_review` (audit_log,
  course_match_scores, course_merge_records).
- **Container.** `Dockerfile` builds against the sibling
  `course-matcher` via the path dependency; debian:13-slim runtime
  base; `podman build` / `podman run` per the family-wide container
  switch (2026-06-03). `docker-compose.yml` brings up Postgres +
  service on host port `8084` (sidesteps person-service's `8080` so
  both can run side-by-side).
- **Docs.** `spec.md` §1–§18, `AGENTS.md` + `AGENTS/{index,
  spec-driven-development, models, matching, restful, testing}.md`,
  `README.md`, `CLAUDE.md`, `index.md` with worked curl examples.

### Spec & roadmap

- Per spec §1.1 / §1.2, the Course Service deliberately models the
  abstract `Course` (template) separately from `CourseInstance`
  (offering). Multiple instances per course; instances may
  eventually reference event-service `Event` resources but ship
  inline for MVP.
- §13 task queue lists T-1 (scaffold) as complete; T-2..T-15 cover
  SeaORM entities, repositories, validation, search, matcher
  adapter, REST implementations, bridge tests, benchmarks, OpenAPI
  completion, and JWT auth.

### Cross-references

- Sibling matcher: [`../course-matcher-rust-crate/`](../course-matcher-rust-crate/) — embedded via Cargo path dependency.
- Front-end consumer: [`../course-front-end-with-svelte/`](../course-front-end-with-svelte/).
- Sibling services for reference: person, worker, place, thing, event.

### Validation

Skeleton compiles (`cargo check --bin course-service`). All handler
stubs return `501`. Unit tests cover model construction + serde
round-trip + `IdentifierType::is_deterministic`.
