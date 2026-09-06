# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added — `merged_from` on the `Merged` event envelope (umbrella spec §13 T-13)

`Envelope` had no way to name which duplicate a `Merged` event's
survivor absorbed — unlike the six person-style crates
(person/worker/place/thing/event/course), which all carry a dedicated
`merged_from: Option<String>`. Without it, the link-graph aggregator's
merge-repointing consumer has no way to know which edges to move off
the duplicate onto the survivor. Added `merged_from` to `Envelope`
(additive, does not bump `SCHEMA_VERSION`) and a `merge_envelope`
constructor that populates it; `merge_and_emit` now builds the
survivor's `Merged` event through it under both the `memory` and
`outbox` transports. New DB-free `src/streaming.rs` tests plus a new
DB-gated `tests/merge_event_carries_merged_from.rs` (own binary,
`outbox` transport) pin that the persisted `event_outbox` row's
payload carries the absorbed duplicate's pid. See spec §13.

### Fixed — FHIR `GET /fhir/Organization` search now resolves through the Tantivy index (ORG-T5)

The FHIR search handler ran a capped in-memory scan over `OrgModel::list`
even though the native `/search` endpoint already queries the Tantivy
index. A text-bearing param (`name`, `address`, `address-city`,
`address-postalcode`, `identifier`) is now resolved via
`SearchEngine::search`, the same full-text field set `/search` queries;
`FhirOrgSearchParams::matches` still runs on every candidate as the
authoritative, field-precise filter — only retrieval moved, not the
matching semantics. A bare-`_id` or fully empty request keeps the
original capped scan, since there is no text to search on.

### Fixed — `score_breakdown` was never actually surfaced on review-queue rows (ORG-T1)

The `deduplicate` scan hard-coded `score_breakdown: None` when storing
a new review-queue row, and the wire `ReviewQueueItem` had no field to
carry one back even if it had been stored — so `GET /review-queue`
could never show why a pair matched, forcing the front-end to call
`POST /organizations/match` a second time against the loaded pair for
a live breakdown. `deduplicate` now stores the matcher's real
`MatchBreakdown` (`serde_json::to_value(&r.breakdown)`, the same
pattern person-service uses); `ReviewQueueItem` carries it, and
`review_row_to_item` reads it back. A re-fetch of the queue now
answers "why did this match" with no second request.

### Added — URL well-formedness + ISO 3166 country-code validation (ORG-T4)

`src/validation.rs` previously only length-bounded `url`,
`address.country`, `jurisdiction`, and `same_as[i]` — never checking
they parse as a URL or a real country code. Added `is_valid_url` (the
same `http://`/`https://` scheme check every sibling entity crate
applies) for `url` and each `same_as[i]`, and `is_valid_country_code` +
a 249-entry `ISO_3166_1` alpha-2/alpha-3 const table for
`address.country` and `jurisdiction`. New unit tests plus two DB-gated
request-level `422` pins. See spec/index.md ORG-T4.

### Added — real OpenTelemetry OTLP export (PRO-H12 slice 4 of 7)

- **2026-08-30**: new `src/observability.rs` — this crate carried no
  observability module at all before this change, and is the first of
  the four remaining loco-idiomatic registries (organization,
  care-pathway, case, portfolio) to carry one; PRO-H12 slices 1–3
  (course, place, thing) were all person-style crates. Close port of
  course-service's, itself a port of person-service's, itself a port of
  link-graph-service's original reference: the `tracing-opentelemetry`
  bridge over an OTLP/gRPC `SdkTracerProvider`/`SdkMeterProvider`,
  export-on-by-default at `OTLP_ENDPOINT` (default
  `http://localhost:4317`) with `service.name` from `OTLP_SERVICE_NAME`
  (default `organization-service`). Real adaptation, confirmed rather
  than assumed: this crate has exactly **one** router-construction
  surface (`App::routes`/`App::after_routes` in `src/app.rs` — even its
  own request-level test suite boots the real `App` via loco's testing
  harness, not a second hand-rolled router), unlike the person-style
  crates' two, so `observability::trace_mw` is layered once, as the
  outermost middleware in `after_routes`. No `tonic` dev-dependency
  rename was needed: this crate declares no `tonic` dependency at all
  (no gRPC stub), so the in-process OTLP collector tests take a plain
  `tonic = "0.14"` dev-dependency, exactly as course's did.
  `tests/otlp_export.rs` + `tests/otlp_middleware.rs` +
  `tests/otlp_collector/` (ported from course-service) prove real
  export against a real in-process gRPC listener. Verified
  independently: `cargo fmt --check`, `cargo clippy --all-targets -D
  warnings`, `cargo deny check`, `cargo bench --no-run`, and the MSRV
  check (`cargo +1.96 check --all-targets`) all clean; `cargo test
  --lib` 198/198 (was 190, +8 new); `cargo test --test otlp_export
  --test otlp_middleware` 4/4. See spec §13.



### Added — TSV bulk import/export, and fuzzed row decoders

`BulkFormat` gains `Tsv`, accepted on the same `format` field as `jsonl`
and `csv` for both import and export.

TSV is CSV with a different delimiter, so it **shares the codec** rather
than forking it: `csv::encode`/`decode` take a `delimiter: u8`, and
`BulkFormat::delimiter()` is the single place that decides which byte a
format uses. A second place to decide it would be a second place for the
two to drift.

The delimiter is **passed in, never sniffed**. Reading CSV as TSV
resolves no column and reconstructs the row from nothing — which for this
entity surfaces as a per-row parse error, and for an all-optional entity
would be a silently empty record. Pinned by a test that asserts the data
is not recovered either way.

Also added: a `bulk_decoders` cargo-fuzz target driving `csv::decode`
under **both** delimiters plus the JSONL split/parse path over arbitrary
bytes. A bulk import is the one path that takes a whole file from a
caller, so its decoders are the outermost parser in the service. The
target pins never-panic, decode determinism, and the §7 per-row error
contract — a malformed row must not abort the load, because the good rows
are promised to commit.

### Added — cargo-fuzz harness for the request-path logic (FUZZ-2)

A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate with
three coverage-guided libFuzzer targets. Until now the harnesses covered
only the dependency-light libraries; the services had none, despite
carrying the surface that actually faces the network.

- **`validate_json`** — the real request path: arbitrary bytes →
  `serde_json` → `Organization` → `validation::problems`. Never-panic,
  deterministic, and a **bounded problem report**.
- **`validate_built`** — the validator driven directly, building the
  `Organization` from raw bytes so the fuzzer controls array cardinality and
  entry contents without first having to learn JSON. A run of NUL bytes
  becomes a run of blank entries — the exact SEC-M8 shape.
- **`merge_orgs`** — the merge fold over two arbitrary payloads:
  never-panic; the survivor keeps its `name`; deterministic; and
  **absorbing**, so a retried merge cannot inflate the record.

The sub-crate declares an empty `[workspace]` table: this crate is a
workspace root, so `fuzz/` would otherwise be pulled in as a member, and
a cargo-fuzz build needs its own sanitizer flags and lockfile. Nightly
only, so it is exempt from the repo MSRV. See
[`fuzz/README.md`](./fuzz/README.md).

### Fixed — an over-long array produced one `422` problem string per entry (SEC-M8)

`validation::problems` reported an over-long array's cardinality
violation once and then still walked **every** entry, so a payload with
ten thousand blank `keywords` or `identifiers` came back with ten thousand problem strings — which
the controller joins into a single `422` body. A small request bought a
large response. Worse here than a blank check: each entry also ran the
SEC-M5 check-digit validation for its declared scheme.

Every per-entry loop now walks a new `inspected()` helper, which yields
at most `MAX_ARRAY_LEN` entries. The cardinality problem already rejects
the payload, so inspecting the tail decides nothing; bounding the
**report** is the same input-bounding rule (SEC-M1) as bounding the work.
The helper is named rather than inlined at each call site so a per-entry
loop added later without it reads as different from the ones that have
it. Pinned by a test.

Case landed this first as the reference; this is the roll-out
(repo `tasks.md` SEC-M8b).

### Fixed — the search index built a new Tantivy writer on every write

`SearchEngine::index_organization` (and `delete_*` / `clear`) called
`self.index.writer(WRITER_HEAP_MB)` per call. Tantivy's `IndexWriter`
allocates its whole 50 MB arena and spawns merge threads on
construction, so **every create, update, merge, and soft-delete paid
that setup synchronously**, on the request path. Measured at ~155 ms per
indexed document against a fresh index; holding one writer for the
process brings it to ~78 ms, the remainder being the durable commit and
reader reload that indexing-on-write inherently costs.

It was also a concurrency hazard, not only a slow one: an `IndexWriter`
holds the index directory's exclusive lock, so taking and releasing it
per call left two simultaneous writes able to collide on it. One owner
for the process cannot.

The engine now holds a `Mutex<IndexWriter>` created in `new()`. A
poisoned lock recovers the guard rather than failing for ever — the only
operations held across it are `delete_term` / `add_document` / `commit`,
and a permanently dead index would be the worse outcome.

Found by the new benchmark, which is the point of having one.

### Added — Criterion benchmarks

- `benches/service_bench.rs`, covering the CPU-bound halves of a request
  — the part a database benchmark hides behind I/O. Three groups:
  **validation** (every create and update pays it; the `oversized_arrays`
  case exercises the SEC-M1 input caps, because rejecting an abusive
  payload has to be cheap or the caps are not doing their job),
  **merge** (a whole-record fold, with a scaling case showing the cost
  sits in the collections it unions), and **search** (indexing one
  document — what every write pays synchronously — plus exact / fuzzy /
  phonetic retrieval and the `candidates` blocking query a duplicate
  check actually calls, against a populated index).
- `criterion` is a new dev-dependency; test-only, so it is not in any
  release artefact.

### Added — declared MSRV (Rust 1.96)

- `Cargo.toml` now declares `rust-version = "1.96"`, the repository's
  **current stable minus two** floor
  (`spec/rust-msrv-n-minus-2/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.96 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption. *(Corrected 2026-09-06: this entry
  originally said 1.95 / N-3, matching the policy at the time it was
  written; the repository-wide MSRV policy has since tightened to N-2,
  and `Cargo.toml` already declares 1.96 — this entry is edited in place,
  since it was still `[Unreleased]`, rather than left to misstate the
  crate's actual floor.)*

## [0.1.0] - 2026-08-04
### Added — `seed_examples` CLI task (EX-4, 2026-08-04)

`cargo loco task seed_examples` loads the repository's shared demo
fixture (`examples/data/organizations.jsonl`, 20 rows) into the
`organizations` table, for the tutorials (`tasks.md` EX-4). Inserts
via the **model-layer create** (`models::organizations::Model::create`)
rather than `POST /api/organizations`, so the same seed path as person
and case, and no audit row or event is written by the seed itself.
Refuses to insert into a non-empty `organizations` table (prints a
message and exits cleanly), so a second run is a no-op rather than a
duplicate load. New `src/tasks/seed_examples.rs` (`parse_fixture`
reuses `bulk::jsonl::parse_line`, `seed`, `SeedExamples`); DB-free unit
tests parse the real fixture; a DB-gated `tests/seed_examples_db.rs`
proves a first run seeds all 20 rows and a second run changes nothing.

### Added — Durable event bus, real-broker sink (BUS-3, 2026-08-03)

`FluvioSink` (`src/relay.rs`) — the Phase-3 relay's real-broker
`EventSink`, ported from case-service's BUS-1 reference implementation,
behind this crate's own `fluvio` Cargo feature (off by default;
`fluvio` 0.50). One producer per topic, partitioned by record `pid` per
`agents/share/event-bus.md` §7. New env vars:
`ORGANIZATION_FLUVIO_ENDPOINT` (unset ⇒ unchanged `LoggingSink`
default) and `ORGANIZATION_EVENT_TOPIC` (default
`mxi.organization.events`). An endpoint configured without the `fluvio`
feature refuses to start the relay rather than silently falling back
to `LoggingSink` — that fallback would mark outbox rows
`published_at` without ever reaching the broker the operator asked
for. `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a local
SC+SPU broker for opt-in manual runs (not part of any automated CI
stage); `tests/fluvio_relay.rs` is a feature-gated, `#[ignore]`d
live-broker round-trip, verified by compiling under `--features
fluvio` rather than an actual execution (no automated run in this repo
stands up a live broker).

### Added — BLK-5 async bulk import/export (2026-08-03)

- **`POST`/`GET /api/organizations/import[/{id}]` and
  `export[/{id}]`, plus `GET /api/organizations/bulk-jobs`** — async,
  loco-worker-driven bulk import and export
  (`agents/share/bulk-import-export.md`), scoped to what BLK-1/BLK-2
  need: **JSONL + CSV only** (no Parquet) and a **local-filesystem-only**
  artifact store (no S3 backend; the trait is async so a future S3
  backend needs no signature change).
- New `src/bulk/` module: the wire "bulk row" shape (an organization's
  own fields plus an optional top-level `pid`, since
  `organization_matcher::Organization` carries no id of its own), the
  JSONL/CSV codecs, the stable-key resolver (LEI → DUNS → explicit
  `pid`; a keyless row runs the same search-blocking + matcher
  duplicate detection `POST /check-duplicates` uses and is queued in
  the review queue with `provenance = "import"`), the per-row error
  report, the pipeline (reuses `streaming::create_and_emit`/
  `update_and_emit` for every written row — a bulk-imported
  organization gets the same event/audit/search-index side effects as
  one created interactively), the local artifact store, the
  `BulkJobWorker`, and the REST handlers.
- New `bulk_jobs` table (`m20260803_000002_bulk_jobs`) and a
  `review_queue.provenance` column
  (`m20260803_000001_review_queue_provenance`, mirroring person's
  `m20260802_000001`).
- Export defaults to the masked view (`crate::privacy::mask_organization`);
  the privileged `full` profile requires elevated authorisation.
  `include_soft_deleted=true` is `400` (not yet supported). Every
  export is audited, and the audit write gates delivery (SEC-B8): a
  failed audit write fails the job before the artifact is stored.
- **Known limitation:** the per-row upsert is not wrapped in a SEC-B3
  stable-key advisory lock, unlike the family reference pattern — see
  spec §10.7 "Concurrency" for why (a lock held on a separate guard
  transaction deadlocked every import under this crate's own
  `config/test.yaml` `max_connections: 1`, since
  `streaming::create_and_emit`/`update_and_emit` are hard-coded to
  `&DatabaseConnection` rather than generic over `ConnectionTrait`).
  Two importers racing the identical stable key in the same instant can
  both create a row; closing this is a tracked follow-up.
- 8 new request-level tests (`tests/requests/bulk.rs`, Postgres-gated)
  plus DB-free unit tests throughout `src/bulk/`.

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**, the framework's first stable release: sea-orm
  1.1 → 2.0, sea-orm-migration → 2.0, sea-query → 1.0. Mechanical
  fallout: raw `Statement` queries in `models/review_queue.rs` move from
  `.execute`/`.query_one`/`.query_all` to the `_raw` variants (sea-orm 2.0
  splits typed `StatementBuilder` calls from raw-SQL ones); a
  `useless_conversion` in `models/event_outbox.rs` from a now-unneeded
  `.into()`.
- **loco's `ColType::PkAuto` now generates a 64-bit primary key**
  (`BIGINT`, was `SERIAL`). The `organizations`, `audit_logs`, and
  `merge_records` generated entities (and the compliance-report /
  test-fixture code that carries their row ids) move from `i32` to
  `i64` to match. `event_outbox` is unaffected — its migration writes
  raw SQL (`id SERIAL PRIMARY KEY`) rather than the loco schema DSL,
  specifically to control the exact table name, and that raw SQL was
  left as `SERIAL`.
- No behavioural change; verified with the full DB-gated suite (26
  tests) against a freshly migrated Postgres 18.

### Added — pagination on list and search (2026-08-01)

- **`GET /api/organizations` and `GET /api/organizations/search` take
  `?limit=` and `?offset=`**, and report `X-Total-Count` / `X-Limit` /
  `X-Offset` (the family convention, now written down in
  `agents/share/restful.md`). The body shape is unchanged — these
  endpoints return a bare array and every existing caller parses one, so
  the count goes in a header rather than in an envelope that would break
  them all for a number most do not use.
- **Defaults preserve the old behaviour**: no parameters ⇒ the first 100
  (list) or 50 (search), which is exactly what the hard caps returned.
  `limit` **clamps** to 500 rather than erroring — a caller asking for
  100 000 wants "as many as you'll give me" — while an `offset` past
  10 000 is a `400`, because that one is a cheap denial of service rather
  than an unusual request (SEC-G7).
- Search's total comes from Tantivy's `Count` collector, not the page
  length: a page cannot tell a caller how much there is, which is the
  whole point of the header. The count is the index's match count rather
  than the number of rows that resolved, so it does not wobble when a hit
  refers to a since-deleted row.
- Tests: DB-free pins on the clamp/default/bound rules, plus a DB-gated
  request test walking a window, checking the total exceeds the page,
  the clamp, and the `400`.

**Found while writing it:** `#[serde(flatten)]` on a query-parameter
struct silently breaks typed fields — a flattened struct deserializes
from a string-keyed map, so `limit=2` arrives as the string `"2"` and
fails as a `u64`, turning a valid request into a `400`. The page fields
are therefore declared inline on the search params rather than flattened.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-2, the loco-style half of the rollout (case was the reference; the
five axum-style services landed the same day as AU-1).

- **The verifier and the ABAC policy are now reloadable holders**
  (`ReloadableVerifier` / `ReloadablePolicy`) that the blanket guard
  **and** the bearer extractors read per request. They were boot-only
  `OnceLock` snapshots, so a rotated key set or an edited policy could
  not have reached a running process at all.
- **`spawn_key_refresh`** re-fetches `ORGANIZATION_PASETO_KEYS_URL` every
  `ORGANIZATION_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset). A failed fetch **keeps the current key set** —
  a transient auth-service outage must not lock every caller out.
- **`spawn_policy_watcher`** polls `ORGANIZATION_ABAC_POLICY_FILE`'s mtime every
  15 s and calls `reload_policy()`; a malformed edit falls back to the
  built-in default rather than leaving the service unprotected.
- **`tests/enforcement.rs`** — the activation proof, in its own binary
  because the auth `OnceLock`s are process-wide: public paths stay open,
  a protected read and write without a token are `401`, a malformed
  bearer is `401` (not a 500), a valid token with no attributes reads
  `200` and writes `403`, and `access=write` creates. The record-level
  `authorize_record` added with the privacy layer reads the same holder,
  so masking decisions follow a reloaded policy too.
- New environment variable: `ORGANIZATION_PASETO_KEYS_REFRESH_SECS`.

### Added — field masking + GDPR export (2026-08-01)

- **`src/privacy.rs`** — `mask_organization` redacts what is genuinely
  sensitive about an organization and nothing else: `telephone` and
  `email` (routinely a named individual's line or inbox) are masked to
  their tail, the address's `street_address` is dropped (for a sole
  trader the registered address is a home address, and there is no
  `is_sole_trader` flag to key on), and `TaxId` / `Vat` identifier
  values are masked. Public registry identifiers — LEI, DUNS, ROR,
  ISNI, Wikidata — the names, `url`, and `jurisdiction` are untouched:
  masking those would break the lookups a registry exists for.
- **`GET /api/organizations/{pid}/masked`** — the redacted view on
  demand.
- **`GET /api/organizations/{pid}/export`** — the GDPR right-of-access
  envelope (`entity`, `pid`, `exported_at`, `masked`, `record`, `note`).
  **Audited on every call**, masked or not: a disclosure of personal
  data is itself a recordable event.
- **The ABAC `mask` obligation is wired.** `src/auth.rs` gains
  `authorize_record` + `organization_resource_attrs`
  (`resource.jurisdiction`, `resource.has_fiscal_id`), and
  `GET /{pid}` honours a `mask`-obligation allow by returning the
  redacted record **from the same URL** — so a policy can grant a
  partial read without a second endpoint, and the caller cannot ask for
  the unredacted form. The export follows the same decision and reports
  `masked: true`, because an access request answered with redactions
  must not look complete. All of it is a no-op while
  `ORGANIZATION_REQUIRE_AUTH` is off.
- **No consent model, deliberately.** The shared contract's consent is a
  *data subject* granting a purpose. An organization is not one; the
  natural persons behind it are, and the person service owns their
  consent. A second, unauthoritative home for it would be worse than
  none.
- Tests: 10 DB-free pins (each redaction, the fields that must survive,
  char-safe masking, the export envelope) plus a dedicated
  `tests/masking.rs` binary — its own process because the auth
  `OnceLock`s are process-wide — proving end to end that the obligation
  redacts the ordinary `GET`, carries into the export, and audits both.
  Mutation-checked: dropping the obligation branch fails the suite.


### Added — Tantivy full-text search, fuzzy + phonetic, dedup blocking (2026-07-31)

- **`src/search/`** — a Tantivy index (`index.rs`: schema + lifecycle;
  `mod.rs`: the `SearchEngine` facade and a process-wide `OnceLock`
  engine). Indexed: `name`, `legal_name`, `alternate_names`, Soundex
  codes of every name token, identifier values, `keywords`, the
  flattened postal address, `url` (full-text) plus `jurisdiction` and
  `active` (exact). Only `pid` is stored — hits are resolved against
  Postgres, which stays the source of truth.
- **`GET /api/organizations/search`** is now full-text and ranked, with
  `fuzzy=true` (Levenshtein ≤ 2) and `phonetic=true` (Soundex). Blank
  `q` is still `400`; an unopenable index is `503` rather than an empty
  result, so a broken index cannot masquerade as "no matches". A query
  Tantivy's parser rejects falls back to an OR over its tokens.
- **`POST /api/organizations/check-duplicates` now blocks on the index**
  (fuzzy name + exact identifier + phonetic routes, ≤ 200 candidates)
  instead of scanning up to 1000 rows. This removes the scale cliff
  where record 1001 was unreachable however obvious a duplicate it was;
  in particular a record sharing only an LEI, under a completely
  different name, is now found (pinned by a request test).
- **Indexing is wired into `src/streaming.rs`**, the single seam both
  the native and the FHIR controllers write through: create/update
  replace the document in place, delete and the duplicate side of a
  merge remove it. It runs after the write is durable and is
  best-effort — a failed index write is logged at `ERROR` and never
  fails a request that already committed.
- **`cargo loco task search_reindex`** (`src/tasks/search.rs`) rebuilds
  the index from the database (paginated, clears first, skips and
  counts unreadable payloads), and an **empty index over a populated
  table is rebuilt automatically at boot** — so an upgrade or a lost
  index volume self-heals. `ORGANIZATION_SEARCH_BOOT_REINDEX=0` opts
  out.
- New environment variables: `ORGANIZATION_SEARCH_INDEX_PATH`
  (default `data/search-index`) and `ORGANIZATION_SEARCH_BOOT_REINDEX`
  (default on).
- Tests: 16 DB-free search unit pins and 6 DB-gated request tests
  (keyword hit, index follows update + delete, fuzzy/phonetic over the
  wire, identifier-only duplicate blocking, `search_reindex` rebuild,
  boot self-heal). The DB-gated suite is 22 tests and green against
  Postgres 18.

### Removed

- The Postgres `ILIKE '%q%'` name search (`Model::search`) and its
  `escape_like` wildcard guard (SEC-G4). This crate now issues no
  `LIKE` query at all, so leaving an unused escaper behind would only
  invite a future caller to assume it was still wired in. The sibling
  care-pathway / case services keep theirs — they still search with
  `ILIKE`.

### Added — batch dedup + stored review queue + decision endpoints (2026-07-19)

- `POST /api/organizations/deduplicate` — pairwise batch scan (up to the
  check-duplicates cap) that **persists** candidates in the new
  `review_queue` table (migration `m20260719_000001`; normalized-pair
  UNIQUE upsert — re-scans refresh scores, decided rows keep their
  decision, item ids stay stable) and reports the stored rows. Already
  destructive-classed under ABAC.
- `GET /api/organizations/review-queue[?status=&limit=]` — the stored
  queue, newest first (cap 500).
- `POST /api/organizations/review-queue/{id}/decision`
  (`{"status": "confirmed" | "rejected"}`) — first-writer-wins decision
  (`404`/`422` on unknown/already-decided); reviewer = verified bearer
  `sub`; writes a `review_decision` audit row.
- The Postgres-gated auth-gate request test now detects the process-wide
  `OnceLock` flag cache being poisoned by an earlier test and skips
  honestly (it previously failed the full `--ignored` suite run).

### Fixed

- 2026-07-18 — **Fresh-database `db migrate` failure.** The
  `…_000004_event_outbox` migration used the loco `create_table`
  helper, which pluralizes table names (`event_outbox` →
  `event_outboxes`); its own index DDL then failed and rolled back
  the entire fresh migrate (zero tables). Rewritten as explicit SQL
  creating exactly `event_outbox`; verified against a fresh
  Postgres 18 (all migrations apply, correct table names). Family-wide
  fix (case, care-pathway, organization, portfolio; patient-flow
  shipped with the explicit-SQL form).


### Security

- **SEC-M5: check-digit / format validation of deterministic identifiers.**
  The service stored any `identifiers[i].value` verbatim and validated only
  that it was non-blank — but LEI / DUNS / GLN / VAT drive the matcher's
  **deterministic short-circuit to `1.0`**, so a malformed value in one of
  those could be stored and produce a **false deterministic match**.
  `validation::problems` now validates the deterministic schemes before
  store (`identifier_problem`): **LEI** (ISO 17442 — 20 alphanumerics + ISO
  7064 MOD 97-10 check), **GLN** (13 digits + GS1 mod-10 check digit),
  **DUNS** (9 digits — no public check digit), and **VAT** (2-letter country
  prefix + 2–13 alphanumerics; per-country check digits deferred). A bad
  value is a field-scoped `422`. Non-deterministic schemes are unconstrained.
  Pure check-digit helpers unit-tested with hand-verifiable values.

- **SEC-M1: input-size caps on the `Organization` payload.** The service
  stores the matcher's `Organization` verbatim and scored it with only a
  blank-`name` check — a single multi-megabyte string field or a huge array
  could be used as a CPU/memory `DoS` against the matcher's O(n·m)
  Jaro-Winkler / Levenshtein / Jaccard scoring, amplified across the
  `check-duplicates` scan. A new `src/validation.rs` (`problems`, mirroring
  the case-service reference) now bounds every scalar text field
  (`MAX_TEXT_LEN = 1024` chars — incl. the nested `address.*` sub-fields),
  array cardinality (`MAX_ARRAY_LEN = 256`), and per-entry string length
  (`MAX_ITEM_LEN = 512`), and keeps the blank-`name` /
  non-blank-`identifiers[i].value` rules — all collected into one `422`
  *before* the record is stored or matched. The controller's `validate`
  delegates to it. Unit tests cover blank/oversized text, oversized array,
  oversized array item, nested address, multi-problem reporting, and a
  within-caps large record.

- **SEC-G6: trailing slash can no longer downgrade a destructive POST.**
  `derive_action` classified `/merge` / `/deduplicate` / `/import` via
  `path.ends_with`, so a trailing slash (`POST …/merge/`) fell through to
  `Write` — a non-admin `access=write` caller could reach a destructive op.
  The path is now `trim_end_matches('/')`-normalised first. Test extended.

- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance every relay would **double-ship** the
  same rows. `drain_once` now runs in a transaction and `unpublished` claims
  rows with `FOR UPDATE SKIP LOCKED` (a second relay skips locked rows; the
  lock releases on commit). Delivery stays at-least-once (consumers dedupe on
  `event_id`).

### Changed — event bus: audit now joins the outbox transaction (2026-07-09)

- Under the `outbox` transport, the `audit_logs` write now rides the
  **same transaction** as the entity mutation and its `event_outbox` row
  (`agents/share/event-bus.md` §3 — the three "can never disagree"). It
  was previously a best-effort side channel written *after* the
  transaction committed, so a crash or audit failure could leave a
  committed change + event with no audit row. `AuditModel::record` is now
  generic over `ConnectionTrait`; the `create/update/delete/merge_and_emit`
  functions own the audit write (strict/in-txn under `outbox`, best-effort
  logged under `memory`), and both the native and FHIR controllers no
  longer audit separately. New DB-gated `tests/outbox_audit.rs` drives
  `create_and_emit` under `outbox` and asserts entity + event + audit all
  commit together.

### Added — FHIR R5 API + header-based API versioning (2026-07-08)

- **`GET`/`POST`/`PUT`/`DELETE /fhir/Organization{,/{id}}`, `GET
  /fhir/Organization?<params>` (a searchset `Bundle`), and `GET
  /fhir/metadata`** (the `CapabilityStatement`) — this crate is the
  **family reference implementation** of
  `agents/share/fhir.md`, built first and copied by the other
  in-scope services. New `src/fhir/{mod,resources,search}.rs` (resource
  structs, `to_fhir_organization`/`from_fhir_organization`,
  `FhirOperationOutcome`, the searchset `Bundle`, search-param parsing)
  and mounted `src/controllers/fhir.rs`. Maps the stored
  `organization_matcher::Organization` DTO to a FHIR `Organization` at
  `high` fidelity: `name`/`alias`, identifiers (LEI/DUNS/…) →
  `identifier` (token `system|value`), addresses → `address`, telecom →
  `telecom`, `part_of` → `partOf` reference, `active`. Every non-2xx
  response is an `OperationOutcome`; every response carries
  `application/fhir+json`. Reuses the native model helpers, validators,
  and event/audit path; `/fhir/*` sits behind the same blanket
  auth+ABAC guard as `/api/*` (not on the public allow-list). Supported
  search params: `_id`, `_lastUpdated`, `_count`, `identifier`, `name`,
  `address`, `address-city`, `address-postalcode`. 9 DB-free unit tests
  (DTO↔resource round-trip, each interaction, search→Bundle,
  `OperationOutcome` on 404/400/422, `CapabilityStatement` matches
  mounted routes); clippy-clean.
- **Header-based API versioning.** New `src/version.rs`
  (`resolve_version`, pure/unit-tested) layered as `require_version_mw`
  in `App::after_routes`, per `agents/share/api-versioning.md`: no
  `Accepts-version` header ⇒ served at the current version (`1.0`); an
  explicit but unsupported version ⇒ `406 Not Acceptable`; a bare major
  (`1`) aliases its current minor. The resolved version is echoed back
  as an `Accepts-version` response header. Copied from the event-service
  reference implementation; organization's URLs were already
  version-free, so this was purely additive (no `/api/v1` to remove).

### Added — authz: ABAC policy authorization inside the blanket guard (2026-07-05)

- ABAC authorization landed (supersedes the earlier per-crate
  roles/RBAC sketch; family contract:
  `agents/share/authorization-attributes.md`). When
  `ORGANIZATION_REQUIRE_AUTH` is on, a verified PASETO token is
  further checked by the shared policy engine in
  `authentication-verifier` 0.3: the request's action is derived from
  the HTTP method plus the crate's destructive named POSTs
  (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`, `/deduplicate`,
  `/import`), and the policy is evaluated over the token's new `attrs`
  claim, first-match-wins, defaulting to allow-read / deny-mutation.
- New env vars `ORGANIZATION_ABAC_POLICY` (inline JSON) and
  `ORGANIZATION_ABAC_POLICY_FILE` (path); unset or unparsable ⇒
  `tracing::warn!` + the built-in default policy (`svc=true` ⇒
  everything; `access=admin` ⇒ destructive+write; `access=write` ⇒
  write) — the service always boots.
- `auth::enforce` now takes the HTTP method and the policy and returns
  `403` (deciding-rule reason) for a valid token the policy denies;
  `401` remains missing/bad credential. DB-free unit tests pin the
  family §7 matrix. Flag off ⇒ behaviour-neutral.

### Added

- **Boot-time PASETO key-set fetch over HTTP.** New env var
  `ORGANIZATION_PASETO_KEYS_URL`: when set, the service fetches the
  auth-service's published Ed25519 key set once at boot
  (`Verifier::from_paseto_keys_url`, `authentication-verifier` `fetch`
  feature) from `App::after_routes` via the new `auth::init_from_env`,
  seeding the process-wide verifier before serving. The fetched key set
  wins over `ORGANIZATION_PASETO_KEYS` (`tracing::info!`); any fetch
  failure logs a warning and falls back to the env key set, so the
  service always boots. Unset/blank URL keeps the prior env-injection
  behaviour exactly. Fetch-once only — a periodic refresh loop on key
  rotation is tracked as a future spec item (spec §16). Tests: a local
  ephemeral-port HTTP listener serving the test key set (the fetch-built
  verifier accepts a token signed by that key), a fast-failing-URL
  fallback pin (no panic), and a no-URL env-path pin. (Spec §7 env
  table + §13 fetch follow-up.)

### Fixed

- **`cargo fmt` drift.** Reformatted `src/auth.rs` and
  `tests/requests/organizations.rs` so `cargo fmt --check` passes again
  (no behavioural change).

### Changed

- **Auth pivot — sessions + PASETO (spec-level; code follow-up pending).**
  The family is moving off RS256 JWT + JWKS access tokens to server-side
  cookie sessions plus short-lived **PASETO v4.public** tokens verified
  offline against the authentication-service's published **Ed25519** key;
  the `authentication-verifier` becomes a PASETO verifier and RS256/JWKS
  is decommissioned. Front-ends adopt a BFF + httpOnly cookie + CSRF (the
  browser holds no token). The `ORGANIZATION_REQUIRE_AUTH` flag and
  blanket-enforcement semantics are unchanged — only the verified
  credential changes. Human-facing docs (README/agents/index) updated to
  describe the new model; runtime code follow-up is tracked in spec §13.
  Source of truth:
  [agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md).

### Added

- **Doc/test harmonization pass.** Request-level tests added for the
  audit endpoints (`/audit/recent` + `/{pid}/audit` record CRUD actions;
  invalid pid ⇒ `400`) and for the plain-CRUD `created`/`updated`/
  `deleted` events on `/events/recent` (frozen `EventView` projection).
  `index.md` worked-flow extended to cover search / merge / audit /
  events / whoami / metrics, with a worked merge example; `README.md`
  gained worked merge, `whoami`, and `/metrics.prom` examples and a
  corrected Status section. `AGENTS.md` deferred list corrected (blanket
  `/api/*` enforcement is implemented; only JWKS-over-HTTP fetch
  remains). Crate `spec/index.md` and `AGENTS.md` now cross-link the
  entity umbrella spec (`../spec/index.md`) where the `R-DUP`/`T-7`/
  `T-9`/`T-12` task IDs the source comments cite are defined. Umbrella
  spec §13 T-9 follow-up marked blanket-enforcement done; cap-boundary
  truncation test and request-level whoami-200 test recorded as open
  tasks.

- **Prometheus metrics — `GET /metrics.prom`.** A new process-wide
  `prometheus::Registry` (`src/metrics.rs`, behind a `OnceLock` reached
  via `Metrics::global()`, mirroring `auth::verifier`) is served at the
  application **root** path `/metrics.prom` in text-exposition format
  (`text/plain; version=0.0.4`) by a loco controller
  (`src/controllers/metrics.rs`, mounted at root like the docs). The
  metric set: `organization_created_total`, `organization_updated_total`,
  `organization_deleted_total`, `organization_merged_total` (plain
  counters, incremented one per success path in the CRUD/merge
  controller handlers) plus a labelled `http_requests_total`
  (`path`/`status`) declared for a future request middleware. The path
  is added to `auth::is_public_path`, so it stays public under blanket
  JWT enforcement (no bearer token needed to scrape). New DB-free tests:
  registry render + counter increment (`metrics::tests`), the
  `/metrics.prom` OpenAPI path (`openapi::tests`), and `/metrics.prom`
  in the `enforce` public-path matrix (`auth::tests`). Brings parity
  with the older Axum services, which already expose Prometheus metrics.
- **Durable event bus — Phase 1 (in-memory envelope + `EventPublisher`
  seam).** `src/streaming.rs` now carries the canonical versioned
  `Envelope` (`event_id` UUID dedup key, `schema_version` = 1,
  `entity` = `"organization"`, `kind`, `pid`, `seq`, `actor`, `name`),
  an `EventPublisher` trait, and an `InMemoryPublisher` ring buffer
  (process-wide `OnceLock`) implementing it — replacing the flat
  `OrgEvent` free-function buffer. `occurred_at`/`data` are deferred to
  the outbox stage (Phase 2; the in-memory envelope is kept minimal).
  CRUD/merge call
  sites now stamp the bearer `actor` via `publish_with_actor`
  (`publish` kept as a `None`-actor shim). Pure refactor: behaviour is
  identical and the `GET /api/organizations/events/recent` wire shape is
  **frozen** — it returns the flat `EventView { kind, pid, name, seq }`
  projection of the envelope, byte-identical to before (front-end safe).
  Phases 2–3 (transactional outbox + Fluvio relay) remain infra-gated
  roadmap per [`agents/share/event-bus.md`](../../agents/share/event-bus.md).
- **Blanket `/api/*` JWT enforcement (default-off).** A new
  `ORGANIZATION_REQUIRE_AUTH` env flag (lenient bool — `1`/`true`/`yes`/
  `on`) gates an `axum::middleware::from_fn` layer wired in
  `App::after_routes`. When on, every route except the public health/ping
  + OpenAPI/Swagger paths requires a valid bearer token (`401`
  otherwise); when off (the default) behaviour is unchanged. The decision
  is the pure, unit-tested `auth::enforce` (plus `auth::require_auth`,
  `auth::parse_bool`, `is_public_path`). New `auth::tests` cover the
  matrix; a `#[serial]`/`#[ignore]` request test pins un-authed `GET
  /api/organizations` ⇒ `401` with the public OpenAPI doc still `200`.
  Implements the family contract in `agents/share/jwt-enforcement.md`.
- **Request-level integration tests.** `tests/requests/organizations.rs`
  (loco testing harness + `serial_test`): create round-trip
  (snake_case wire), blank-name `422` on create + update, unknown-pid
  `404`, search (+ blank-`q` `400`), check-duplicates ranking.
  `#[ignore]`-gated so the default `cargo test` stays green without
  Postgres; run with `cargo test -- --ignored`.

### Changed

- **Validation failures now return `422 Unprocessable Entity`** (was
  `400`): blank `name` on create and on replace (`PUT`), per the
  family convention. A DB-free unit test pins the mapping; OpenAPI
  updated.
- **Unknown `pid` now returns `404`** on get/replace/delete (loco's
  default `ModelError::EntityNotFound` mapping produced a `500`,
  breaking the documented contract).
- Docs (`README.md`, `index.md`, `AGENTS.md`) now describe the wire
  format as snake_case (`legal_name`, `same_as`, `founding_date`, …)
  matching the actual DTO serialization — entity spec OQ-1 resolved:
  no serde rename; snake_case is canonical.

### Removed

- loco scaffolding leftovers: `src/workers/downloader.rs` (TODO stub)
  and its worker registration, plus the empty `src/data/` and
  `src/tasks/` modules.

- **Audit log + event streaming.** `audit_logs` table records every
  create/update/delete (with a JSONB snapshot); a process-global
  in-memory event stream publishes Created/Updated/Deleted events.
  Endpoints: `GET /api/organizations/audit/recent`, `/{pid}/audit`,
  `/events/recent`.
- **Name search.** `GET /api/organizations/search?q=` — case-insensitive
  Postgres `ILIKE` on the denormalised name (Tantivy full-text remains a
  §13 follow-up).
- **OpenAPI + Swagger UI.** Hand-authored OpenAPI 3 spec at
  `/api-docs/openapi.json` (accurately typed `Organization` schema, since
  the matcher crate is `utoipa`-free) and a Swagger UI page at
  `/swagger-ui`.

- **Inaugural scaffold (v0.1.0).** loco.rs organization-identity
  registry (schema.org/Organization).
  - Generated via `loco new` (loco-rs 0.16) and stripped of the auth
    starter; auth is centralized in the authentication-service.
  - `organizations` table (`pid`, denormalised `name`, full
    `Organization` payload as JSONB `data`, `active`, soft-delete) +
    `sea-orm-migration` migrator.
  - CRUD controller: create / list / get / update / soft-delete, plus
    `POST /match` (rank a `{query, candidates}` set) and
    `POST /check-duplicates` (match a query against stored records).
  - **Embeds `organization-matcher` directly**: the API DTO *is*
    `organization_matcher::Organization`, stored verbatim and matched
    with the canonical engine — no separate model or adapter.
  - DB-free tests (`tests/matching.rs`): matcher embedding + JSON
    storage round-trip. Green `cargo build`, clippy clean.

### Notes

- The inaugural v0.1.0 scope was CRUD + matching; the entries above
  (in this Unreleased section) extend it with name search (`ILIKE`),
  audit + event streaming, record merge, OpenAPI/Swagger, Prometheus
  metrics, JWT verification (+ default-off blanket enforcement), and
  request-level tests. Still deferred (spec §13): Tantivy full-text
  search, per-field privacy/GDPR export, JWKS-over-HTTP fetch at boot,
  and richer validation.
