# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]
### Fixed — SEC-B8 follow-up: per-row create/update audit rows carry the real actor

Closes the item the original SEC-B8 fix (below) explicitly deferred:
`PersonRepository::create`/`update`/`delete`/`merge` built their own
`AuditContext::default()` internally, so every per-row `CREATE`/`UPDATE`/
`DELETE` audit row — from a single `POST`/`PUT`/`DELETE
/api/persons(/{id})`, a FHIR `Patient` write, a merge, or a bulk-imported
row — was stamped `"system"` regardless of who actually made the request.
The job-level bulk audit row already carried the real actor; the per-row
rows underneath it did not, which is the gap a HIPAA §164.312(b) audit
trail cannot have: it must say *who* touched a specific record.

- **`PersonRepository` trait signature change** (`src/db/repositories.rs`):
  `create`, `update`, `delete`, and `merge` each now take an
  `&AuditContext` parameter instead of hard-coding
  `AuditContext::default()` inside `SeaOrmPersonRepository`. The context
  is threaded into the `CREATE`/`UPDATE`/`DELETE` audit-log writes and
  also stamps the `persons.created_by`/`updated_by`/`deleted_by` columns
  (previously always `None`, or a hard-coded `"system"` for
  `deleted_by`) — neither column feeds the content-integrity digest, so
  this is a pure provenance improvement with no compatibility impact on
  `/api/records/verify` or `/api/audit/verify`. `merge` uses one actor
  for both halves of the merge (the survivor's `UPDATE` row and the
  duplicate's `DELETE` row) — one operator initiated the whole action.
- **New helper `auth::audit_context_of`** (`src/api/rest/auth.rs`):
  builds an `AuditContext` from a `MaybeAuthUser` — the bearer `sub` when
  a valid token was presented, else the `"system"` fallback — generalizing
  the pattern the existing `review_decision` handler already used inline.
- **Call sites updated**: REST `create_person` (gained a `MaybeAuthUser`
  extractor it did not previously take), `update_person`, `delete_person`,
  `merge_persons` (also gained the extractor); the FHIR
  `create_fhir_patient`/`update_fhir_patient`/`delete_fhir_patient`
  handlers (none previously took a caller at all); the bulk import
  pipeline (`src/bulk/pipeline.rs`: `process_import_job`,
  `import_upsert_locked`, `create_and_queue_for_review` all now take an
  `&AuditContext`), fed by `bulk::worker::run_import`'s existing
  `actor_audit_context(job)` — so a bulk-imported record's per-row audit
  row now names who ran the import, matching what the job-level row
  already said. The CLI `seed_examples` task and the crate's own
  DB-gated test fixtures use `AuditContext::default()` explicitly — a
  legitimate `"system"` actor, not a gap (no authenticated caller
  exists at those call sites).
- *Tests:* two new DB-gated tests in `src/db/repositories.rs`
  (`create_and_update_audit_rows_carry_the_real_actor`; the existing
  `merge_writes_the_audit_rows_in_transaction` extended to assert the
  real actor on both the survivor's `UPDATE` and the duplicate's
  `DELETE` row) plus one extended in `src/bulk/pipeline.rs`
  (`keyless_row_with_a_likely_duplicate_creates_and_queues_for_review`
  now asserts the imported row's `CREATE` audit row carries the job's
  actor, not `"system"`); a new DB-free unit test for
  `audit_context_of` in `src/api/rest/auth.rs`.

Verified against a real Postgres 18: full DB-gated suite green (22
`--lib` + 25 `api_integration_test` + 5 `cross_service_link_review` + the
other integration suites). `cargo test --lib`: 316 passed (up from 315:
+1 new DB-free unit test). `cargo fmt --check` / `cargo clippy
--all-targets -- -D warnings` clean.

### Added — T-33 regression pin: `matcher_suggested` link creation is audited (link-graph T-33, OQ-9(d))

link-graph's T-33 (governance + scale controls + audit for the
cross-service suggestion job, closing LNK-4) needed to confirm "the
suggestion job audits every POST it makes" rather than merely assert
it. Investigation found `create_link` already writes an unconditional
best-effort `person_link` audit row for every link creation regardless
of `provenance` — no code change was needed on this side, only a test
proving it.

- **`tests/cross_service_link_review.rs`** — new
  `matcher_suggested_link_creation_is_audited`: `POST`s a
  `matcher_suggested` `same_identity` link and confirms exactly one
  `CREATE`/`person_link` `audit_log` row exists naming the created
  link's id, with `provenance = "matcher_suggested"` and the correct
  `to_ref` in its `new_values` snapshot.

Verified against a real Postgres 18: full DB-gated suite green (21
`--lib` + 25 `api_integration_test` + 5 `cross_service_link_review`,
this new test included). `cargo test --lib`: 315 passed (unchanged — no
new `src/` unit tests, only this one integration test). `cargo fmt
--check` / `cargo clippy --all-targets -- -D warnings` clean.

### Added — cross-service review-queue bridge + promotion/rejection (link-graph T-32, OQ-9(b))

Closes a gap T-31 (link-graph's periodic suggestion job) left open: that
job already `POST`s `matcher_suggested` `same_identity` edges to
`create_link`, but the handler never touched `review_queue` — a
suggestion landed on `entity_links` with no way for an operator to see
or decide it. Per
[cross-service-linking.md](../../agents/share/cross-service-linking.md)
§5.2 and link-graph's `spec/16-open-questions.md` OQ-9(b), person's
*existing* review-queue table and decision endpoint are reused rather
than adding a new aggregator-hosted review surface.

- **`src/api/rest/links.rs`** — `LinkRequest` gained an optional
  `score_breakdown` field (meaningful only for `kind = "same_identity"`
  + `provenance = "matcher_suggested"`); `create_link` now best-effort
  queues a `review_queue` row (`queue_cross_service_review`) after a
  suggested edge is written; two new `pub(crate)` functions,
  `promote_cross_service_link` and `reject_cross_service_link`, reused
  by `review_decision` (below) — reasserting/withdrawing via the exact
  same `upsert_and_emit`/`soft_delete_and_emit` paths `create_link`/
  `delete_link` already use, so a promotion is the same edge id, not a
  new one.
- **`src/db/review_queue.rs`** — new `upsert_cross_service`, which does
  **not** normalize `(record_id_a, record_id_b)` order the way `upsert`
  does. That normalization is correct for within-entity dedup (both ids
  are the same entity type, so which column holds which is meaningless)
  and would be actively wrong here: `record_id_a` must always be the
  person pid and `record_id_b` always the worker pid, and reordering by
  raw `Uuid` comparison would silently swap them for roughly half of all
  pairs.
- **`src/db/entity_links.rs`** — new `find_active_by_key`, a
  `(from_pid, kind, to_ref)` natural-key lookup for the rejection path
  (a review-queue row carries no `edge_id` to key on directly).
- **`src/models/review_queue.rs`** — new `match_quality_for_score`,
  extracted from three previously-duplicated inline `if score >= 0.95 {
  ...}` blocks (`match_person`, `check_duplicates`, `batch_deduplicate`
  in `handlers.rs`) and reused by the new cross-service write, so the
  certain/probable/possible thresholds live in exactly one place.
- **`src/api/rest/handlers.rs`** — `review_decision`'s `confirmed`
  branch now also promotes the edge and its `rejected` branch withdraws
  it, gated on **both** `provenance == "matcher_suggested"` **and**
  `detection_method == "cross_service_same_identity"` so an ordinary
  within-entity decision is unaffected; best-effort (logged on failure,
  never turns a successful decision into an error response).
- **`link-graph-service-with-loco`'s `src/suggest/job.rs`**
  (`HttpSuggestionSink::post_suggestion`) now sends its T-29
  `IdentityMatchScore` as a `score_breakdown` object alongside
  `kind`/`to_ref`/`confidence`/`provenance` — see that crate's own
  `CHANGELOG.md`.
- New `tests/cross_service_link_review.rs` (DB-gated, 4 tests): a
  suggestion POST creates the right review-queue row (fields + score
  breakdown carried through); confirming promotes the same edge (not a
  duplicate); rejecting withdraws it; an ordinary
  `provenance="operator"`/`detection_method="batch_deduplication"`
  decision writes no `entity_links` edge (the regression pin — proves
  the gate, not just asserts it).

Verified against a real Postgres 18 (`scripts/ci-check.sh test-db`):
all pre-existing DB-gated suites green (21 `--lib` + 25
`api_integration_test`), plus the 4 new tests. `cargo test --lib`: 315
passed (was 314; +1 for `match_quality_for_score`'s boundary test).
`cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` clean.

### Added — `GET /api/persons`, a genuine database-backed list endpoint (link-graph T-31 follow-up)

Found while building `link-graph-service`'s cross-service `same_identity`
suggestion job (T-31), which needs to enumerate every person: this
service had **no way to list its own collection**. `GET
/api/persons/search` is the closest thing, but it is Tantivy-backed and
requires a non-empty `q`; a live investigation (bring up the real
server, create a real record, query it) found that even `q=*` — which
the query grammar's own `UserInputLeaf::All` production maps to
`AllQuery` in isolation, confirmed against the actual pinned
`tantivy-query-grammar` 0.22.0 source, not a guess — could come back
empty against a real running instance. Root cause: this crate's `.env`
carries a **stale** `SEARCH_INDEX_PATH=./search_index` (a pre-loco-
conversion leftover, alongside an equally stale `mpi_user`/`mpi`
`DATABASE_URL`) pointing at a long-lived index directory that had
accumulated ~900 documents across unrelated past dev sessions, with no
lifecycle tie to whatever database happens to be attached at any given
time — `q=*` correctly matched *all* of those (proven with `AllQuery`
instrumentation logged inside the real running server, not inferred),
but a small `limit` page landed entirely on entries with no surviving
database row, so every hit was silently dropped by the
found-in-index-but-not-in-database guard and the page came back empty.
A **clean** index reproduces `q=*` correctly — so the search index was
never "broken" — but a search index can *always* legitimately drift
from the database in any deployment (resets, deletes, partial reindex
failures), which makes it the wrong foundation for a "list everything"
primitive regardless of this one instance's specific cause.

- **`src/api/rest/handlers.rs`** — `list_persons` (`GET /api/persons`),
  `ListQuery` (`limit`/`offset`/`mask_sensitive`, same shape as
  `SearchQuery` minus the query fields), `ListResponse`. Sources from
  [`PersonRepository::list_active`](src/db/repositories.rs) — a plain
  paginated database query — **never** the search index, so its answer
  is only ever as stale as the database itself. Reuses `search_persons`'s
  existing SEC-G7 offset bound (`search_offset_within_bound`,
  `MAX_SEARCH_OFFSET`) and SEC-G3 per-record read authz/masking
  (`search_result_disposition`) verbatim — an aggregate read must never
  reveal more than the equivalent single `GET`, exactly as already
  proven for search.
- **`src/db/repositories.rs`** — `PersonRepository::list_active` gained
  `.order_by_asc(persons::Column::Id)`. This was latent and load-bearing,
  not cosmetic: without an explicit `ORDER BY`, Postgres does not
  guarantee a stable row order across repeated `LIMIT`/`OFFSET` calls, so
  a caller paginating through every page — the bulk-export pipeline
  already did this; the link-graph suggestion job now does too — could
  silently skip or duplicate rows between pages.
- **`src/api/rest/mod.rs`** — `GET /api/persons` wired onto the existing
  `POST /api/persons` route (both router surfaces: `create_router` for
  tests, `persons_routes()` for the real boot path) and the OpenAPI
  `paths`/`schemas` registry.
- Tests: `tests/api_integration_test.rs` —
  `test_list_persons_paginates_every_created_record_exactly_once`
  creates seven persons with genuinely distinct surnames (a shared long
  prefix differing by only a trailing digit scores high enough under
  Jaro-Winkler to trip real-time duplicate detection — an early version
  of this test 409'd on itself) and pages through the **whole**
  collection with a `limit` smaller than the created count, asserting
  every created id is seen **exactly once** across pages — proving both
  the no-loss and the no-duplication properties the new `ORDER BY`
  guarantees. `test_list_persons_rejects_out_of_bound_offset` pins the
  SEC-G7 bound. Both pass against a real migrated Postgres
  (`DATABASE_URL=… cargo test --test api_integration_test -- --ignored`).
  **Also verified against a live running server** (not just the test
  harness): 25 real persons created via `POST /api/persons`, then
  enumerated page-by-page via `GET /api/persons?limit=7&offset=…` —
  4 pages, all 25 seen, zero missing, zero duplicates.
- `AGENTS/restful.md` documents the new endpoint and adds a "not a
  list-all mechanism" note under `/persons/search` explaining why, so
  the next reader does not reach for `q=*` again. `spec/09-api-surface.md`
  endpoint count bumped 15 → 16.
- Consumed by `link-graph-service`'s T-31 suggestion job
  (`src/suggest/job.rs`), which switched from `search?q=*&…` to this
  endpoint in the same fix — see that crate's own `CHANGELOG.md`.

### Verified — `PersonRepository::search`'s `cust_with_values` SQL is sound (QA-CUST-SQL)

Audited for the same MySQL-placeholder footgun fixed in
`authentication-service` (`Expr::cust_with_values("LOWER(email) = ?", …)`
— a `?` placeholder Postgres rejects). This crate's `search()`
(`src/db/repositories.rs`) already spells `"LOWER(family) LIKE $1"` —
Postgres-style — so the specific defect does not apply. Unlike worker
and event, this method is **not dead code**: the bulk export pipeline
(`src/bulk/pipeline.rs::run_export`) calls `repo.search(q)` whenever an
export request carries a `query` filter. It is already exercised by an
existing DB-gated test, `bulk::pipeline::db_tests::export_round_trips_through_jsonl`
(plus the CSV/masking/Parquet export tests that also set
`query: Some(...)`) — all confirmed green against Postgres 18
(`scripts/ci-check.sh test-db`, 21/21 lib unit tests including these).
No code change; the decision is "exercise it, and it already is."

## [0.5.0] - 2026-08-04
### Fixed — `merge` (and any `use_type`/`telecom`) writes were rejected by the database (PERSON-CONTACT-CASE, 2026-08-04)

Every merge of two *different* persons failed with `500 DATABASE_ERROR`
("`patient_names_use_type_check`"). `merge_duplicate_into_main`
unconditionally sets the duplicate's aliased name to `NameUse::Old` and
adds a `LinkType::Replaces` link, and `src/db/repositories.rs` wrote
`NameUse`/`IdentifierUse`/`ContactPointSystem`/`ContactPointUse`/
`LinkType` via `format!("{:?}")` — `"Old"`, `"Phone"`, `"Replaces"` —
into columns whose CHECK constraints accept only lowercase
(`migrations/2024122800000003_create_patient_related_tables`). This is
the same defect `examples/data/README.md` already documented for
`telecom`/name `use_type` on fixture data, but unconditional on merge:
no test caught it because no test posts a name/identifier with
`use_type` set, and the suite's only merge test is the self-merge
rejection guard, which exits before the insert. Found writing TUT-2
(`tasks.md`), whose entire premise depends on merge working.

Fix: the write side now uses the pre-existing `enum_to_tag` helper
(already correct for `person_addresses`/emergency-contact tables)
instead of `format!`; the read side now uses `tag_to_enum` instead of
hand-rolled `PascalCase` match arms. `identifier_type`'s `Other`
variant (Debug: `"Other"`, CHECK: `'OTHER'`) is fixed the same way.
`NameUse` and `LinkType` gained `PartialEq, Eq` for the new regression
test's assertions. New DB-gated
`test_merge_two_persons_round_trips_alias_name_and_replaces_link`
(`tests/api_integration_test.rs`) merges two real persons and re-fetches
the survivor, pinning both the write (insert succeeds) and read (stored
lowercase tags deserialize to the right enum variants) sides together.

**Residual, narrower gap, not fixed here:** `LinkType::ReplacedBy`'s
`#[serde(rename_all = "lowercase")]` produces `"replacedby"`, not the
CHECK's `'replaced_by'` — nothing in this crate constructs that variant
today, so it's tracked but not blocking.

### Added — `seed_examples` CLI task (EX-4, 2026-08-04)

`cargo loco task seed_examples` loads the repository's shared demo
fixture (`examples/data/persons.jsonl`, 50 rows including five
deliberate duplicate pairs) into the `persons` table, for the
tutorials (`tasks.md` EX-4). Inserts via the **model-layer create**
(`db::repositories::SeaOrmPersonRepository::create`) rather than
`POST /api/persons`, deliberately bypassing real-time duplicate
detection — the normal create endpoint returns `409` on the second
half of every duplicate pair (confirmed live by EX-1), which would
silently drop half the fixture. No audit row or event is written by
the seed itself (no audit log / event publisher attached); the
tutorials that exercise duplicate detection, audit, and events do so
against the seeded records afterward. Refuses to insert into a
non-empty `persons` table (prints a message and exits cleanly), so a
second run is a no-op rather than a duplicate load. New
`src/tasks/seed_examples.rs` (`parse_fixture`, `seed`, `SeedExamples`);
DB-free unit tests parse the real fixture; a DB-gated
`tests/seed_examples_db.rs` proves a first run seeds all 50 rows
(including both halves of the "Okonkwo/Okonkow" pair) and a second run
changes nothing.

### Fixed — cross-service link endpoints now use the uniform response envelope (2026-08-03)

`POST`/`GET`/`DELETE /api/persons/{pid}/links` previously returned bare
JSON bodies while every other person REST endpoint wraps in
`{success,data,error}` (`ApiResponse<T>`) — a front-end client that
unwraps `.data` would have silently read these as `undefined`. Fixed;
the bulk aggregator endpoint (`GET /api/persons/links`) is unchanged
(still bare, for the link-graph aggregator's HTTP client). New DB-gated
regression test pins the wrapped shape end-to-end.

### Added — Durable event bus, real-broker sink (BUS-3, 2026-08-03)

`FluvioSink` (`src/relay.rs`) — the Phase-3 relay's real-broker
`EventSink`, ported from case-service's BUS-1 reference implementation,
behind this crate's own `fluvio` Cargo feature (off by default; `fluvio`
0.50). One producer per topic, partitioned by record `pid` per
`agents/share/event-bus.md` §7. New env vars: `PERSON_FLUVIO_ENDPOINT`
(unset ⇒ unchanged `LoggingSink` default) and `PERSON_EVENT_TOPIC`
(default `mxi.person.events`). An endpoint configured without the
`fluvio` feature refuses to start the relay rather than silently
falling back to `LoggingSink` — that fallback would mark outbox rows
`published_at` without ever reaching the broker the operator asked for.
`compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a local
SC+SPU broker for opt-in manual runs (not part of any automated CI
stage; host ports offset from case-service's so both can run side by
side); `tests/fluvio_relay.rs` is a feature-gated, `#[ignore]`d
live-broker round-trip, verified by compiling under `--features
fluvio` rather than an actual execution (no broker is stood up in this
repo's CI) — it builds the outbox row directly via
`db::outbox::OutboxInsert` rather than a `create_and_emit`-style
helper, since this crate's write path enqueues the outbox row inside
`PersonRepository::create`/`update`/`delete`. SOUP register updated.

### Added — S3-compatible bulk artifact store, feature-gated (2026-08-02)

- **`ArtifactStore` (`src/bulk/store.rs`) is now async** and gained an
  `S3ArtifactStore` backend alongside `LocalFsArtifactStore`, ported from
  the care-pathway service (the family's reference implementation,
  `agents/share/bulk-import-export.md` §12).
- `PERSON_BULK_ARTIFACT_BACKEND` selects `local` (default) or `s3`
  (behind this crate's own `s3` Cargo feature, off by default; an
  unknown value falls back to `local` with a warning; `s3` without the
  feature is a clean error, never a silent local-storage fallback).
- S3 configuration: `PERSON_BULK_S3_{BUCKET(required),ENDPOINT,REGION
  (default us-east-1),FORCE_PATH_STYLE(default on)}`; credentials from
  the standard AWS credential chain. References are `s3://<bucket>/<key>`
  URLs; `presigned_get` issues a short-lived download URL (TTL clamped to
  `[1, 3600]` seconds) and refuses a reference naming a foreign bucket.
- `AppState::new` (`src/api/rest/state.rs`) is now `async fn … ->
  crate::Result<Self>`, since the S3 backend's credential resolution is
  async; both call sites (`app.rs`, `tests/common/mod.rs`) were already
  async.
- New optional dependencies `aws-config`, `aws-sdk-s3`,
  `aws-credential-types` (1.x), gated by the `s3` feature; SOUP register
  updated.

### Added — Parquet export, feature-gated (2026-08-02)

- **`format: "parquet"`** on `POST /api/persons/export` — **export-only**
  (the import handler refuses it, on every build) and **feature-gated**
  behind this crate's own `parquet` Cargo feature (off by default): the
  `arrow`/`parquet` dependencies only exist when the feature is on, so a
  deployment that never needs Parquet carries none of that weight. A
  binary built without the feature still accepts `format: "parquet"` as a
  recognised token but returns a clean `422` rather than silently
  substituting JSONL.
- The CSV column-flattening declaration (spec §10.6) moved to a new
  shared `src/bulk/columns.rs`, used by both `csv.rs` and the new
  `parquet_format.rs` — one column list, so the two formats can't drift
  apart.
- New dev-dependency `bytes` (reads Parquet bytes back in tests;
  `parquet::file::reader::ChunkReader` has no `std::io::Cursor` impl).
- SOUP register (`compliance/soup.tsv`) updated for the three new direct
  dependencies (`arrow`, `parquet`, `bytes`) per IEC 62304 §5.3.3.

### Added — bulk CSV wiring + keyless-row review-queue routing (2026-08-02)

- **CSV is now a full peer of JSONL** on `POST /api/persons/import` and
  `POST /api/persons/export` (`format: "csv"`): the `bg_pg` worker
  dispatches on the job's stored format, and stored artifact filenames
  carry the matching extension.
- **A keyless import row** (no strong identifier, no `tax_id`, no
  explicit `id`) now runs through the same duplicate detection
  `POST /check-duplicates` uses instead of a blind create. A likely
  duplicate still creates the row — a bulk load never silently withholds
  legitimate data — and queues the pair in the stored review queue with a
  new `provenance = "import"` column, so it's visible to an operator
  rather than only surfacing on a later batch scan.
- New `review_queue.provenance` column (migration
  `2026080200000001_review_queue_provenance`; existing rows backfilled
  `operator`).

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16.4 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. Feature renames applied: `auth_jwt` → `auth`,
  `bg_pg` → `worker`.
- **22 raw `Statement` call sites** across `src/compliance/erasure.rs`,
  `src/bulk/pipeline.rs`, `src/db/audit.rs`, `src/db/review_queue.rs`,
  `src/api/rest/handlers.rs`, and two test files move from
  `.execute`/`.query_one`/`.query_all` to the `_raw` variants — the
  largest count in the family so far, reflecting how much hand-rolled
  SQL this crate's audit and bulk paths carry.
- **`sea-orm`'s `BigDecimal` support needs an explicit feature.** Three
  match-score columns (`gender_score`, `address_score`,
  `identifier_score`) are stored as `bigdecimal::BigDecimal`; sea-orm
  2.0 no longer implements `ValueType`/`Nullable` for it without
  `with-bigdecimal` enabled (worked before only because something else
  pulled the impl in transitively under 1.1's default feature set).
  Added it explicitly.
- **A pre-existing missing import surfaced.** The `audit_log` SeaORM
  module imports its dependencies explicitly (`use super::{...}`)
  rather than glob-importing the prelude like every sibling module in
  the same file — and was missing `EntityTrait`. sea-orm 1.1's
  `DeriveEntityModel` expansion tolerated this; 2.0's does not. Every
  other module in `src/db/models.rs` already had it; this one just
  hadn't been exercised by a macro version that cared.
- **`Worker::perform_later()` now returns `Result<String>`** (the job
  id) instead of `Result<()>`. One call site in `src/bulk/handlers.rs`
  matched on `Ok(())`; changed to bind and ignore the id.
- A `useless_conversion` in `src/db/outbox.rs` from a now-redundant
  `.into()`.
- No behavioural change; verified with the full DB-gated suite (40
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-1, with this crate as the axum-style reference (case is the
loco-style one).

- **One reloadable verifier.** The PASETO verifier moved out of
  `AppState` into a process-wide `ReloadableVerifier` that the blanket
  guard **and** the `AuthUser` / `MaybeAuthUser` extractors read per
  request. It used to be an `Arc<Verifier>` snapshot taken at router
  construction and copied into the enforcement middleware — two
  snapshots that a rotation could only ever update one of. `AppState`
  no longer carries a verifier at all, so there is nothing to fall out
  of step.
- **`spawn_key_refresh`** re-fetches `PERSON_PASETO_KEYS_URL` every
  `PERSON_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset) and swaps the result in, so a key rotation at
  the auth service is picked up **without a restart**. A failed fetch
  **keeps the current key set** — a transient auth-service outage must
  not lock every caller out.
- **`policy()` is a `ReloadablePolicy`**, with `reload_policy()` and
  **`spawn_policy_watcher`** polling `PERSON_ABAC_POLICY_FILE`'s mtime
  every 15 s. An operator can edit the policy file and have it take
  effect immediately; a malformed edit falls back to the built-in
  default rather than leaving the service unprotected.
- **`tests/enforcement.rs`** — the activation proof, in its own binary
  because the auth `OnceLock`s are process-wide. With
  `PERSON_REQUIRE_AUTH=1` over the real router: public paths stay open,
  a protected read and write without a token are `401`, a malformed
  bearer is `401` (not a 500), a valid token with no attributes reads
  `200` and writes `403` — the 401/403 split the ABAC contract requires
  — and `access=write` creates. Mutation-checked: forcing the flag off
  fails it.
- New environment variable: `PERSON_PASETO_KEYS_REFRESH_SECS`.


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

### Added — stored review queue + decision endpoints (2026-07-19)

- `review_queue` table (migration `m20260719_000001_create_review_queue`):
  the batch-dedup scan persists its candidate pairs (normalized pair
  order, UNIQUE upsert — re-scans refresh scores, decided rows keep
  their decision, ids stay stable) and the scan response now reports
  the **stored** rows.
- `GET /api/persons/review-queue[?status=&limit=]` — list the stored
  queue (newest first, cap 500).
- `POST /api/persons/review-queue/{id}/decision`
  (`{"status": "confirmed" | "rejected"}`) — decide a `pending` item;
  first-writer-wins in SQL, `404`/`422` on unknown/already-decided.
- Each decision writes a `review_decision` audit row (actor = verified
  bearer `sub`, else `system`).

### Added — bulk CSV codec (BLK-1)

- A CSV codec for bulk import/export (`src/bulk/csv.rs`) alongside the
  lossless JSONL reference, following the family-wide §5 flattening
  convention: scalars → one column each; the primary name (single nested
  object) → dotted columns (`name.family`, …); arrays / arrays-of-objects
  → a single JSON-encoded cell (`identifiers`, `telecom`, `addresses`,
  `links`, `name.given`, …). `encode(&[Person]) -> Vec<u8>` writes a header
  + one row per person; `decode(&[u8]) -> Vec<serde_json::Result<Person>>`
  matches columns **by header name** (operator-reordered / extra columns
  tolerated) and returns a **per-row** result (§7 — a malformed row is an
  `Err` in its slot, not a whole-file abort). It **round-trips losslessly**
  against the wire type — `decode(encode(p)) == p` — proven by unit tests
  over a fully-populated person, a sparse person, reordered/extra columns,
  a bad-JSON-cell per-row error, and multi-row. Person's exact column set is
  declared in spec §10.6. Adds the `csv` crate. Wiring the codec into the
  async `bg_pg` import/export pipeline (the `format` dispatch + export
  handler) + keyless-row → review-queue routing is the follow-up (BLK-2).

### Added — matcher-partition guard test (cross-service-linking §7)

- A bridge test (`tests/duplicate_detection.rs::links_are_not_a_matcher_signal`)
  pins the partition rule: cross-service links are **never** a matcher
  signal. Cross-service `entity_links` are structurally excluded (their own
  table, never a field on the domain `Person`, so they never reach
  `to_matcher_person`), and the adapter also ignores the within-entity
  `Person.links`. The test adds link data to a record and asserts its match
  score is unchanged — a regression guard so a future edit that routed any
  link into the matcher input fails here. Closes the spec §13 T-9 partition
  acceptance box.

### Added — cross-service `linked` / `unlinked` events (LNK-1)

- Person now **emits** its cross-service link events on the durable event
  envelope (previously deferred). `EventKind` gained `Linked` / `Unlinked`,
  and `Envelope` gained an **additive** `data: Option<Value>` field
  (`skip_serializing_if = "Option::is_none"`, so the existing CRUD/merge
  wire shape is byte-identical) carrying the §4.2 edge detail (`edge_id` /
  `from_ref` / `to_ref` / `edge_kind` / `role` / `confidence` /
  `provenance` / `valid_from` / `valid_to`) that the link-graph aggregator
  deserialises into its `LinkedEvent`.
  - `POST /api/persons/{id}/links` emits `linked`; `DELETE …/{link_id}`
    emits `unlinked`. Under `PERSON_EVENT_TRANSPORT=outbox` the edge upsert
    (or soft-delete) and its event are enqueued in **one transaction** —
    the outbox guarantee (no committed edge without its event); under
    `memory` (dev) the in-memory `PersonEvent::Linked`/`Unlinked` is
    published as a lossy signal (it carries only the two `Uuid`s).
  - Tests: `kind_tokens_match_the_serde_form` (linked/unlinked),
    `crud_envelope_omits_data_on_the_wire` (frozen CRUD shape),
    `for_link_carries_edge_detail_data` (the aggregator seam), and a
    DB-gated `linked_event_is_enqueued_to_the_outbox`. (Repo tasks.md LNK-1.)

### Added — cross-service affiliation edges (LNK-3)

- The person link endpoints now originate the **`works_at` / `member_of`**
  affiliation edges (person → organization, temporal) in addition to the
  `same_identity` backbone. `validate_edge`'s permit set went from
  `same_identity`-only to `{same_identity, works_at, member_of}`, relying on
  the shared `entity-ref` registry's `EdgeKind::permits` for the endpoint
  check — so `works_at`/`member_of` require an **organization** target and
  `employed_by` (worker-originated) / `subject_of` (case-originated) are
  still rejected on the person side. No schema or endpoint change (the
  `entity_links` table + `POST`/`GET`/`DELETE /api/persons/{id}/links` +
  bulk pull are unchanged and already generic over kind). Accept/reject
  matrix unit-tested (`accepts_works_at_and_member_of_person_to_org`,
  `rejects_affiliation_to_non_org`, `rejects_kinds_person_does_not_originate`).
  (Repo tasks.md LNK-3.)

### Added — cargo-fuzz harness (SEC-I2 / SEC-B2)

- A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate with a
  `parse_line` libFuzzer target over the bulk-import JSONL parsers
  (`bulk::jsonl::split_lines` / `split_lines_capped` over raw upload bytes,
  and `parse_line` over the whole blob plus each split line). These turn
  **attacker-supplied uploaded file bytes** into `Person` records before any
  validation, so the target pins that the slice-then-deserialize path never
  panics on hostile input — complementing the existing
  `parse_line_never_panics` proptest with coverage-guided search. Run on
  nightly: `cargo +nightly fuzz run parse_line` (see `fuzz/README.md`). The
  `fuzz/` crate is standalone (not a workspace member), so it never affects
  the crate's normal stable build/test/clippy. Verified: `cargo +nightly fuzz
  build` compiles it and a short campaign runs clean (173k execs, no panics).

### Security

- **SEC-G3: record-level read authz on `search_persons`.** The person
  search results were masked **only** by the client-supplied
  `mask_sensitive` query param — so a deployment whose ABAC policy grants a
  *masked* read (a `mask` obligation) was bypassed by simply omitting the
  param, and a policy that *denied* the read had no effect on search. An
  aggregate read could therefore reveal more than the equivalent single
  `GET /api/persons/{id}` (violating `agents/share/security.md` invariant
  #5, "masking on every read path"). `GET /api/persons/search` now applies
  record-level authorization to **every** hit via the new
  `auth::read_visibility` (= `authorize_record(Read).ok()`, the same idiom
  the case service uses): a record the caller may not read is **omitted**
  from the page (concealed — its existence never leaks, rather than
  `403`-ing the whole page), and a `mask` obligation returns the masked
  view even when the client did not request masking. The `mask_sensitive`
  convenience still masks on request. It is a **no-op when
  `PERSON_REQUIRE_AUTH` is off** (the shipped default), so behaviour is
  unchanged until enforcement is activated. The per-result
  omit/mask/full decision is the pure `search_result_disposition`,
  unit-tested across the matrix (denied ⇒ omit; readable + `mask`
  obligation ⇒ masked without the client param; readable + no obligation ⇒
  full unless the client asked). (Repo tasks.md Phase 5 SEC-G3.)

- **SEC-G8: default-off exposure pin.** A new unit test
  (`default_off_exposes_sensitive_reads_activation_is_a_release_gate`)
  documents explicitly that with `PERSON_REQUIRE_AUTH` off (the shipped
  default) the most sensitive reads — a person's PII, the GDPR export, the
  audit trail, and the `same_identity` cross-service links — are **open
  without a token**. This exposure is by design
  (`agents/share/security.md` §4), but the test pins it so activation is
  understood as a **tracked release gate** and the default cannot be flipped
  to "secure" silently by assumption.

- **SEC-G7: bound the `search_persons` pagination offset.** `GET
  /api/persons/search` asked the search engine for `offset + limit` hits
  with an **unbounded** `offset`, so a caller passing a huge `offset` forced
  the index to materialise arbitrarily many results (a CPU/memory `DoS`),
  and `offset + limit` could overflow. The handler now rejects
  `offset > MAX_SEARCH_OFFSET` (10 000) with `400 OFFSET_TOO_LARGE` **before**
  searching, and computes the total with `saturating_add`. Deep pagination
  beyond the cap is unsupported (narrow the query / use cursor pagination).
  Pure `search_offset_within_bound` unit-tested; DB-gated integration test
  asserts the `400`.

- **SEC-M1: input-size caps on the `Person` payload.** The validator
  enforced format/required rules but capped no field's *size*, so a single
  multi-megabyte text field or a huge array could be a CPU/memory `DoS`
  against the matcher's O(n·m) Jaro-Winkler / Levenshtein / Jaccard scoring,
  amplified across the `check-duplicates` / `deduplicate` scan.
  `validate_person` now also bounds every scalar text field
  (`MAX_TEXT_LEN = 1024`), string-array cardinality + per-entry length
  (`MAX_ARRAY_LEN = 256` / `MAX_ITEM_LEN = 512`), and the inner text +
  cardinality of the nested collections (names, `additional_names`,
  `identifiers`, `addresses`, `telecom`, `documents`,
  `emergency_contacts`, `photo`, `tax_id`, `marital_status`) — returning
  field-scoped `422`s *before* the record is stored or matched. The caps
  are factored into `person_size_caps` / `cap_*` helpers. Unit tested
  (oversized text / array / array-item + a within-caps large record
  accepted).

- **SEC-B10: write the person merge audit in-transaction.** `merge` wrote
  its `UPDATE` (survivor) and `DELETE` (duplicate) audit rows **after**
  `txn.commit()`, best-effort — a crash between the commit and the audit
  writes left a durable merge with **no audit trail** (and an audit-insert
  error was only logged). The audit rows are now written on the merge
  transaction (`AuditLogRepository::log_update_on` / `log_delete_on`, new
  connection-generic helpers) **before** commit, so the merge and its audit
  commit atomically; an audit failure now rolls the whole merge back. The
  survivor's new-value snapshot is its applied payload (matching
  `apply_update_rows`). *Test (DB-gated):* after a merge, the survivor
  `UPDATE` and duplicate `DELETE` audit rows are present.

- **SEC-B9: wire the idempotency key so a retried submit dedupes.** The
  `bulk_jobs` table carried a `UNIQUE (entity, kind, idempotency_key)`
  constraint, but both submit handlers hardcoded `idempotency_key = None`,
  so it never fired — a client that retried a bulk submit (network blip,
  proxy retry) silently ran the import/export **again**, duplicating work
  and, for imports, re-processing every row. Now `POST
  /api/persons/import` and `/export` read an **`Idempotency-Key`** request
  header; `create_or_get_idempotent` returns the original job (no re-store,
  no re-enqueue) when the key already names one, and the unique constraint
  backstops the check-then-insert race (on violation the winner is
  re-fetched). A blank key is treated as absent; a key-less submit always
  creates. *Tests:* DB-gated same-key re-submit ⇒ same job id / one row /
  not re-run; key-less ⇒ always distinct; pure key-trim/blank test.

- **SEC-B8: bulk audit gaps — job-level import audit + fail-closed export.**
  A bulk import wrote **no job-level audit row** (only per-row create/update
  audit), and the export audit was **best-effort** — `log_export` errors
  were swallowed and the artifact was delivered anyway, so a bulk extract of
  personal data could complete with no audit trail. Now:
  - a successful import writes a job-level `IMPORT` audit row
    (`AuditLogRepository::log_import`) carrying the acting operator (from the
    job's `actor`) and the reconciled counts / dry-run flag;
  - the export audit is written **before** the job is finished and its error
    **propagates**, so a failed audit marks the job `failed` and the
    `download_url` is never surfaced — the extract is not retrievable
    without its audit trail;
  - the acting operator is threaded into both audit rows (falling back to
    `system` only when the job had no authenticated caller).
  The audit summaries (`import_audit_summary` / `export_audit_summary`) are
  pure and unit-tested (actor, counts, filter, masking profile present).
  Threading the real actor into each **per-row** create/update audit
  (currently a default `system` context) needs a repository-signature change
  and remains a follow-up.

- **SEC-B3: serialise bulk upsert to close a create-create race.** The
  import pipeline did a SELECT-then-INSERT with no locking, so two
  concurrent importers of the same stable key both missed in
  `find_existing` and both `create`d — duplicating the record. A
  `UNIQUE(system,value)` is the wrong tool (the registry intentionally
  permits duplicate identifiers — dedup is a workflow). Instead, the per-row
  find→create/update now runs under a **transaction-scoped advisory lock**
  on the stable key (`pg_advisory_xact_lock(hashtext(key))`,
  `import_upsert_locked`): a second importer of the same key blocks until
  the first commits, then observes the just-written record and upserts it in
  place. Dry-run classification stays lock-free (it commits nothing).
  *Test (DB-gated):* two concurrent imports of one SSN key ⇒ exactly one
  distinct person owns the identifier, one create + one upsert; plus a pure
  test that the lock-key string is collision-free across kinds/boundaries.

- **SEC-B4: bulk artifact hardening — path confinement, IDOR, and TTL.**
  Three holes in the bulk job/artifact surface:
  - **Arbitrary file read.** `LocalFsArtifactStore::get` stripped a
    `file://` prefix and read **any** absolute path, so a crafted
    reference (`file:///etc/passwd`) or a `..`-escaping key could read
    outside the store. `get` now resolves and **confines** the path to the
    store's canonicalised base (rejecting escapes), and both `put` and
    `get` validate keys with `is_safe_key` (no `..`, absolute, or drive/
    backslash components).
  - **IDOR / BOLA on job status.** `GET /api/persons/import/{id}` and
    `/export/{id}` returned **any** job by id — including its
    `download_url` / `errors_url` — to any caller. The status handler now
    takes the caller and returns `404` unless the caller **owns** the job
    (`is_job_owner`: the job's `actor` equals their token `sub`) or is
    **elevated** (an `access=admin` / `svc=true` token the ABAC policy
    would allow a `destructive` action). Off by nature when
    `PERSON_REQUIRE_AUTH` is off (no identity to check).
  - **No retention.** Jobs were created with `expires_at = NULL`, so an
    export of personal data was retrievable forever. `create` now stamps
    `expires_at = created_at + BULK_ARTIFACT_TTL_SECS` (7 days) and the
    status handler treats an expired job as `404` (`artifact_expired`), so
    a stale download URL is never handed out. Physical artifact deletion
    (an object-store sweep) is a follow-up.
  Pure cores (`is_safe_key`, `is_job_owner`, `artifact_expired`,
  store-confinement) are unit-tested, including the outside-the-base
  `file://` refusal.

- **SEC-B2: bound bulk import/export against an OOM DoS.** `POST
  /api/persons/import` read the whole multipart upload into memory
  unbounded (`field.bytes()`), the pipeline materialised every row before
  processing, and `export` had no ceiling on the requested `limit` — an
  oversized or unbounded (chunked) upload, or a giant export, could exhaust
  memory. Now:
  - the upload is read **chunk-by-chunk** and rejected with `413 Payload
    Too Large` the moment the running total exceeds `MAX_IMPORT_BYTES`
    (64 MiB), so it is never fully materialised (`read_field_capped` /
    `exceeds_cap`, unit-tested boundary incl. saturating-add overflow);
  - the import pipeline rejects a load whose non-blank row count exceeds
    `MAX_IMPORT_ROWS` (1,000,000) via `jsonl::split_lines_capped`, marking
    the job `failed` before any per-row database work;
  - a caller-supplied export `limit` is clamped to `MAX_EXPORT_ROWS`
    (1,000,000) at the worker's param mapping and again in the pipeline's
    listing path (`clamp_export_limit`).
  - **Fuzz:** proptest pins that the JSONL parse boundary (`parse_line` /
    `split_lines` / `split_lines_capped`) never panics on arbitrary
    strings, random bytes (incl. invalid / truncated UTF-8), or a
    pathologically long single line.
  True end-to-end streaming (never buffering the whole file, so the caps
  can rise) remains a follow-up; the caps make the current buffered path
  safe. (`proptest` added as a dev-dependency.)

- **SEC-G6: trailing slash can no longer downgrade a destructive POST.**
  `derive_action` classified `/merge` / `/deduplicate` / `/import` via
  `path.ends_with`, so a trailing slash (`POST …/merge/`) fell through to
  `Write` — a non-admin `access=write` caller could reach a destructive op.
  The path is now `trim_end_matches('/')`-normalised first. Test extended.

- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance every relay would select and
  **double-ship** the same rows. `drain_once` now runs in a transaction and
  `outbox::Model::unpublished` claims rows with `FOR UPDATE SKIP LOCKED`, so a
  second relay skips the locked rows (released on commit; unpublished rows
  retry next tick). Delivery stays at-least-once (consumers dedupe on
  `event_id`).

- **SEC-G4: escape `LIKE` wildcards in the repository name search.** The
  fallback `search` (`db/repositories.rs`) built its pattern as
  `format!("%{}%", query.to_lowercase())` with no escaping, so a query of
  `%` matched every row and `_`×N forced expensive scans (wildcard
  injection / DoS — the value was already a bound parameter, so not SQL
  injection). It now escapes `\`/`%`/`_` via a new `escape_like` helper
  before wrapping in the contains-pattern. Unit test
  `escape_like_neutralises_wildcards`.

- **SEC-B5: reject self-merge and lock merge participants (TOCTOU).**
  `POST /api/persons/merge` had **no self-merge guard**: `main ==
  duplicate` applied the survivor and then soft-deleted the *same* id,
  tombstoning the record and destroying its data — now rejected with `422`
  before any fetch (integration test `test_merge_into_self_is_rejected`).
  Separately, the merged payload was computed from **unlocked** reads, so
  two concurrent merges of the same duplicate could both see it active and
  fan its data into different survivors; the repository merge transaction
  now locks both participant rows `FOR UPDATE` (in id order, so opposing
  merges can't deadlock) and re-checks the duplicate is still active,
  making the loser fail closed instead of double-applying.
- **SEC-G1: authorise + audit the governed bulk-links read.**
  `GET /api/persons/links` returned **every** `same_identity`
  (person → worker) edge — identity-linking PII — with only the coarse
  blanket-read gate and no audit. It now authorises the cross-person dump
  as a privileged governed read (`authorize_record(Action::Destructive,
  …)`, which the default policy admits only for `svc=true` peers or
  `admin`) and writes an audit row on every surfacing. Unit test pins the
  `Destructive` classification (a downgrade to `Read` would reopen the
  leak); the full 401/403/200 behaviour is proven e2e on the case service's
  identical gate (`bulk_links_requires_elevated_authority`).

### Added — bulk export: rollout step 3 — masking + gating + audit (2026-07-10)

- Bulk **export** now honours the §8 privacy contract
  ([bulk-import-export.md](../../agents/share/bulk-import-export.md) §8):
  a `masking_profile` (`masked` default / `full`) and an
  `include_soft_deleted` flag (default `false`), plus a per-export audit
  row on every run. Import, CSV, and Parquet are untouched (other steps).
- New `bulk::MaskingProfile` (`masked` | `full`, default `masked`, wire
  tokens `masked`/`full`). `ExportParams` gains `masking_profile` and
  `include_soft_deleted`.
- `process_export_job` now maps every record through
  `privacy::mask_person` under the default `Masked` profile before
  encoding, so a default export never reveals more than the masked read
  view; `Full` leaves records unmasked. It returns the row count for the
  audit and **rejects** `include_soft_deleted=true` as
  `Error::Validation` ("not yet supported") — the repository cannot
  express a soft-deleted listing without a larger change, so the flag is
  refused rather than leaked or silently ignored.
- `POST /api/persons/export` accepts `masking_profile` (default `masked`)
  and `include_soft_deleted` (default `false`); an unknown profile token
  is a `400`. The **privileged** paths (`full` OR `include_soft_deleted`)
  are gated behind elevated authorisation via person's existing
  record-level guard (`auth::authorize_record` with a `destructive`
  action): a no-op when `PERSON_REQUIRE_AUTH` is off, else `403` unless
  the ABAC policy allows it (`access=admin` / `svc=true` by default). The
  default masked, active-only export stays open to any authorised caller.
- The export audit row (worker `audit_export`) now records actor, the
  filter (`q`/`limit`/`offset`), format, masking profile,
  `include_soft_deleted`, and the row count — written even for a zero-row
  export via the new `AuditLogRepository::log_export` (`EXPORT` action).
- Tests: DB-free unit — masking applied for `Masked` / skipped for `Full`
  (`apply_masking`), the privileged-path gate decision
  (`export_requires_elevation`), and `MaskingProfile` round-trip;
  DB-gated `#[ignore]` — a default export returns masked JSONL and writes
  an `EXPORT` audit row, a `Full` export returns unmasked, and
  `include_soft_deleted=true` is rejected.

### Added — bulk import/export: rollout step 1 (2026-07-10)

- Person is the **reference entity** for the family-wide bulk
  import/export capability
  ([bulk-import-export.md](../../agents/share/bulk-import-export.md) §3–§7,
  §10; rollout step 1). Async, job-based, driven by a Postgres-backed
  background worker (`bg_pg`); **JSONL** is the lossless reference format.
- New `bulk_jobs` table (migration `m20260710_000002_create_bulk_jobs`;
  `UNIQUE(entity, kind, idempotency_key)`), SeaORM entity
  (`db::models::bulk_jobs`), and persistence helpers (`db::bulk_jobs`:
  `create`/`set_input_url`/`set_status`/`finish_import`/`finish_export`/
  `find_by_id`/`list_recent`).
- New `bulk` module: `store` (the `ArtifactStore` trait +
  `LocalFsArtifactStore` for dev/test, `PERSON_BULK_ARTIFACT_DIR`; S3 is
  the deployment backend, deferred), `jsonl` (streaming codec — one
  person wire record per line), `stable_key` (person's upsert key —
  §10.1: a strong scheme-scoped identifier (SSN/TAX/NPI/PPN) → `tax_id` →
  record `pid`), `error_report` (§7 per-row `row_number/field/code/message`
  → CSV), `pipeline` (the testable `process_import_job` /
  `process_export_job` core), and `worker` (the `BulkJobWorker` adapter,
  registered in `connect_workers`).
- Import (§6): per row parse → validate (the single-create validators, so
  the same `422` reasons) → **upsert in place** when the stable key
  matches an existing record (idempotent re-import), else create; invalid
  rows are skipped into the downloadable error report, never aborting the
  load; each written row emits its normal event + audit via the repository.
- Export (§8): honours the person list/search filter, streams matching
  records to a JSONL artifact, and writes an export audit row.
- Endpoints (`bulk::handlers`, mounted on `persons_routes`, in OpenAPI):
  `POST /api/persons/import` (multipart, `202 {job_id}`, `dry_run`
  supported; a declared destructive POST),
  `POST /api/persons/export` (JSON filter, `202 {job_id}`),
  `GET /api/persons/import/{id}` + `GET /api/persons/export/{id}` (status +
  counts + `errors_url`/`download_url`), `GET /api/persons/bulk-jobs`.
- Tests: DB-free unit (JSONL round-trip, stable-key precedence,
  error-report shape, store round-trip, enum round-trips — 16 tests) plus
  DB-gated `#[ignore]` pipeline tests (create-then-idempotent-upsert with
  error report, dry-run commits nothing, export JSONL round-trip).
- **Deferred** (rollout steps 2–5, noted not built): CSV + Parquet
  formats, export masking profiles + `include_soft_deleted` gating,
  keyless-row → duplicate-review routing, S3 artifact store, other
  entities.

### Added — cross-service links: `same_identity` write side (2026-07-10)

- Person is the **reference originator** of the cross-service
  `same_identity` (person ↔ worker) backbone edge
  ([cross-service-linking.md](../../agents/share/cross-service-linking.md)
  §4.1/§4.2/§9, rollout step 2). New `entity_links` table (migration
  `m20260710_000001_create_entity_links`; `UNIQUE(from_pid, kind, to_ref,
  valid_from) NULLS NOT DISTINCT` for idempotent upsert), SeaORM entity
  (`db::models::entity_links`), and persistence (`db::entity_links`:
  `upsert` — idempotent, revives a soft-deleted row; `list_active`;
  `find_active`; `list_all_active(since)`; `soft_delete`).
- Endpoints (`api::rest::links`, mounted on both router surfaces):
  `POST /api/persons/{id}/links` (validate → upsert → best-effort audit),
  `GET /api/persons/{id}/links`, `DELETE /api/persons/{id}/links/{link_id}`,
  and the aggregator's reconciliation pull
  **`GET /api/persons/links[?since=<rfc3339>]`** returning
  `{ "edges": [EdgeDetail…] }` in the canonical §4.2 shape (`edge_id` /
  `edge_kind` / `from_ref = person:<id>`). Depends on the shared
  `entity-ref` crate.
- Validation (`validate_edge`, DB-free, unit-tested): accepts **only**
  `same_identity` person → worker; `422` for a non-`same_identity` kind
  (`subject_of` / `works_at`), a `same_identity` to a non-worker, an
  unknown kind, or a malformed `to_ref`. Writes are authorised at the
  person record-level (`authorize_record`) and audited (`person_link`
  create/delete).
- **Deferred:** cross-service `linked`/`unlinked` **event** emission —
  neither the durable `Envelope` (no link kind / no `data`) nor the
  in-memory `PersonEvent::Linked` (person `Uuid`s only) can carry the
  §4.2 edge `data` without a cross-cutting refactor; the bulk endpoint is
  the aggregator's sync path (§8). Worker's symmetric side is the
  follow-up.

### Added — authz: record-level resource attributes + obligations (2026-07-05)

- Record-level ABAC (verifier 0.3 → 0.6). Beyond the coarse blanket
  guard, `GET`/`PUT`/`DELETE /api/persons/{id}` run a second, finer
  decision after loading the record: `auth::person_resource_attrs`
  derives `resource.active` / `resource.deceased` / `resource.managing_org`
  and `auth::authorize_record` calls `Policy::evaluate_with_context`
  (gated on `PERSON_REQUIRE_AUTH`, a no-op when off). `PUT`/`DELETE`
  evaluate the **stored** record. A deployment can thus write e.g.
  "deny write on a deceased person's record unless `access=admin`".
- Also supplies **environment attributes** (`env.hour` / `env.after_hours`,
  UTC, via `auth::request_env_attrs`) and honours the **`mask`
  obligation** on `GET` (returns `mask_person`). New `auth::MaybeAuthUser`
  extractor + module-level `auth::policy()` / `require_auth()` accessors.
  DB-free tests for the resource-attribute mapping and the working-hours
  derivation.

### Added — authz: ABAC policy authorization inside the blanket guard

- ABAC authorization landed (spec §13 T-1c, the authorization sub-item
  — supersedes the earlier roles/RBAC-on-`roles`/`scope` sketch;
  family contract: `agents/share/authorization-attributes.md`). When
  `PERSON_REQUIRE_AUTH` is on, a verified PASETO token is further
  checked by the shared policy engine in `authentication-verifier`
  0.3: the request's action is derived from the HTTP method plus the
  crate's destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES`
  — `/merge`, `/deduplicate`, `/import`), and the policy is evaluated
  over the token's new `attrs` claim, first-match-wins, defaulting to
  allow-read / deny-mutation.
- New env vars `PERSON_ABAC_POLICY` (inline JSON) and
  `PERSON_ABAC_POLICY_FILE` (path), read once at router construction
  by the new `auth::policy_from_env` (restart to change); unset or
  unparsable ⇒ `tracing::warn!` + the built-in default policy
  (`svc=true` ⇒ everything; `access=admin` ⇒ destructive+write;
  `access=write` ⇒ write) — the service always boots.
- `auth::enforce` now takes the HTTP method and the policy and
  returns `403` (with the deciding-rule reason) for a valid token the
  policy denies; `401` remains missing/bad credential. `Enforcement`
  carries the policy alongside the verifier.
- DB-free unit tests pin the family §7 matrix: action derivation,
  empty-`attrs` read-only default, `access=write` / `access=admin` /
  `svc=true` tiers, deny-beats-later-allow, 401-vs-403, bad-policy
  fallback.
- Flag off ⇒ behaviour-neutral: no authn and no authz, exactly as
  before.

### Added — boot-time PASETO key-set fetch (`PERSON_PASETO_KEYS_URL`)

- New `PERSON_PASETO_KEYS_URL` env var (spec §13 T-1c fetch item): when
  set, the auth-service published Ed25519 key set
  (`/.well-known/paseto-keys`) is fetched **once at boot** via
  `Verifier::from_paseto_keys_url` (the `authentication-verifier`
  `fetch` feature, now enabled in Cargo.toml). On success the fetched
  key set **wins** over `PERSON_PASETO_KEYS` (logged at `info`); on any
  fetch failure (network / HTTP / parse) a `warn` is logged and the
  verifier falls back to the `PERSON_PASETO_KEYS` env path — the
  service **always boots**; auth-service downtime never prevents
  startup. Unset/blank URL ⇒ prior behaviour exactly. One-shot fetch —
  no refresh loop (periodic refresh is a spec §15 roadmap note).
- Wired in the loco `after_routes` hook: the verifier is resolved
  (`state::verifier_from_env_or_fetch`) and swapped into `AppState` via
  `with_verifier` **before** the enforcement middleware and the
  shared-store state are built, so both router surfaces (the
  enforcement layer and the `AuthUser` extractor) verify against the
  fetched key set. Issuer/audience still come from
  `PERSON_TOKEN_ISSUER` / `PERSON_TOKEN_AUDIENCE` (same defaults).
- New DB-free tokio tests in `src/api/rest/auth.rs` (reusing the
  in-process PASETO minting helpers): a local ephemeral-port HTTP
  listener serves the key set and a token signed by that key verifies;
  a dead port falls back to the env path without panicking; URL-unset
  uses the env path (precedence).
- Authorization has since landed as ABAC (see the top entry), not
  RBAC — the spec §13 T-1c authorization item is complete.

### Added — blanket auth enforcement (default-off)

- New blanket `/api/*` auth enforcement middleware (spec §13 T-1b; family
  contract: `agents/share/jwt-enforcement.md`). When `PERSON_REQUIRE_AUTH`
  is truthy (`1`/`true`/`yes`/`on`, case-insensitive; unset/blank/junk ⇒
  **off**, the default), every route requires a valid PASETO `v4.public`
  bearer token except the public allow-list: `/api/health`, loco's
  `/_health` / `/_ping`, `/api-docs/openapi.json`, `/swagger-ui*`, and
  `/metrics.prom`. Unauthorised requests get `401`.
- Implemented as a pure, DB-free `auth::enforce(flag, path, headers,
  verifier)` decision plus an `Enforcement` middleware state in
  `src/api/rest/auth.rs`, layered unconditionally on **both** router
  surfaces (`create_router` and the loco `after_routes` hook). The flag
  is snapshotted at router construction — changing the env var requires
  a restart; the flag is the only switch.
- New DB-free unit tests pin the full enforcement matrix (off + no
  token ⇒ Ok; on + each public path ⇒ Ok; on + protected + no token ⇒
  `401`; on + valid ⇒ Ok; on + expired/tampered ⇒ `401`) and the lenient
  flag-parser semantics, reusing the in-process PASETO minting helpers.
- Boot-time HTTP key fetch has since landed (see the entry above);
  authorization has since landed as ABAC (top entry), completing
  spec §13 T-1c.

### Changed — auth pivot: RS256 JWT/JWKS → PASETO v4.public

- Bearer-token verification migrated off RS256 JWT + JWKS to **PASETO
  `v4.public`** (Ed25519), per the family-wide design in
  `agents/share/authentication-sessions.md` (§5, §9 step 4; spec §13
  T-1a). The `AuthUser` extractor and `GET /api/whoami` are unchanged
  in shape; only the credential changes.
- `authentication-verifier` bumped from the crates.io `0.1` (RS256)
  release to the monorepo path dependency `0.2` (PASETO-only:
  `Verifier::from_paseto_keys_value`); the direct `jsonwebtoken`
  dependency is dropped.
- The verifier is now built from the environment at boot:
  `PERSON_PASETO_KEYS` (the Ed25519 key set the auth service publishes
  at `/.well-known/paseto-keys`), `PERSON_TOKEN_ISSUER` (default
  `authentication-service`), `PERSON_TOKEN_AUDIENCE` (default
  `main-x-service`). Absent/blank/unparseable key set ⇒ empty key set:
  every token is rejected but the service still boots.
- New DB-free unit tests in `src/api/rest/auth.rs` mint `v4.public`
  tokens in-process (throwaway Ed25519 key via `rusty_paseto` +
  `ed25519-dalek` dev-deps) and pin valid / missing / non-bearer /
  expired / tampered / no-key outcomes.

### Fixed — privacy masking UTF-8 safety

- `privacy::mask_value` is now char-based instead of byte-based. The
  previous implementation sliced the string at byte offset
  `len - visible_chars`; when that offset fell inside a multibyte UTF-8
  character it **panicked** (`end byte index … is not a char boundary`),
  so the masked-view endpoint (`GET /api/persons/{id}/masked`) would
  500 on any person whose tax ID, identifier, document number, or phone
  carried a non-ASCII character near the tail (accented names, non-Latin
  identifiers). Masking now counts Unicode scalar values and keeps
  exactly the last four *characters* visible. Pinned by
  `privacy::tests::test_mask_value_multibyte_does_not_panic`; the
  contract is recorded in spec §6.6.

### Added — matcher bridge

- New `src/matching/adapter.rs` exposing
  `to_matcher_person(&service::Person) -> person_matcher::Person`.
  Projects the FHIR/schema.org-shaped service record into the matcher
  crate's builder shape: name flattening (`HumanName` → flat
  `given_name`/`family_name`/`middle_name`), telecom sampling
  (first phone / sms / email of each system),
  identifier routing by FHIR-style `system` URI (UK NHS via `https://fhir.nhs.uk/Id/nhs-number` → `uk_nhs_number`, US SSN, 40+ country slots
  with type-based fallbacks), and address field
  renaming (`state` → `county`, `postal_code` → `postcode`).
- `src/matching/mod.rs` now re-exports the sibling `person-matcher` crate
  as `matcher_lib`, so callers can reach `MatchingEngine`,
  `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`, and
  every public matcher type without taking a separate dependency.
- Field-routing rules are inline-documented in `adapter.rs` and
  pinned by `tests/duplicate_detection.rs`.

### Added — tests

- New `tests/duplicate_detection.rs`. Black-box bridge tests that
  drive service records through `to_matcher_person` and assert on
  the canonical `MatchingEngine::match_persons` output. Covers
  identical clones, name typos (Jaro-Winkler), deterministic
  short-circuits (national / strong identifiers), negative cases
  (unrelated records, divergent demographics), per-adapter field
  routing, and config-preset invariants (strict ⊆ lenient).

### Added — bridge benchmarks

- New `benches/bridge_bench.rs` (Criterion). Three groups:
  `bridge_adapter_only` (projection cost on minimal vs. rich
  records), `bridge_end_to_end` (adapter + engine call), and
  `bridge_one_to_many` (single query vs. 10 / 50 / 100 candidates).
  Regression guard for the duplicate-check hot path.

### Added — observability

- New `src/metrics.rs` exposing a process-wide `LazyLock<Metrics>`
  Prometheus registry. Standard counters
  (`person_created_total` / `_updated_total` / `_deleted_total` /
  `_matched_total`, labeled `http_requests_total`) and histograms
  (`http_request_duration_seconds`, `person_match_score`,
  `person_search_duration_seconds`).
- New `GET /metrics.prom` route on the web router serving Prometheus
  text-exposition format (`text/plain; version=0.0.4`). The
  canonical `/metrics` continues to render the HTML dashboard;
  configure scrapers with `metrics_path: /metrics.prom`.

### Added — UI

- `assets/static/css/themes/` ships 39 standalone Lily Design System
  themes (light, dark, dracula, nord, cyberpunk, … + four
  United Kingdom NHS variants). The layout's theme picker now lists
  all 39; default is `light`. Selection swaps the `<link href>` of
  `<link id="lily-theme-css">` at runtime; persisted in
  `localStorage["lily-theme"]`. The command palette also lists all
  39 themes.

### Changed — Loco background jobs

- Dropped the `bg_redis` and `bg_sqlt` features from the `loco-rs`
  dependency. Background jobs are now backed exclusively by
  PostgreSQL (`bg_pg`), using the same database as application data
  — no external Redis broker. `config/development.yaml` and
  `config/production.yaml` updated to `queue.kind: Postgres` with
  `uri: DATABASE_URL`. Removes the `rusty-sidekiq` →
  `redis 0.22.3` future-incompat warning chain.

### Changed — documentation

- Reduced healthcare / clinical / patient / hospital / clinician /
  practitioner framing across spec.md, AGENTS.md, AGENTS/*, README,
  CLAUDE.md, and index.md. Preserved: FHIR R5 resource and field
  names (e.g. `Patient.birthPlace`, `Practitioner` resource),
  national-identifier proper nouns (United Kingdom National Health
  Service Number, Australia IHI), paper citations, the
  `compliance-for-healthcare.md` doc, and `HIPAA` / `NHS` / `PHI`
  as compliance regimes.
- `spec.md §11 Testing Strategy` now lists the bridge integration
  tests; `AGENTS/testing.md` gained a `## Bridge Integration Tests`
  section; `AGENTS/restful.md` gained adapter + Prometheus blocks;
  `index.md` gained a worked example showing the canonical bridge
  in action.

### Fixed

- The person-matcher crates.io 0.3.0 API drift (Sweden personnummer
  renamed from `se_personnummer` to `se_workernummer`,
  `united_kingdom_national_health_service_number` shortened to
  `uk_nhs_number`) is now caught at the matcher level by each
  matcher's `tests/adapter_contract.rs` — see the matcher
  CHANGELOG.
