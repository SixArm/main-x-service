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
### Added

- **`GET /api/courses/{id}/instances/{instance_id}`** — wires the
  single-instance read path that was stuck on `not_implemented` even
  though `list` / `create` / `update` / `delete` were all
  shipped. Drives the existing `CourseRepository::get_instance` so
  the front-end can deep-link straight to one offering without
  fetching the whole parent. Annotated for OpenAPI.

### Fixed

- **AGENTS/testing.md was advertising aspirational tests.** Unit-
  test table marked `matching::adapter`, `validation`, `search`,
  `privacy` as "planned T-X" for tasks that all shipped; bench
  block tagged "planned T-13" though benches landed; integration
  block referenced a `docker-compose.test.yml` that has never
  existed. Rewrote against the real layout: 35 unit tests broken
  down per-module (db / matching / matching::adapter / search /
  validation / streaming / privacy / handlers), the 14 bridge tests,
  the 12 #[ignore]-tagged integration tests with the actual
  Postgres bring-up commands, and the three criterion benches.

- **README was advertising the wrong product.** Status block
  claimed "MVP scaffold — REST routes return 501 Not Implemented"
  long after FR-1..FR-9 and FR-14..FR-18 shipped. Testing block
  marked the bridge suite (T-11) and benches (T-13) as planned
  even though both had landed, and linked to a
  `docker-compose.test.yml` that has never existed. "Next
  milestones (T-2..T-7)" listed a backlog that was 100% complete.
  Rewrote the Status, API, Testing, and Status-summary sections
  against the current state and added the
  `/swagger-ui` + `/api-docs/openapi.json` pointers.

- **AGENTS/restful.md SearchQuery table was aspirational.** Listed
  `educational_level` / `language` / `provider_id` query filters
  that were never implemented and described the handlers as "stubs
  in MVP". Rewrote against the actual `SearchQuery` struct: `q` /
  `limit` / `offset` / `fuzzy` / `phonetic` (no-op) /
  `mask_sensitive` (no-op), with notes on the empty-query → `list`
  fallback and where the Soundex behaviour actually lives. Added a
  pointer to the live `/swagger-ui` + `/api-docs/openapi.json`
  endpoints.

- **`GET /api/courses/{id}` was returning `instances: []`.** FR-2
  mandates the embedded instances collection, but the T-3 repository
  rounds-trip leaves the field empty (instances live in their own
  child table). The `get_course` handler now calls
  `list_instances(&id)` after the repository fetch and embeds the
  result. List + update views stay cheap (no hydration). The
  front-end detail view's `course.instances` rendering becomes
  populated for the first time.

### Changed

- **`ScoredCandidate` wire shape** — added flat `name` and optional
  `course_code` fields next to the existing `course_id`, so the
  front-end can render `/api/courses/match` /
  `/api/courses/check-duplicates` hit lists without a per-row
  round-trip back to `GET /api/courses/{id}`. Both
  `find_probable_duplicates` and `score_all_blocked_candidates`
  populate them from the hydrated candidate. Schema and
  OpenAPI spec updated automatically via the existing `ToSchema`
  derive.

### Added

- **Integration test suite** (T-12). `tests/api_integration_test.rs`
  drives `tower::ServiceExt::oneshot` against the full Axum router
  with real PostgreSQL + Tantivy + the in-memory event publisher.
  12 tests, all `#[ignore]`-tagged so `cargo test --lib` stays fast:
  - `health_returns_ok`
  - `create_get_update_softdelete_lifecycle`
  - `validation_failure_returns_422_with_details`
  - `search_finds_created_record`
  - `check_duplicates_flags_a_clone`
  - `match_endpoint_returns_ranked_candidates`
  - `merge_folds_duplicate_into_main`
  - `batch_dedup_returns_response_shape`
  - `instance_subresource_round_trips`
  - `audit_log_records_create_then_update`
  - `masked_view_clears_provider_and_instructors`
  - `gdpr_export_envelopes_the_record`
  `tests/common/mod.rs` builds `AppState` against env-configured
  Postgres + a process-shared Tantivy `TempDir`. Run with
  `cargo test --test api_integration_test -- --ignored` against a
  migrated DB (see `podman compose up -d`).
- **OpenAPI via utoipa** (T-14). Every wired handler carries a
  `#[utoipa::path]` block; every public domain type (`Course`,
  `CourseInstance`, `Schedule`, `Session`, `CourseIdentifier`,
  `IdentifierType`, `CourseLink`, `LinkType`, `CourseStatus`,
  `EducationalLevel`, `LearningResourceType`, `InteractivityType`,
  `EducationalCredential`, `CredentialCategory`, `Syllabus`,
  `Provider`, `ProviderKind`, `MergeRequest`/`Response`/`Record`/
  `Status`, `BatchDeduplicationRequest`/`Response`, `ReviewQueueItem`/
  `ReviewStatus`, `MatchBreakdown`, `ValidationError`, `AuditEntry`,
  plus handler-local `HealthResponse`, `SearchQuery`, `SearchResponse`,
  `ScoredCandidate`, `AuditQuery`) derives `ToSchema`. `SearchQuery`
  and `AuditQuery` also derive `IntoParams` so query-string args are
  documented. `ApiDoc` aggregator + Swagger UI mounted at
  `/swagger-ui`; raw OpenAPI 3 JSON at `/api-docs/openapi.json`.
  Tagged into 7 groups (`health`, `courses`, `instances`, `search`,
  `matching`, `privacy`, `audit`).
- **Criterion benchmark suite** (T-13). Three benches establish
  perf baselines:
  - `benches/matching_bench.rs` — `match_courses` on a fully-populated
    pair, the deterministic short-circuit path, and `find_matches`
    ranking 100 candidates.
  - `benches/search_bench.rs` — `index_course` on one row,
    exact `search`, `fuzzy_search`, and `search_by_name_and_provider`
    all against a 100-row index.
  - `benches/validation_bench.rs` — `validate_course` on a
    populated record exercising every FR-21..FR-28 branch.
  `criterion = "0.5"` added as dev-dep with three `[[bench]]`
  targets. Run with `cargo bench`.
- **Batch dedup** (T-7c, FR-9). `POST /api/courses/deduplicate`:
  - Pages through every active Course via
    `CourseRepository::list(100, offset)`.
  - For each probe, blocks via `search_by_name_and_provider`, dedupes
    reverse pairs via `canonical_pair` (lexicographic UUID ordering).
  - Below `threshold` → skipped.
  - Above `auto_merge_threshold` → auto-merge inline (same fold +
    record_merge + audit + `CourseMerged` event pipeline as the
    interactive merge handler; soft-deleted ids are tracked in-memory
    so the same row isn't auto-merged twice in the same pass).
  - Between thresholds → `ReviewQueueItem { status: Pending,
    detection_method: "BatchScan", … }` returned in the response
    body (DB-backed review queue persistence deferred).
  - Validates `threshold ∈ [0,1]`, `auto_merge_threshold ∈ [0,1]`,
    `auto_merge_threshold ≥ threshold`; 422 otherwise.
- **Match + Merge handlers** (T-7b, FR-6 + FR-8).
  - `POST /api/courses/match` — scores the request body against every
    blocked candidate (via `search_by_name_and_provider`), returns
    `ScoredCandidate[]` sorted by descending score; the front-end
    applies its own threshold. Empty `name` → 422.
  - `POST /api/courses/merge` — folds `duplicate_course_id` into
    `main_course_id`. Pure `fold_duplicate_into_main` helper unions
    free-text collections (`alternate_names`, `keywords`, `same_as`,
    `image`, `about`, `in_language`, `teaches`, `assesses`,
    `competency_required`, `course_prerequisites`,
    `available_language`, `financial_aid_eligible`), records the
    duplicate's primary name as `[former] <name>` on
    `alternate_names`, dedupes identifiers by `(scheme, value)`, and
    appends a `LinkType::Replaces` link from main → duplicate. Then:
    update main + reindex, soft-delete duplicate + remove from index,
    insert a `course_merge_records` row, audit + emit
    `CourseUpdated` + `CourseDeleted` + `CourseMerged` events.
    `MergeResponse { merge_record, main_course }` mirrors the
    family-wide shape. `CourseRepository::record_merge` is the new
    write surface.
- **Privacy** (T-10, FR-15 + FR-16). `src/privacy/mod.rs`:
  - `mask_course(&Course) -> Course` — clears the course `provider_id`
    and every nested `instances[*].instructor_ids`; replaces each
    `instructor_names[*]` with `[REDACTED]`. Returns a fresh value;
    the input is not mutated.
  - `export_course(&Course) -> Value` — GDPR Article-15 envelope
    `{exported_at, source, schema, course}` wrapping the full
    unmasked record so a data subject (provider, instructor, or
    learner) can be served the data we hold.
  - `GET /api/courses/{id}/masked` (FR-16) and
    `GET /api/courses/{id}/export` (FR-15) wired.
- **Audit + event streaming** (T-9, FR-14 / FR-17 / FR-18).
  - `src/db/audit.rs` — `AuditLogRepository::{log_create, log_update,
    log_delete, list_for_entity, list_recent}`. `AuditEntry` is the
    public read shape.
  - `src/streaming/mod.rs` — `CourseEvent` (with `course` / `instance`
    constructors), `EventKind` (`CourseCreated`, `CourseUpdated`,
    `CourseDeleted`, `CourseMerged`, `CourseInstanceCreated`,
    `CourseInstanceUpdated`, `CourseInstanceDeleted` — PascalCase),
    `EventPublisher` trait, `InMemoryEventPublisher` MVP capturing
    events in an `Arc<Mutex<Vec<_>>>` so the planned integration
    suite (T-12) can assert on them.
  - `AppState` now carries `audit_log: Arc<AuditLogRepository>` and
    `event_publisher: Arc<dyn EventPublisher>`.
  - Create / update / soft-delete handlers for Course AND
    CourseInstance call `audit_log.log_*` and `event_publisher.publish`
    fire-and-forget (warn on failure, do not fail the request).
  - `GET /api/courses/{id}/audit` and `GET /api/audit/recent` wired
    with an optional `?limit=` (default 20). Fluvio adapter under
    feature flag deferred.
- **Instance sub-resource** (T-8). `CourseRepository` grows
  `list_instances` / `get_instance` / `create_instance` /
  `update_instance` / `soft_delete_instance`. Round-trips the
  `course_instances` row plus its JSONB `schedule` blob. Four new
  handlers under `/api/courses/{id}/instances`:
  - `GET` — lists active instances, sorted `schedule.start_date DESC
    NULLS LAST` (sort in-memory after hydration since `schedule` is
    JSONB). FR-10.
  - `POST` — validates via `validate_instance`, returns `201`. FR-11.
  - `PUT /{instance_id}` — replaces. FR-12.
  - `DELETE /{instance_id}` — soft-delete. FR-13.
  Parent-course existence is enforced for FR-10 / FR-11 (returns
  `404` if the course is missing or soft-deleted).
- **Changed.** `CourseInstanceStatus` and `CourseMode` now serialise
  as `snake_case` so `EnrollmentOpen` round-trips against the DB
  `CHECK` constraint as `enrollment_open` (was `enrollmentopen`).
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
