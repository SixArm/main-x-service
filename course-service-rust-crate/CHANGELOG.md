# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

Nothing yet.

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
