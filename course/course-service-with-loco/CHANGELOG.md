# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]
### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration → 2.0,
  sea-query → 1.0. This is the family's first "person-style" crate to
  migrate (explicit `default-features = false` feature list rather than
  the loco-style default set), so it's the first to hit the feature
  renames: `auth_jwt` → `auth`, `bg_pg` → `worker`. The renamed features
  gate exactly what they did before (the unused `bgworker::Queue`
  scaffold in `src/app.rs` still compiles); no code depended on the old
  names beyond the `Cargo.toml` feature list itself.
- A `useless_conversion` in `src/db/outbox.rs` from a now-redundant
  `.into()` after `Expr::current_timestamp()`.
- No PK-width fallout: this crate's tables don't use loco's
  `ColType::PkAuto` schema-DSL helper (the outbox already keys on
  `i64`), so the 64-bit-primary-key change that touched every
  loco-style crate in this migration doesn't apply here.
- No behavioural change; verified with the full DB-gated suite (15
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-2, the loco-style half of the rollout (case was the reference; the
five axum-style services landed the same day as AU-1).

- **The verifier and the ABAC policy are now reloadable holders**
  (`ReloadableVerifier` / `ReloadablePolicy`) that the blanket guard
  **and** the bearer extractors read per request. They were boot-only
  `OnceLock` snapshots, so a rotated key set or an edited policy could
  not have reached a running process at all.
- **`spawn_key_refresh`** re-fetches `COURSE_PASETO_KEYS_URL` every
  `COURSE_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset). A failed fetch **keeps the current key set** —
  a transient auth-service outage must not lock every caller out.
- **`spawn_policy_watcher`** polls `COURSE_ABAC_POLICY_FILE`'s mtime every
  15 s and calls `reload_policy()`; a malformed edit falls back to the
  built-in default rather than leaving the service unprotected.
- **`tests/enforcement.rs`** — the activation proof, new here and in its
  own binary, carrying a minimal builder for the production router since
  this crate's other tests do not boot one.
- New environment variable: `COURSE_PASETO_KEYS_REFRESH_SECS`.

### Fixed — the DB-gated suite ran for the first time (2026-08-01)

- **`POST /api/courses` stored an all-zeros `id` verbatim.** `Course::id`
  mints a fresh UUID via `#[serde(default)]`, but a serde default only
  applies to an *absent* field — an explicit nil UUID (a widespread "you
  pick" sentinel, and what the event service already treats as one) was
  written through. The first such create claimed the nil id; every later
  one died on `duplicate key value violates unique constraint
  "courses_pkey"` with a `500`. The handler now mints on nil.
- **Test fixtures fought the duplicate detector.** Integration courses
  were named `Integration <suffix> <micros>`; consecutive microsecond
  timestamps share nearly every leading digit, so two such names scored
  ~0.92 on Jaro-Winkler and each create after the first came back `409
  DUPLICATE_CANDIDATE`. Swapping in a random UUID was not enough — the
  constant `Integration ` prefix kept Jaro-Winkler's prefix bonus at
  ~0.88. Names now lead with the random token, so they differ from the
  first character. The detector was never wrong here; the fixtures were.

  Suite: 14/14 green vs Postgres 18; crate enrolled in
  [`ci/db-suites.txt`](../../ci/db-suites.txt).


### Changed — `Config::from_env` gained a testable seam and more variables (2026-07-23)

- The env overlay moved into a pure `Config::from_source(lookup)`;
  `from_env` is now a two-line delegation to it. This makes the
  variable-to-field mapping unit-testable without mutating the process
  environment — which matters because `std::env::set_var` is `unsafe`
  in the 2024 edition (this crate forbids `unsafe`) and process env is
  global state that makes parallel tests flaky.
- Added variables: `SEARCH_CACHE_SIZE_MB`, `STREAMING_BROKER_URL`,
  `STREAMING_TOPIC` (the previously-unreachable config fields).
- A blank or whitespace-only value now counts as **unset** rather than
  overwriting the default with an empty string, and typed values
  tolerate surrounding whitespace (a `.env` line like `SERVER_PORT = 9090 `).
- Pinned by five unit tests; behaviour is otherwise unchanged.

### Security

- **SEC-M1: input-size caps on the `Course` payload.** The FR-21..FR-28
  validator enforced semantic rules but capped no field's *size*, so a
  single multi-megabyte text field or a huge array (or a huge `instances`
  list) could be a CPU/memory `DoS` against the matcher's O(n·m)
  Jaro-Winkler / Levenshtein / Jaccard scoring, amplified across
  `check-duplicates`. `validate_course` / `validate_instance` now also
  bound every scalar text field (`MAX_TEXT_LEN = 1024`), string-array
  cardinality + per-entry length (`MAX_ARRAY_LEN = 256` / `MAX_ITEM_LEN =
  512`), and the cardinality of the language/struct lists (`identifiers`,
  `syllabus_sections`, `instances`, `links`) — returning field-scoped
  `422`s *before* the record is stored or matched. The caps are factored
  into `course_size_caps` / `cap_*` helpers. `course_code` keeps its
  stricter FR-22 1..=100 cap; the BCP-47 language entries keep their FR-24
  length bound. Unit tests: oversized text / array / array-item / huge
  `instances`, plus a within-caps large record accepted.

- **SEC-G5: blanket guard switched from prefix-gate to guard-all
  (deny-unless-public).** `auth::enforce` previously returned `Ok`
  (bypassing auth) for any path **not** under `/api` and **not** under
  `/fhir`, leaving out-of-prefix routes (`/`, `/admin`, …) unguarded when
  enforcement was on. It now denies by default: only the small
  `is_public_path` allow-list (`/_health`, `/_ping`,
  `/api-docs/openapi.json`, `/swagger-ui*`, `/metrics.prom`, `/api/health`,
  `/fhir/metadata`) is public; every other route requires a valid bearer
  token. Removed the now-dead `API_PREFIX` / `FHIR_PREFIX` /
  `PUBLIC_API_PATHS` constants and the `is_api_path` / `is_fhir_path`
  helpers.
- **SEC-G6: trailing-slash normalisation in `derive_action`.** A path is
  now `trim_end_matches('/')`-normalised before the destructive-suffix
  check, so `POST /api/courses/merge/` (and `//`) stays `Destructive`
  rather than being silently downgraded to `Write` — which would let a
  non-admin `access=write` caller reach a destructive op.
- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance every relay would **double-ship** the
  same rows. `drain_once` now runs in a transaction and `unpublished` claims
  rows with `FOR UPDATE SKIP LOCKED` (a second relay skips locked rows; the
  lock releases on commit). Delivery stays at-least-once (consumers dedupe on
  `event_id`).

### Fixed

- `src/api/rest/mod.rs` had rustfmt drift (test-module comment/line
  wrapping) that broke the crate's `cargo fmt --check` gate.
  Reformatted with `cargo fmt`; no behavioural change, 42 lib tests
  and clippy `-D warnings` unchanged and green.

## [0.3.0] — 2026-06-15

### Added

- **Prometheus metrics — `GET /metrics.prom`** (T-16). A new
  process-wide `prometheus::Registry` (`src/metrics.rs`, behind a
  `OnceLock` reached via `Metrics::global()`) is served at the
  application **root** path `/metrics.prom` in text-exposition format
  (`text/plain; version=0.0.4`) by a loco controller route
  (`metrics_routes()` in `src/api/rest/mod.rs`, mounted at root like the
  docs; also wired into `create_router` for the Axum test surface). The
  metric set: `course_created_total`, `course_updated_total`,
  `course_deleted_total`, `course_merged_total` (plain counters,
  incremented one per success path in the create/update/delete/merge
  handlers) plus a labelled `http_requests_total` (`path`/`status`)
  declared for a future request middleware. The endpoint is public (no
  bearer token needed to scrape) and carries a `#[utoipa::path]`
  annotation so it appears in the OpenAPI document. New DB-free tests:
  registry render + counter increment (`metrics::tests`) and root
  mounting of the metrics route (`api::rest::tests`). Brings parity with
  the sibling services, which already expose Prometheus metrics.

### Fixed

- **Dockerfile non-root user was named `mpi`** (Master Patient
  Index) — leftover from the person-service copy-adapt. Renamed
  to `course` across the `useradd`, `USER`, and four `--chown`
  references so the layer audit and any future ps-output during
  troubleshooting names the actual service, not its sibling.
  Behavioural no-op (same uid 1000, same /app layout); cleanup
  only.
- **OpenAPI `info.version` was hardcoded `0.1.0`.** After the
  v0.2.0 cut, the spec served at `/api-docs/openapi.json` (and
  rendered by Swagger UI) still advertised v0.1.0 — any consumer
  pulling the schema for codegen would have stamped the wrong
  version on generated clients. Replaced the literal with
  `env!("CARGO_PKG_VERSION")` so the OpenAPI info can't drift
  from the crate version again.
- **`handlers.rs` doc comments overstated the `not_implemented`
  surface.** Header doc claimed "a couple of placeholder routes",
  and the handler's own doc said "every endpoint not yet ticked
  off in `spec.md §13` routes here". Only one route uses the shim
  (`GET /api/courses`, deliberately parked per §9); §13 itself is
  fully closed except T-15 (auth). Rewrote both doc comments to
  name the single endpoint and explain why it stays.

### Documentation

- **Doc-harmonization pass** reconciling the SDD artefacts with the
  shipped T-16 state and the version bump to 0.3.0:
  - **Unit-test count 35 → 42** everywhere it was quoted (spec §11,
    §14, `index.md` ×2, `AGENTS/testing.md` ×2). The count had not
    been reconciled after T-16's metrics tests landed; this pass also
    adds three new DB-free tests (a live `GET /metrics.prom` via
    `tower::oneshot`, a `canonical_pair` order-independence pin, and a
    batch-dedup FR-9 threshold-band classification pin), bringing the
    real total to 42.
  - **spec §14** gains a **Metrics** row (Prometheus `/metrics.prom`,
    T-16) and the Tests row reads 42.
  - **spec §15 roadmap** re-cut: v0.3 marked **shipped** (T-16 + this
    pass); JWT (T-15) + Fluvio + the `http_requests_total` middleware
    moved to v0.4; syllabus sub-resource → v0.5; LMS round-trip → v0.6+.
  - **Toolchain floors bumped** to the mandated stack: `Dockerfile`
    builder `rust:1.93-slim` → `rust:1.95-slim`; `docker-compose.yml`
    and `index.md` `postgres:17-alpine` → `postgres:18-alpine`;
    `index.md` prerequisites Rust 1.93+/PostgreSQL 17+ → Rust 1.95+
    (2024 edition) / PostgreSQL 18+.
  - **Worked examples added**: `GET /metrics.prom` scrape output and a
    `409 Conflict` duplicate-on-create response shape (FR-1 / FR-20,
    `ScoredCandidate[]` under `error.details`) in both `index.md` and
    `AGENTS/restful.md`; `AGENTS/restful.md` Health & ops table gains a
    `/metrics.prom` row.
  - **spec §10** documents why `sea-orm` retains the `with-time`
    feature (older-service convention shared across the first-converted
    loco services); cross-service harmonization to `with-chrono` is
    queued as **T-17** and the `http_requests_total` request-path
    middleware + end-to-end metrics integration test as **T-18** in §13.

## [0.2.0] — 2026-06-05

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

- **spec.md §11 / §15 / §16 stale milestones.**
  - §11 Testing Strategy referenced a
    `docker-compose.test.yml` that has never existed; rewrote
    against the real `--ignored` integration flow + the regular
    docker-compose.yml's postgres service. Filled in per-layer
    test counts (35 unit / 14 bridge / 12 integration).
  - §15 Roadmap listed v0.2..v0.4 as future work though every
    task in those buckets (T-2..T-14) has landed. Re-cut the
    roadmap: v0.2 = shipped, v0.3 = JWT auth + Fluvio adapter,
    v0.4 = syllabus-section sub-resource (the actual remaining
    gap), v0.5+ = LMS round-trip.
  - §16 OQ-3 ("Should `CourseCode` be deterministic? To be decided
    in T-6") marked **resolved**: provider-scoped via rule R-1
    (`provider_id + normalised(course_code)` → 1.0), not promoted
    to the `is_deterministic()` set.

- **spec.md §9 / §13 / §14 still had stale "MVP scaffold" claims.**
  §9 said `501` was returned "for any endpoint not yet implemented
  in the MVP scaffold" — true at v0.1 but only `GET /api/courses`
  (list-without-search) remains 501 today. §13 T-3 said
  "audit-log writes still pending alongside T-9 event publisher" —
  T-9 shipped iterations ago. §14 Implementation-Status table
  showed "Skeleton...REST routes return 501 🚧 in progress",
  "instances + syllabus pending T-8", and "Tests 27 unit + 14
  bridge" — all outdated. Rewrote the row set against current
  numbers (35 unit + 14 bridge + 12 integration + 3 benches; 9
  SeaORM modules; T-6 phonetic bonus called out).

- **AGENTS.md `Where work lives` table** marked `src/validation/`
  and `src/privacy/` as "(planned, T-5/T-10)" though both shipped.
  Promoted both rows and added entries for audit / streaming /
  bridge tests / integration tests / benchmarks / OpenAPI so the
  table maps the whole current surface.

- **AGENTS/models.md `CourseInstanceStatus` column omitted the wire
  shape.** Listed the Rust variants in PascalCase but didn't mention
  the snake_case JSON / DB encoding (`enrollment_open`, not
  `EnrollmentOpen` or `enrollmentopen`). A client writing
  `"status": "EnrollmentOpen"` from this table alone would have
  hit a serde-rejected payload. Added the JSON wire-shape note.
- **AGENTS/models.md + AGENTS/matching.md "planned T-2 / T-6"
  pointers** dropped to "shipped" — both modules have been live
  for several iterations. Also added a Phonetic-bonus subsection
  to AGENTS/matching.md pointing at the matcher's T-6 work
  (`+0.05` capped at `0.95`, initial-letter-preserving).

- **index.md curl examples were unreachable.** Used
  `http://localhost:8080` while docker-compose maps the service to
  host port 8084 — copy-pasting any example from the index hit the
  wrong (or no) service. Rewrote every example to 8084.
- **index.md match example shipped `threshold` in the body.** Same
  drift as the front-end's match page: the handler accepts a
  `Course`-shaped body, the `threshold` field is silently dropped
  on the wire. Removed it and added a comment naming where the
  threshold cutoff actually applies (client-side, post-response).
  Added a Swagger UI / OpenAPI pointer to the bottom of the
  "worked examples" block.

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
