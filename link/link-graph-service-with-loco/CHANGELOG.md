# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth
> (numbered §1–§18; live work queue in §13); [README.md](./README.md) —
> user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide. The two
> upstream design docs are
> [cross-service-linking.md](../../agents/share/cross-service-linking.md)
> and [event-bus.md](../../agents/share/event-bus.md).

## [Unreleased]

### Added — governance no-leak coverage for `/neighbors` and `/single-view` (T-27)

`tests/concealment.rs` proved the `subject_of` concealment invariant
only on `GET /api/edges`, but spec §11.4 requires it across `/edges`,
`/neighbors`, and `/single-view` alike — all three call
`auth::conceal_governed`. Extended the test (same fixtures/policy) to
also assert a case-authorised caller's `/neighbors/{person}` and
`/single-view/{person}` surface the edge/affiliation while a non-case
caller's calls to the same endpoints do not, and that a surfaced
`single-view` read is audited under `read_single_view` exactly once.
See spec §13 T-27.

### Added — operator-forceable reconciliation pass (T-36)

There was no endpoint, task, or admin route to force a reconciliation
pass on demand — the reconciliation-divergence runbook documented this
as a confirmed operational gap. Added `POST
/api/admin/reconcile/{entity}`, `Action::Destructive`-gated (built-in
default policy: `svc=true` or `access=admin` only, matching
case-service's bulk `subject_of` dump), which calls
`reconcile::reconcile()` directly — the exact same call the periodic
worker makes — so the on-demand and periodic paths cannot drift. `404`
when `entity` has no `LINK_GRAPH_RECONCILE_URL_<ENTITY>` configured
(nothing to force). New DB-gated test (`tests/force_reconcile.rs`, its
own binary): 401/403/404/200 matrix, plus a mocked-HTTP source proving
the forced pass reports the real divergence and updates the same
T-34/T-35 gauges the periodic worker updates, converging to zero on a
second pass. Not a new link-write endpoint of this service's own — see
`AGENTS.md` "read-only to the world" — it repairs this aggregator's own
read-model against a source it already trusts, the same category of
action the periodic worker already performs unattended. See
`spec/13-tasks.md` T-36 and
`agents/share/runbooks/reconciliation-divergence.md`.

### Fixed — reconciliation gauges' two "sharp edges" closed (T-34/T-35)

`agents/share/runbooks/reconciliation-divergence.md` documented two
operational gaps in the reconciliation metrics, both closed here:

- **T-34**: `link_graph_reconciliation_divergence` was a single,
  unlabelled gauge — every entity's worker wrote the same series, so a
  converged `case` pass's `0` could overwrite a diverging `person`
  pass's real `47` moments later, with no way to tell from the metric
  alone. It is now an `IntGaugeVec` labelled `["entity"]` (mirroring
  `consumer_lag_seconds`'s existing pattern), so each entity's
  divergence is an independent series.
- **T-35**: there was no per-pass success signal at all — a failed
  pass (timeout, non-2xx, malformed JSON) left the divergence gauge
  exactly where it was, so a genuine `0` and a "hasn't run since boot"
  `0` looked identical, and the only signal was a log line. New
  `link_graph_reconciliation_last_success_unixtime` (`IntGaugeVec`,
  `["entity"]`) is set only on a successful pass, so staleness is
  readable from Prometheus alone.

New unit tests in `src/metrics.rs` plus two new DB-gated tests in
`tests/reconcile.rs`. The shared runbook is updated in the same PR
(a four-part change per this crate's shared-contract rule) to record
both sharp edges as closed. See spec/13-tasks.md T-34/T-35.

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

## [0.2.0] - 2026-08-05
### Added — real OpenTelemetry OTLP export (T-22 / repo AU-3, 2026-08-05)

**The first working OTLP exporter anywhere in the Main X Index family.**
The previous attempt at this task closed with the finding that there was
nothing to copy, and that was correct: person, worker and event carry an
`src/observability/` module that builds an OTel `Resource` and then
installs a plain JSON `tracing` subscriber with the exporter and the
`tracing_opentelemetry` layer commented out behind
`// TODO: Initialize OTLP exporter`; every other service, this one
included, had no such module at all. So this is new work written against
[`rust-tracing-opentelemetry-stack.md`](../../agents/share/rust-tracing-opentelemetry-stack.md),
not a port.

- **`src/observability.rs`** — `OTLP_SERVICE_NAME` / `OTLP_ENDPOINT`
  configuration with the shared doc's defaults; one OTel `Resource`
  (`service.name`, `service.version`); an OTLP/**gRPC** batch
  `SdkTracerProvider` and periodic `SdkMeterProvider`; a
  `tracing_opentelemetry` bridge layer installed **alongside** loco's own
  fmt layer, so local JSON/compact logs and remote export are both live
  rather than either/or.
- **Wired through loco's own seam.** `Hooks::init_logger` returns `true`
  and composes `logger::init_env_filter` + `logger::init_layer` (public
  in loco 1.0 precisely so an application can add its own layers), so
  `RUST_LOG`, `logger.level`, `logger.format` and
  `logger.override_filter` keep behaving exactly as before.
  `Hooks::on_shutdown` flushes both providers, so the last batch is not
  lost with the process.
- **`trace_mw`** — one span per request (outermost layer, so a 401/403
  from the read guard is inside the trace), an
  `http.server.request.duration` histogram, and the W3C `traceparent`
  response header the shared doc has always promised. The span is named
  `http.server.request` rather than `{method} {route}`: a `from_fn` layer
  runs outside routing, and this service's raw paths embed `EntityRef`s
  including governed `case:{uuid}` ones, which must not become span names
  or metric labels.
- **Versions**: `opentelemetry` / `_sdk` / `-otlp` /
  `-semantic-conventions` 0.32, `tracing-opentelemetry` 0.33 — *not*
  person's 0.27/0.28 pins. Those were never exercised, and 0.27's
  `install_batch(runtime::Tokio)` pipeline API no longer exists upstream;
  pinning a dead API for symmetry with a stub would be the wrong kind of
  consistency.

**Export is on by default**, at `http://localhost:4317`, because that is
what the shared doc specifies and it describes no activation flag (unlike
`<ENTITY>_REQUIRE_AUTH`). Setting `OTLP_ENDPOINT=""` disables it, which
makes `init_logger` return `false` so loco's untouched logger runs — the
disabled path is loco's, not a re-implementation that could drift. Safe
as a default because the tonic channel is built `connect_lazy()`, the
batch processor owns a dedicated thread, and a full queue drops rather
than blocks: verified by booting against Postgres with nothing listening
on 4317 and getting a normal `200` with a `traceparent`, then a clean
`SIGTERM`.

**Verified against a real collector, not by compiling.** Given that no
span had ever left any service in this family, a "it builds and does not
panic" test would have proved exactly what the existing stubs prove.
`tests/otlp_export.rs` and `tests/otlp_middleware.rs` therefore run a
real in-process OTLP/gRPC collector (`tests/otlp_collector/`) and assert
on the decoded protobuf: the span arrives with the configured
`service.name`, its `tracing` fields as OTel attributes, and — for the
mounted middleware — a trace id **equal to the `traceparent` the HTTP
response carried**. Neither is `#[ignore]`d and neither needs a database.

**One defect this surfaced.** The first live boot with no collector
logged *nothing*: loco's `EnvFilter` is a module whitelist with no
`opentelemetry*` entry, so every failed export was invisible — which
reads exactly like success. `with_exporter_diagnostics` widens the filter
for those targets, but only when the operator has not supplied their own
`RUST_LOG` / `override_filter`; a failing export now logs
`BatchSpanProcessor.ExportError` once per batch interval. Related and
worth knowing: with the bridge installed, `RUST_LOG` also decides what is
*exported*, so a blanket `RUST_LOG=trace` ships every internal `h2` /
`hyper` / `sqlx` span to the collector too.

Still open from T-22's original wording: the Podman health check and
non-root container hardening (`/_health` / `/_ping` and graceful shutdown
are loco's and were already in place).

### Fixed — DOC-6 doc audit (2026-08-04)

Repo-wide task DOC-6, unblocked by LNK-4's completion (T-29..T-33,
above). Confirmed `spec/13-tasks.md` and `spec/16-open-questions.md`
were already accurate (each T-29..T-33 commit kept both current as it
landed — no gaps found there). The real drift was in the narrative
docs those two commit streams never touched: **`README.md` and every
non-tracker `spec/*.md` file were last edited before LNK-4** (git log
confirms — `spec/index.md` not since the auth-pivot commit,
`spec/08-architecture.md` not since it either), so a whole feature
program (LNK-4) and, in `spec/08-architecture.md`'s case, the entire
real `src/` module layout (never updated past the pre-scaffold
"planned" sketch) were undocumented outside the task tracker. Fixed,
no `src/` change:

- **`spec/14-implementation-status.md`** — the headline "15 tests"
  claim was stale by 80 tests (live `cargo test --lib`: 95); the
  "what is implemented" list stopped at the 2026-07-09 v1 core and
  never gained the bus consumer, governance/auth, reconciliation,
  metrics, or LNK-4; §14.3 "upstream prerequisites" described the
  durable bus and the person/worker `entity_links` write-side as still
  pending when both landed months ago. Rewritten against a fresh
  `cargo test --lib` run and `git log` per claim, not inference.
- **`spec/08-architecture.md`** §8.5 "Module structure (planned)" was
  the pre-scaffold sketch (`ref/`, `registry/`, `consume/`,
  `projector/`, `presence/`, `verify/`, `api/rest/`, `workers/`, `db/`,
  `observability/`) — none of those directories exist; replaced with
  the real 20-file `src/` tree (verified via `find src -name '*.rs'`).
  Also added the suggestion job to the component table and diagram.
- **`spec/02-scope.md`** — "Matching… a future cross-service
  `same_identity` matcher is a producer of edges" and "Suggestion-queue
  UI… ships no review workflow" were both false: LNK-4 landed exactly
  those two "future" items. Fixed both directions (in-scope list +
  out-of-scope list).
- **`spec/12-compliance.md`** §12.3 asserted `same_identity` "is
  operator-asserted / high-confidence" unconditionally — no longer true
  since `matcher_suggested` `same_identity` edges exist at
  `confidence < 1.0`.
- **`spec/10-persistence.md`** had no section at all for the
  `suggestion_runs` table (migration `m20260804_000001_suggestion_runs`,
  T-33) despite documenting every other table; added §10.7.
- **`spec/06-functional-requirements.md`** had zero FRs for LNK-4's
  entire suggestion pipeline; added FR-23..FR-28 (§6.8).
- **`spec/01-purpose-and-vision.md`, `AGENTS.md`, `README.md`** — the
  "read-only to the world" / "not a matcher" invariants, true as
  originally written, were phrased in a way a future reader could
  misread as contradicting the suggestion job's real outbound
  `GET`/`POST` traffic to person/worker. Clarified in place (the
  invariant is about this crate's own *inbound* surface — pinned at
  `spec/16-open-questions.md` OQ-9(c), confirmed already correctly
  resolved there, not re-litigated).
- **`spec/03-stakeholders-and-users.md`, `spec/04-glossary.md`,
  `spec/05-domain-model.md`, `spec/07-non-functional-requirements.md`,
  `spec/09-api-surface.md`, `spec/11-testing-strategy.md`,
  `spec/15-roadmap.md`, `spec/17-references.md`** — added the missing
  LNK-4 stakeholders, glossary terms, `IdentityProbe`/`IdentityMatchScore`/
  `IdentityCandidate` domain types (§5.5), a scale-bound NFR-15, an
  explicit "no new route" confirmation, the LNK-4 unit-test tier + the
  manual live-test tier (§11.7, also recording T-33's own follow-up
  note about `test-db` currently sweeping up the live tests), and
  marked the roadmap's "Beyond v0.5 (candidate)" items **done** rather
  than still-future.
- **Not touched, confirmed accurate**: `spec/13-tasks.md`,
  `spec/16-open-questions.md`, `spec/18-change-control.md`, this
  `CHANGELOG.md`'s own T-29..T-33 entries (all eight landing commits
  already have a dated entry here).

Verified: `cargo build`, `cargo test --lib` (95 passed), `cargo fmt
--check`, `cargo clippy --all-targets -- -D warnings` — all clean,
before and after (no `src/` edit in this pass).

### Added — T-33: governance, scale controls, and audit for cross-service suggestion (OQ-9(a)/(d)) — closes LNK-4

The last of the five LNK-4 sub-tasks (T-29 comparator → T-30 blocking →
T-31 periodic job → T-32 review-queue bridge → **T-33**). Adds the two
configured scale caps OQ-9(d) pinned, the durable per-run audit trail,
and a live proof that a suggestion is never auto-promoted regardless of
score. No behaviour change for a deployment that never sets
`LINK_GRAPH_SUGGEST_URL_PERSON` — see T-31's entry below.

- **`src/suggest/mod.rs`** — `generate_candidates_bounded(persons,
  workers, max_candidates)`: the same blocking as T-30, plus a
  per-anchor cap on same-block comparisons. Investigated
  `person-service-with-loco`'s `BatchDeduplicationRequest::max_candidates`
  before copying its semantics: `.take(max_candidates)` off the front
  of an order-preserving candidate slice, per anchor — not a shared
  budget, not score-sorted. Matched exactly:
  `worker_indexes.iter().take(max_candidates)` per person anchor within
  a block, `worker_indexes` in the `workers` slice's own input order,
  so which entries get dropped when a block exceeds the cap is
  deterministic. `generate_candidates` (T-29/T-30's entry point) is now
  a thin wrapper over this at the new `DEFAULT_MAX_CANDIDATES` (`50`)
  constant — no behaviour change for any existing caller or test whose
  fixtures stay under 50 same-block records. 3 new unit tests,
  including the load-bearing truncation proof (a 10-worker
  identifier-sharing block capped at 3 returns exactly `workers[..3]`,
  twice in a row).
- **`src/suggest/job.rs`**:
  - `run_suggestion_pass` now takes `max_candidates` and
    `max_edges_per_run` explicitly, calls `generate_candidates_bounded`
    with the former, and — when the candidate count exceeds the
    latter — sorts candidates descending by confidence (ties broken on
    the `(person, worker)` id pair) and `POST`s only the survivors. The
    number dropped is logged (`tracing::warn!`) and carried on the new
    `SuggestionRunStats::dropped` field. `LINK_GRAPH_SUGGEST_MAX_CANDIDATES`
    / `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN` (defaults `50`/`200`,
    zero/unparseable/unset all fall back to the default) are read once
    per pass in `run_periodic` and threaded through. 4 new unit tests:
    env-parsing fallback for both variables, the cap threading through
    to blocking at the `run_suggestion_pass` level (not just proven
    deeper in `suggest.rs`), and the highest-confidence-survives-the-cut
    proof (three independent same-family-Soundex-avoiding pairs at
    three known confidences via the DOB-proximity table, capped to 2,
    proving the lowest-confidence pair is the one dropped, not an
    arbitrary one).
  - `run_periodic` now takes a `DatabaseConnection` (threaded from
    `app.rs`'s `ctx.db.clone()`), used only to durably record each
    completed pass to the new `suggestion_runs` table (below) and to set
    the new `link_graph_suggestion_last_run` gauge — the job's actual
    work (HTTP fetch + POST) is otherwise unchanged.
  - **Audit investigation, not a new mechanism, for "every POST":**
    person's `create_link` handler already writes an unconditional
    best-effort `person_link` audit row for every link creation
    regardless of provenance, including `matcher_suggested` — this
    job's only write goes through that exact handler. Building a second
    audit trail here would have audited the same event twice from the
    wrong side of the wire. Regression-pinned instead of left as a doc
    claim: `person-service-with-loco`'s new
    `tests/cross_service_link_review.rs::matcher_suggested_link_creation_is_audited`.
- **`src/models/suggestion_runs.rs`** (new) + migration
  `m20260804_000001_suggestion_runs` — one durable row per **completed**
  suggestion pass (`persons_fetched`/`workers_fetched`/`candidates`/
  `posted`/`failed`/`dropped`/`max_candidates`/`max_edges_per_run`,
  `started_at`/`completed_at`). `reconcile.rs`'s own periodic pass only
  ever exposes its one summary number as a live gauge — sufficient
  there ("did the last pass find drift" only needs the current value)
  but not here, since OQ-9(d) explicitly asks for this job's richer
  summary to survive a missed scrape or a restart. A failed pass (fetch
  error) records nothing, matching `run_periodic`'s existing
  log-and-retry posture. New DB-gated test `tests/suggestion_runs.rs`
  proves two completed passes accumulate two rows (history, not a
  last-value slot) and the stored counts round-trip through Postgres
  exactly.
- **`src/metrics.rs`** — new `link_graph_suggestion_last_run` gauge vec
  (labelled `stat`), mirroring `reconciliation_divergence`'s existing
  live-visibility idiom; the durable table above is the history, this
  gauge is the live snapshot. 2 new unit tests.
- **`tests/live_suggest_never_promoted.rs`** (new, manual, `#[ignore]`d,
  not in any CI stage) — the LNK-4 governance capstone, run **live**
  against one real running person-service (not mocked): a real seeded
  person sharing a coded identifier with an in-test synthetic worker
  probe drives the real pipeline; the resulting edge is confirmed at
  `IDENTIFIER_MATCH_CEILING` (`0.99`, above the family's own
  within-entity `auto_merge_threshold` of `0.95`) with `provenance =
  "matcher_suggested"` — never `operator`/`1.0` — and the review-queue
  row stays `pending` — never `confirmed`/`automerged`. A second
  identical pass reasserts the same edge and leaves it still `pending`,
  ruling out a background promotion path. Verified by hand against a
  real running person-service (test-db-backed): both passes green.
- **`spec/13-tasks.md`** / **`spec/16-open-questions.md`** — T-33 marked
  done with the full landing account; OQ-9 marked fully resolved;
  **LNK-4 marked complete**, unblocking DOC-6.

**Acceptance:** `cargo test --lib`: 95 passed (was 85, +10). `cargo fmt
--check` / `cargo clippy --all-targets -- -D warnings` clean. DB-gated
tests green against Postgres 18 (7 test binaries, 19 tests, run
individually — `scripts/ci-check.sh test-db` for this crate currently
aborts after the first binary because of a **pre-existing**, unrelated
issue: the manual `live_suggest_fetch.rs`/`live_suggest_full_pipeline.rs`
tests are swept up by the blanket `-- --ignored` the `test-db` CI stage
runs despite documenting themselves as outside any CI stage — confirmed
via `git stash` to already fail identically at T-32's landing commit,
so not a T-33 regression, but worth a follow-up to give live/manual
tests their own excluded tier). Person-service's own DB-gated suite is
green (21 `--lib` + 25 `api_integration_test` + 5
`cross_service_link_review`, the new audit test included, + 1
`enforcement` + 1 `seed_examples_db`); `cargo test --lib`: 315 passed
(unchanged from T-32).

### Added — T-32: cross-service review-queue bridge + promotion/rejection (OQ-9(b))

Closes the gap T-31 (below) left open: the periodic suggestion job
already `POST`s `matcher_suggested` `same_identity` edges to person, but
the edge landed with no way for an operator to review or confirm it.
Reuses person's **existing** `review_queue` table/endpoints per OQ-9(b)
— no new aggregator endpoint, and this crate stays read-only to the
world.

- **`src/suggest/job.rs`** — `HttpSuggestionSink::post_suggestion` now
  sends the T-29 `IdentityMatchScore` breakdown as a `score_breakdown`
  JSON object (`identifier_match`/`name_score`/`dob_score`/
  `gender_score`) alongside `kind`/`to_ref`/`confidence`/`provenance`,
  so the review-queue row person creates from it carries the
  per-component evidence, not just the final confidence number. The
  pure comparator/blocking logic (`src/suggest.rs`) is unchanged.
- **`tests/live_suggest_full_pipeline.rs`** (manual, `#[ignore]`d, not
  in any CI stage) extended to also fetch person's review queue after
  the pipeline run and confirm the suggested pair's row landed with a
  `score_breakdown` object — closing the T-31→T-32 loop end to end
  against two real running services, not just proving the edge exists.
- **Everything else — the actual review-queue write, the
  entity-type-ambiguity resolution (a non-reordering
  `upsert_cross_service` insert path, since person's existing `upsert`
  normalizes pair order in a way that is correct for within-entity
  dedup and wrong for a person/worker pair), and the
  `review_decision` promotion/rejection extension — lives entirely in
  `person-service-with-loco`.** See that crate's own `CHANGELOG.md` and
  `spec/13-tasks.md` T-32 entry for the full account, and
  `spec/13-tasks.md` T-32 in this crate for why the write was kept
  server-side on person rather than added as a second call from this
  job.

`cargo test --lib`: still 85 passed (the pure suggest/blocking logic did
not change). `cargo fmt --check` / `cargo clippy --all-targets -- -D
warnings` clean.

### Fixed — T-31's fetch source: `search?q=*` replaced with a real list endpoint (verified live)

An independent live check of the T-31 landing below found its core
enumeration claim empirically false: `GET /persons/search?q=*` on a real
running person-service returned zero results against real indexed data.
Root cause, isolated by instrumenting the real running server (not
inferred): person's Tantivy search index is a separate artefact from
its database and can drift from it (in this case, a stale `.env`
`SEARCH_INDEX_PATH` pointing at a long-lived dev index directory with
~900 orphaned documents no longer present in the attached database) —
`q=*` correctly matched everything *in the index*, but a small `limit`
page landed entirely on entries with no surviving database row, so
every hit was dropped by the found-in-index-but-not-in-database guard.
The query grammar's `q=*` → `AllQuery` mapping itself is correct
(confirmed against the exact pinned `tantivy-query-grammar` 0.22.0
source and reproduced with a minimal standalone Tantivy program), but a
search index is the wrong foundation for a "list everything" primitive
in *any* deployment, regardless of this one instance's specific cause.

- **`src/suggest/job.rs`** — `HttpIdentitySource::fetch_all` now pages
  `GET {base_url}?limit=&offset=` (person's/worker's new database-backed
  collection-list endpoint — see `person-service`/`worker-service`'s own
  `CHANGELOG.md` entries for that side of the fix) instead of
  `search?q=*&…`. The Tantivy search index is no longer consulted by
  this job at all. `SearchData` renamed `ListData` to match. Module docs
  rewritten (the "Why `q=*`" section replaced with "Why `GET
  {base_url}?limit=&offset=`, not `search?q=*`", documenting the live
  finding and the correction).
- **`tests/live_suggest_fetch.rs`** (new, `#[ignore]`d) — drives the
  real `HttpIdentitySource::fetch_all` against a genuinely running
  person-service or worker-service, asserting every returned id is
  unique (no page double-counted) and the fetch is non-empty. Not part
  of any automated CI stage — no CI job here brings up a second full
  service. Run by hand against both services: 25 real persons and 21
  real workers, each fully enumerated with zero loss/duplication.
- **`tests/live_suggest_full_pipeline.rs`** (new, `#[ignore]`d) — drives
  the complete real pipeline (`HttpIdentitySource` × 2 →
  `run_suggestion_pass` → `HttpSuggestionSink`) against two real running
  services sharing a seeded shared-identifier person/worker pair, then
  independently `GET`s the person's `/links` to confirm the
  `matcher_suggested` `same_identity` edge actually landed — not merely
  that the POST returned 2xx. Run by hand: `SuggestionRunStats {
  persons_fetched: 26, workers_fetched: 22, candidates: 1, posted: 1,
  failed: 0 }`, edge confirmed present on read-back.
- No unit-test count change (71 pre-existing + 14 from T-31 = 85 still)
  — the fix is in the HTTP call shape (`HttpIdentitySource::fetch_all`'s
  URL), which was never unit-tested directly (only the pure/mocked
  logic was, per T-31's original note) — the two new `#[ignore]`d live
  tests are what actually cover it. `cargo fmt --check` / `cargo clippy
  --all-targets -- -D warnings` clean.

### Added — Cross-service suggestion job (LNK-4, spec T-31, 2026-08-04)

- **`src/suggest/job.rs`** (new — `src/suggest.rs` became
  `src/suggest/mod.rs` plus this sibling, since the T-29/T-30 comparator
  file was already 1000+ lines on its own): the actual I/O this feature
  has been building toward — fetches person + worker identity data over
  HTTP, maps each record to T-29's `IdentityProbe`, runs the pair
  through T-30's `generate_candidates`, and POSTs every surviving
  candidate to person's `POST /api/persons/{id}/links` as a
  `matcher_suggested` `same_identity` edge. Mirrors `src/reconcile.rs`'s
  shape with the verb flipped (`GET` there, `POST` here); the aggregator
  gains no write endpoint of its own (OQ-9(c)).
- `IdentitySource` / `SuggestionSink` traits (mirroring
  `reconcile::AuthoritativeSource`) so the fetch→block→compare→post
  pipeline is unit-tested against mocks, never a live person/worker
  service. `HttpIdentitySource` / `HttpSuggestionSink` are the real
  `reqwest`-backed implementations.
- **A real gap this task surfaced, not assumed:** neither person's nor
  worker's REST API has a plain "list every record" endpoint — only
  `GET /<plural>/search?q=…`, and an *empty* `q` parses to an empty
  Tantivy `BooleanQuery` (zero hits), not "match everything". The
  query grammar's dedicated `*` token, however, parses to
  `UserInputLeaf::All` → `AllQuery`, which does match every indexed
  document (confirmed against the vendored `tantivy-query-grammar`
  0.22 source rather than assumed). `q=*` combined with the existing
  `limit`/`offset` pagination is therefore how this job enumerates a
  service's full collection — the only way to do so through either
  service's public API today. Closing that gap properly (a real
  bulk-list endpoint) is a `person`/`worker`-service change, out of
  this task's scope.
- **Env vars.** `LINK_GRAPH_SUGGEST_URL_PERSON` (person's collection
  base, e.g. `http://host/api/persons` — doubles as the fetch source
  via `/search` and the write target via `/{id}/links`, since person is
  this job's sole write target) and `LINK_GRAPH_SUGGEST_TOKEN` are
  exactly as pinned in OQ-9(c). **`LINK_GRAPH_SUGGEST_URL_WORKER`** is
  a necessary addition beyond OQ-9(c)'s literal text (which names only
  the person write target): the job cannot produce a single candidate
  without also reading worker's collection, and there is nowhere else
  for that URL to come from. Named to match the established
  `LINK_GRAPH_RECONCILE_URL_<ENTITY>` per-entity convention rather than
  inventing a new shape. `LINK_GRAPH_SUGGEST_SECS` (default 3600,
  coarser than reconcile's 300s since this job does real `O(pairs)`
  scoring work) follows the same skip-first-tick pattern as
  `reconcile::run_periodic`.
- SEC-B7 reused, not reimplemented: `reconcile::source_auth_ok` is now
  `pub(crate)` and this job calls it for **both** fetch URLs and the
  write URL — a remote host refuses to start without
  `LINK_GRAPH_SUGGEST_TOKEN`; a loopback host may go token-less.
- Wired into `App::after_routes` (`src/app.rs`): spawns
  `suggest::job::run_periodic` only when `sources_from_env()` resolves
  (i.e. `LINK_GRAPH_SUGGEST_URL_PERSON` is set, `_WORKER` is also set,
  and the SEC-B7 check passes for both). No database handle needed —
  the job is pure HTTP client traffic between two peers, not a
  read-model repair.
- Person/worker → `IdentityProbe` mapping: `name.given` space-joined
  (mirroring how both services' own Tantivy indexers already flatten
  multiple given names — `person.name.given.join(" ")` in their own
  `search/mod.rs`); an identifier's block/match `scheme` prefers the
  FHIR `system` namespace URI (the more specific, cross-service-stable
  field — both services use the same well-known URIs for the same
  real-world scheme, the same signal `person`'s own
  `matching::adapter::route_identifier` keys on) and falls back to the
  coarser `identifier_type` token only when `system` is blank; gender
  tokens (`"male"`/`"female"`/`"other"`/`"unknown"`) map to their
  `person_matcher::Gender` variant one-for-one, with `"unknown"`
  mapping to the real `Unknown` variant (not `None`) and any
  unrecognised token mapping to `None` (excluded, not guessed).
- Deliberately **not** built here (T-33's job, per its own §13 entry):
  `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` / `_MAX_EDGES_PER_RUN`, per-POST
  audit logging, and the non-auto-promotion governance test. This job
  applies one defensive, non-configurable fetch-pagination cap of its
  own (`MAX_FETCH_OFFSET = 10_000`, mirroring person's own SEC-G7
  `MAX_SEARCH_OFFSET` — worker's search handler enforces no such bound
  itself).
- 14 new unit tests: `probe_from_wire` against fixture JSON matching
  the real `Person`/`Worker` wire shape (including the `system`-blank
  fallback and sparse records); every gender-token mapping; the SEC-B7
  accept/reject matrix for `resolve_sources` (loopback without a token,
  remote without a token refused on either the person or the worker
  side, remote with a token, person URL unset, worker URL missing);
  and the mocked end-to-end pipeline (`run_suggestion_pass`) — a
  shared-identifier pair is posted exactly once at the identifier
  ceiling, an unrelated pair posts nothing, a sink failure is counted
  in `failed` without aborting the run, and empty sources produce an
  empty successful pass. `cargo test --lib`: 85 passed (71 pre-existing
  + 14 new); `cargo fmt --check` / `cargo clippy --all-targets -- -D
  warnings` clean. T-32 (person's review-queue promotion) and T-33
  (governance/scale/audit) build on this next.

### Added — Cross-service candidate blocking (LNK-4, spec T-30, 2026-08-04)

- **`src/suggest.rs`** — `generate_candidates(&[(EntityRef, IdentityProbe)],
  &[(EntityRef, IdentityProbe)]) -> Vec<IdentityCandidate>`, built on
  T-29's `compare_identity`: blocks both sides before scoring so the
  cost is `O(n + m + Σ|block|²)` rather than the full `O(n·m)`
  cross-product (design §16 OQ-9(a)). Still pure/offline — no database,
  no HTTP, no clock; pulling real person/worker records is T-31's job.
- Block key (`block_keys`, private): two-tier, per OQ-9(a) — a probe
  carrying one or more coded identifiers blocks on **each** normalised
  `(scheme, value)` pair (a record with multiple identifiers may belong
  to multiple identifier-blocks); a probe with none falls back to a
  single `Soundex(family) + birth_year` block. A probe with neither a
  usable identifier nor a usable name+DOB gets no block key and can
  never be compared to anything.
- Reused `person_matcher::Normalizer::phonetic_code` (already `pub`,
  wraps a real American-Soundex implementation) for the phonetic
  fallback rather than writing a new Soundex — no `pub` change needed
  in `person-matcher`, and it is the same primitive the within-entity
  name matcher already applies as a phonetic bonus.
- Threshold: `SUGGESTION_THRESHOLD` (`0.7`), reused from
  `BatchDeduplicationRequest::threshold`'s default /
  `IMPORT_REVIEW_THRESHOLD` rather than a new number. A same-block pair
  scoring below it is compared but discarded — never returned. There is
  **no auto-merge tier**: every `IdentityCandidate`, even a `0.99`
  identifier-ceiling hit, is returned the same shape for T-32/T-33's
  operator confirmation. A pair reachable through more than one shared
  block (e.g. two shared identifiers) is scored and returned at most
  once (dedup on the underlying pair, not the block key).
- 6 new unit tests, including the load-bearing blocking proof
  (`pairs_in_different_blocks_are_never_compared_even_when_score_would_qualify`):
  a pair that scores `>= 0.7` when compared directly (identical name,
  one-year-apart birth dates) is never returned by `generate_candidates`
  because a one-year difference lands them in different
  `Soundex(family) + birth_year` blocks — proving the blocking actually
  bounds what gets compared, not just that scoring is correct. Also:
  the identifier-block finds the sharing pair and excludes an unrelated
  third record in a different block; the phonetic+birth-year fallback
  block scores a pair with no identifiers; a low-scoring same-block pair
  is discarded; multiple shared identifiers don't duplicate the
  candidate; empty inputs (either side, or both, or an entirely
  unblockable pair) never panic. `cargo test --lib`: 71 passed (65
  pre-existing + 6 new); `cargo fmt --check` / `cargo clippy
  --all-targets -- -D warnings` clean. T-31 (the periodic suggestion
  job) builds on this next.

### Added — Cross-service identity comparator (LNK-4, spec T-29, 2026-08-04)

- **`src/suggest.rs`** — the first piece of the `same_identity`
  suggestion job (design §16 OQ-9, `cross-service-linking.md` §5.2):
  a pure, deterministic, DB-free `IdentityProbe { name, birth_date,
  gender, identifiers }` + `compare_identity(&IdentityProbe,
  &IdentityProbe) -> IdentityMatchScore`, the lean projection a `Person`
  and a `Worker` both reduce to before comparison. No I/O, no clock, no
  cross-service edge consumed or produced — mapping real records and
  writing suggested edges is T-30/T-31's job.
- Depends on `person-matcher` 0.6.1 (new path dependency) for
  `Scorer::jaro_winkler_similarity` and `Gender` — not `worker-matcher`,
  whose `Scorer`/`Gender` are near-duplicates and add nothing this probe
  needs. No changes to person-matcher or worker-matcher themselves.
- Scoring: a shared coded identifier (exact match on a normalised
  `(scheme, value)` pair) short-circuits to `IDENTIFIER_MATCH_CEILING`
  (`0.99`); otherwise a weighted blend of name (0.45), birth date
  (0.45), and gender (0.10) — each excluded (not zeroed) when either
  side lacks the field — scaled to `PROBABILISTIC_CEILING` (`0.97`) so
  a perfect demographic match can never outrank a real identifier match.
  Full weight table and rationale in the module doc.
- `score_dob_pair` here is a **fresh** implementation of the full
  six-row table `agents/matching.md` documents ("Birth Date Matching");
  `person-matcher`'s own private `score_dob_pair` only implements two of
  those six rows (a pre-existing doc/code drift, left as-is rather than
  reached into or silently papered over).
- 17 new unit tests + a `compile_fail` doctest pinning the §7 partition
  rule: `IdentityProbe` has no `From`/`Into` conversion to
  `person_matcher::Person` (or `worker_matcher::Worker`), so a
  suggestion produced here cannot be fed into either within-entity
  `MatchingEngine`. `cargo test --lib`: 65 passed (48 pre-existing + 17
  new); `cargo fmt --check` / `cargo clippy --all-targets -- -D
  warnings` clean.

### Added — Real Fluvio bus consumer (BUS-2, spec T-6, 2026-08-03)

- **The read-model's first real bus consumer**: one task per entity
  topic (`mxi.<entity>.events`, `entity_ref::EntityType::ALL` — 10
  topics), each folding every record into the read-model via a new
  `events::apply_event_idempotent`. Behind this crate's own `fluvio`
  Cargo feature (off by default); gated further by
  `LINK_GRAPH_FLUVIO_ENDPOINT` — unset ⇒ unchanged behaviour (lazy
  verify-on-read + reconciliation remain the integrity path); **set
  without the feature** ⇒ the consumer refuses to start (logged at
  `error`) rather than silently doing nothing, the same
  no-silent-fallback shape as BUS-1.
- New `processed_events` table (spec §10.3) + `apply_event_idempotent`:
  dedupes on the envelope `event_id` under at-least-once delivery. An
  envelope with no `event_id` (optional in v1) applies unconditionally,
  as before.
- **Resume position is delegated to Fluvio's own named-consumer offset
  management** (`offset_consumer` + `OffsetManagementStrategy::Auto`),
  not reconstructed from `consumer_offsets.offset_val` — that column
  keeps writing exactly what `apply_event` always has (the envelope's
  own per-`entity_pid` `seq`), now understood as a freshness/diagnostic
  value rather than a literal Fluvio partition offset. See
  `src/consumer.rs`'s module docs for the full reasoning.
- `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` (copy-adapted from
  case-service's BUS-1 files) provision a local SC+SPU broker for
  opt-in manual runs; not part of any automated CI stage.
  `tests/fluvio_consumer.rs` is a feature-gated, `#[ignore]`d live
  round-trip (produce onto `mxi.person.events` → `consumer::spawn` →
  assert the edge lands), verified by compiling under `--features
  fluvio`, not by an actual execution.
- Verified the real `fluvio` 0.50 consumer API
  (`Fluvio::connect_with_config`, `consumer_with_config`,
  `ConsumerConfigExtBuilder`, `Offset::beginning`,
  `OffsetManagementStrategy::Auto`) against the actual compiler, same
  approach as BUS-1's producer side.

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration → 2.0,
  sea-query → 1.0. This crate's tables (`edges`, `entity_presence`,
  `audit_log`) key on UUID, not an auto-increment integer, so the
  `ColType::PkAuto` 64-bit width change (affecting every other crate in
  this migration) doesn't touch it; no raw `Statement` calls, no
  `ExprTrait`/`.eq()` construction, no `DatabaseConnection::Disconnected`
  either — the cleanest bump in the family so far, no source changes at
  all beyond the two `Cargo.toml`s.
- No behavioural change; verified with the full DB-gated suite (16
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — boot key fetch, key rotation, policy hot-reload, and audited caller addresses (2026-08-01)

AU-3, the last service to adopt the family's auth hardening.

- **The verifier could only ever come from the environment.** There was
  no boot-time fetch at all — unlike the ten entity services, this
  aggregator had no way to read the auth service's published key set.
  `auth::init()` now fetches it once from `LINK_GRAPH_PASETO_KEYS_URL`
  (the `fetch` feature is enabled on `authentication-verifier`), and a
  failed fetch leaves the env-built verifier standing so the service
  always boots.
- **`spawn_key_refresh`** then re-fetches every
  `LINK_GRAPH_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables),
  keeping the current key set on failure — a transient auth-service
  outage must not lock every caller out.
- **The verifier and the ABAC policy are reloadable holders** read per
  request by the blanket guard and the bearer extractor, with
  `spawn_policy_watcher` hot-reloading `LINK_GRAPH_ABAC_POLICY_FILE` on
  an mtime change (malformed edit ⇒ the built-in default).
- **Governed audits now record the caller's address.** `user_ip` was
  hard-coded `None`, so every `subject_of` read and write was logged
  with who but never from where — half of what a governance trail exists
  to answer. The three governed read handlers take `ConnectInfo`, and
  the address is taken from `X-Forwarded-For`'s first hop when present,
  else the transport peer: behind a proxy the peer is the proxy every
  time, and one address on every row looks like evidence while being
  none. Pinned by an assertion in `tests/concealment.rs`.

### Not done — OTLP (spec T-22)

Left open deliberately. There is **no working OTLP export anywhere in
the family** to adopt: person, worker and event build an OTel `Resource`
and then install a plain JSON `tracing` subscriber with the exporter
commented out; everyone else has nothing. A real exporter is new work
and a family-wide decision, not a copy — see the note on T-22.


### Spec — cross-service identity-suggestion matcher (LNK-4 spec round)

- Specified the LNK-4 cross-service `same_identity` matcher + review queue
  (design §16 OQ-9 + the §13 task chain T-29–T-33), the mandated spec round
  before any coding. Resolves the load-bearing design decisions — a
  cross-type `IdentityProbe` comparator reusing the matcher crates' scoring
  primitives (never consuming cross-service edges, §7 partition rule);
  identifier-exact + `Soundex(family)`/birth-year candidate blocking; an
  aggregator-hosted periodic job that POSTs `matcher_suggested` edges to
  person's links endpoint (so person owns the write and the aggregator stays
  read-only to the world); and per-service review with idempotent promotion
  to `operator`/`1.0`. Flags the open sub-questions (block key/threshold,
  review-surface home, aggregator-write posture, scale) that must be pinned
  before T-29. No code.

### Added — worker reconcile source (LNK-2)

- The periodic reconciliation loop (`app.rs::after_routes`) now also
  spawns a source for **worker** (`["case", "person", "worker"]`), so once
  `LINK_GRAPH_RECONCILE_URL_WORKER` is set the aggregator pulls the worker
  service's authoritative `same_identity` edges (the **worker → person**
  direction, the inverse of person's person → worker) via the generic
  `HttpAuthoritativeSource`. The existing SEC-B7 origin/endpoint validation
  (`edge_valid_for_source`) already accepts a worker-origin `same_identity`
  edge, and `graph.rs` canonicalises the symmetric pair, so the by-design
  double-assert is deduped. New seam test
  `bulk_response_deserializes_the_worker_same_identity_shape` pins that the
  worker service's `GET /api/workers/links` body deserializes into a
  `LinkedEvent`. No new endpoint (read-only-to-the-world invariant holds).

### Fixed

- **Security (SEC-B11): non-redirecting presence probe + freshness guard
  pin.** The lazy verify-on-read probe used `reqwest::get`, which **follows
  redirects** — a probed source service could return a `3xx` to an internal
  address (cloud metadata, another service) and the aggregator would follow
  it (SSRF-via-redirect). The probe now uses a shared **non-redirecting**
  client (`redirect::Policy::none()`); a `3xx` maps to `Unknown`
  (`outcome_from_status`), so the only host ever contacted is the
  operator-configured `LINK_GRAPH_PROBE_URL_<ENTITY>` template — that config
  *is* the host allow-list. Separately, a regression test pins that
  `GET /api/health/freshness` (an operational consumer-lag oracle) is **not**
  a public path and stays behind the blanket read guard (`401` without a
  token when enforcement is on), so it can't be mistaken for a public health
  probe and silently exposed. Pure helpers unit-tested.

- **Security (SEC-B7): authenticate the reconcile source and validate its
  edges.** `HttpAuthoritativeSource::from_env_for` built a source even when
  `LINK_GRAPH_RECONCILE_TOKEN` was unset — an **unauthenticated pull** from a
  remote host whose returned edges were then applied to the graph directly.
  Now a **remote** URL requires a token (`from_env_for` refuses and returns
  `None`, logging a warning; only a **loopback** URL may be token-less for
  dev/test — `source_auth_ok` / `is_loopback_url`, fail-closed on an
  unparseable URL). And before applying each authoritative edge, `reconcile`
  validates it with `edge_valid_for_source`: the edge must originate from the
  source's own entity **and** its endpoint types must be permitted for its
  kind by the closed registry (`EdgeKind::permits`), so a compromised or
  buggy source cannot inject a cross-typed or foreign-origin edge (ill-typed
  edges are skipped and stay visible as divergence). Pure helpers
  unit-tested.

- **Security (SEC-B1): reconciliation is now scoped to the source entity.**
  `reconcile` diffed a service's authoritative edges against the **global**
  read-model, so every other entity's edges looked "extra" and were deleted
  — each per-entity pass (`case`, `person`) wiped the other's edges and the
  graph never converged (a critical data-loss bug). `AuthoritativeSource`
  now declares `entity()`, and `reconcile` diffs only the read-model edges
  originating from that entity (`Model::edge_ids_from_entity`, exact
  `<entity>:` `from_ref` prefix). Correct for both live sources
  (`subject_of` from=case; canonical `same_identity` from=person). New
  DB-gated `reconcile_is_scoped_to_the_source_entity` test + pure
  prefix-matching unit tests (`course` vs `courseinstance`, literal `_` in
  `care_pathway`).

### Added — reconciliation core (spec T-20 / design §8) (2026-07-10)

- `src/reconcile.rs` — the "cost of two sources of truth" pass: diff the
  read-model `edges` against a service's authoritative `entity_links`,
  emit a divergence metric, and repair.
  - Pure `diff` (missing/extra by `edge_id`) — unit-tested.
  - `AuthoritativeSource` trait (mockable) so the logic is testable
    without a live service.
  - `reconcile` sets the `link_graph_reconciliation_divergence` gauge
    (design §8 SLO) and repairs the read-model (upsert missing via
    `apply_linked`, remove extra via `apply_unlinked`).
  - DB-gated `tests/reconcile.rs` with a mock source: adds a missing
    edge, removes an extra one, converges to 0 on re-run.
- **Now live** (2026-07-10): `HttpAuthoritativeSource` — a bearer-authed
  `GET` of `LINK_GRAPH_RECONCILE_URL_<ENTITY>` parsing the canonical §4.2
  edge list — plus `run_periodic`, spawned from `after_routes` when a
  source is configured (`LINK_GRAPH_RECONCILE_SECS`, default 300). The
  authoritative source is the case service's new `GET /api/cases/links`.
  A DB-free unit test pins that the case bulk-links JSON deserializes into
  the aggregator's `LinkedEvent` (the cross-service seam). `after_routes`
  now spawns one worker per entity that configures a source.
- **Person source** (2026-07-10): the person service's `same_identity`
  edges are reconciled too — `after_routes` iterates `["case", "person"]`,
  and a second seam unit test pins that person's `GET /api/persons/links`
  (`same_identity` person→worker) deserializes into `LinkedEvent`.

### Added — Prometheus metrics (spec T-21) (2026-07-10)

- `src/metrics.rs` — a process-wide registry served at the root
  `GET /metrics.prom` (public, in `is_public_path`; Prometheus
  text-exposition format):
  - `link_graph_events_processed_total{kind}` — counter, incremented in
    `apply_event` for each folded event.
  - `link_graph_edges{status}` — gauge, edge count by integrity status,
    refreshed from the DB (`edges::Model::count_by_status`) at scrape time.
  - `link_graph_consumer_lag_seconds{entity}` — gauge, freshness watermark
    vs now, per entity topic, refreshed at scrape time.
- Adds `prometheus`. Unit test (render output) + DB-gated endpoint test.
  Reconciliation divergence is deferred with the reconciliation worker (T-20).

### Added — lazy verify-on-read (spec T-10 / design §5.1) (2026-07-10)

- `src/probe.rs` — the interim integrity path until the durable bus feeds
  presence via events. When a read surfaces an edge whose endpoint
  presence is unknown, the aggregator can resolve it with a one-shot `GET`
  to the source service, cache the verdict in `entity_presence`, and
  recompute the incident edge status (`unverified` → `verified` /
  `dangling`) — so status settles on first read even before the bus is
  live. Pieces:
  - `PresenceProbe` trait (mockable) + `HttpPresenceProbe` (`2xx`⇒alive,
    `404`⇒absent, else unknown; `reqwest`).
  - Per-entity URL **template** from `LINK_GRAPH_PROBE_URL_<ENTITY>`
    (`{id}` substituted) — no hardcoded service hosts or plural paths; an
    entity with no template is skipped.
  - `verify_unknown` probes only unknown endpoints (deduped), caches, and
    recomputes status.
  - Wired into `neighbors` / `edges` behind `LINK_GRAPH_LAZY_VERIFY` (off
    by default; re-reads only when something resolved). `single-view` is
    not wired (its response carries no status).
  - Unit tests (URL resolution) + DB-gated `tests/lazy_verify.rs` with a
    mock probe (alive⇒verified, absent⇒dangling, idempotent). The real
    HTTP path is compile-checked.

### Added — OpenAPI 3 + Swagger UI (spec T-15) (2026-07-10)

- `src/openapi.rs` — a hand-written OpenAPI 3.0.3 document (dependency-light,
  no `utoipa`, matching the sibling services) covering the four read
  endpoints (`neighbors` / `edges` / `single-view` / `health/freshness`)
  and their enveloped schemas (`Edge`, `Affiliation`, `TopicFreshness`,
  the `{success,data,error}` envelopes, each carrying `as_of`).
- `controllers::docs` serves `GET /api-docs/openapi.json` (the spec) and
  `GET /swagger-ui` (a CDN-loaded Swagger UI page). Both are already in
  `auth::is_public_path`, so the blanket read guard never gates the docs.
- Unit tests: well-formedness + all four endpoints documented + every
  edge-returning response enveloped with `as_of`.

### Added — end-to-end concealment proof (spec T-16 / design §10) (2026-07-10)

- New DB-gated `tests/concealment.rs` (own binary) mints real
  PASETO v4.public tokens against a throwaway key set and installs a
  **restrictive ABAC policy** (any authenticated caller may read the
  aggregator, but `case`-read needs `dept=cases`). It proves the
  load-bearing §10 invariant end-to-end: a `dept=cases` caller sees the
  `subject_of` edge **and that read is audited** (`read_edge` with the
  caller `sub`), while a `dept=hr` caller — who still passes the blanket
  guard — has the same edge **concealed** (an affiliation stays visible
  to both, and the concealed read audits nothing). This closes the
  token-minting follow-up flagged with the blanket guard; concealment is
  now unit- **and** integration-tested.

### Added — blanket `/api/*` read guard (spec §9.4 / T-19) (2026-07-10)

- `auth::enforce` + `auth::is_public_path` + the `require_auth_mw` layer
  (`app.rs::after_routes`): when `LINK_GRAPH_REQUIRE_AUTH` is on, every
  non-public request needs a valid bearer token (`401`) whose `attrs` the
  ABAC policy grants `read` on the aggregator (`403`). The service is
  read-only, so the action is always `Read`. This protects **affiliation**
  edges (previously served to anyone under enforcement); the per-record
  `case↔person` concealment (§10) stacks on top for authenticated callers.
  Off by default (behaviour-neutral until a deployment activates it).
- Unit tests (flag-off / public-path / missing-token) + the DB-gated
  `tests/governance.rs` reworked to assert an unauthenticated read is
  `401` at the guard while a governed write still audits. (The
  end-to-end *concealment* path for an authenticated-but-not-case-authorised
  caller stays unit-covered; a token-minting DB test is a follow-up.)
- `audit_log` added to the test-harness `truncate` list.

### Added — governance audit trail (spec T-17 / design §10) (2026-07-09)

- New `audit_log` table (§10.4) + `models::audit_log` — every read/write
  touching a governed `subject_of` (case↔person) edge is recorded, so the
  aggregator's access trail matches the case service's.
  - **Reads** audit each governed edge actually **surfaced** (post
    concealment): `read_edge` on `neighbors`/`edges`, `read_single_view`
    on `single-view`, stamped with the caller `sub` and `User-Agent`. A
    concealed read audits nothing (the edge was not disclosed).
  - **Writes** audit `apply_linked` for governed edges (no actor —
    bus-driven).
  - DB-gated `tests/governance.rs` pins the write-audit row; `user_ip`
    capture (ConnectInfo) is deferred.

### Added — case↔person governance + PASETO auth (spec T-16/T-19 / design §10) (2026-07-09)

- `src/auth.rs` — offline **PASETO v4.public** verification via
  `authentication-verifier` (env key set `LINK_GRAPH_PASETO_KEYS`,
  fail-closed on a missing key set), a `MaybeAuthUser` extractor, and the
  shared **ABAC** policy (`LINK_GRAPH_ABAC_POLICY[_FILE]`, else the
  built-in default).
- **Governance concealment** (the load-bearing §10 invariant): a
  `subject_of` (case↔person) edge asserts a person is the subject of a
  government case, so an unauthorised caller must not learn it exists.
  `may_see_governed` grants it only to a caller the ABAC policy allows to
  `read` `case` (unauthenticated ⇒ denied); `conceal_governed` strips
  governed edges from `neighbors` / `edges` / `single-view`, so even a
  direct `?kind=subject_of` returns an empty list rather than revealing
  the edge. Keyed on the registry's `Sensitivity::High`, so a future
  high-sensitivity kind is covered automatically.
- Gated on `LINK_GRAPH_REQUIRE_AUTH` (family default-off; a deployment
  handling real case data MUST enable it). Unit tests for the decision +
  concealment logic; DB-gated `tests/governance.rs` (own binary) proves
  end-to-end that an unauthorised caller sees affiliations but not the
  case↔person edge.
- Deferred (spec §13): audit of governed reads (T-17), masking parity
  (T-18), and the blanket `/api/*` guard for affiliation edges (only the
  edge-level case↔person concealment is wired).

### Added — merge repointing (spec T-9 / design §5.3) (2026-07-09)

- A `merged{merged_from}` event now **repoints** every edge referencing
  the merged-away duplicate onto the survivor, centrally (the "one
  aggregator helps" fix-up). Previously `merged` was acknowledged but not
  projected, so a record merge orphaned the duplicate's edges (they
  degraded to `dangling`). Pieces:
  - `graph::repoint` — pure endpoint-swap + re-canonicalise, returning
    `None` when the edge collapses to a self-loop (dropped). Unit-tested
    (directed repoint, symmetric re-canonicalisation, self-loop).
  - `edges::Model::repoint_all` — per-edge repoint with **de-duplication**
    (drop a repointed edge that collides with an existing canonical
    edge) and status recompute against the survivor's presence.
  - `apply_event` `merged` branch: marks the duplicate's presence
    `deleted`, repoints, then recomputes incident status.
  - DB-gated `tests/graph_endpoints.rs`: repoint-onto-survivor and
    collision-de-dup.

## [0.1.0] — 2026-06-16

Inaugural **spec-only** scaffold for the Link Graph Service — the
read-model aggregator (read side) of the Main X Index cross-service
entity-linking design. No Rust crate, no `Cargo.toml`, no migrations, no
code: this release is the specification and doc set.

### Added

- **SDD spec set** (`spec/`, §1–§18, one file per section + `index.md`
  table of contents), realising
  [`cross-service-linking.md`](../../agents/share/cross-service-linking.md)
  (this service is its §4.3 read-model aggregator) and the §9 consumer
  model of [`event-bus.md`](../../agents/share/event-bus.md):
  - §1 Purpose / vision — read-only-to-world aggregator; derived,
    rebuildable read-model.
  - §2 Scope — bus consumption, `edges` + `entity_presence`, integrity
    lifecycle, lazy verify-on-read, merge repointing, read API,
    reconciliation, `case ↔ person` governance.
  - §3 Stakeholders, §4 Glossary (`EntityRef`, edge, `linked`/`unlinked`,
    `status`, `as_of`, partition rule, …).
  - §5 Domain model — `EntityRef`, `Edge`, `EdgeStatus`, `Provenance`,
    `EntityPresence`, the closed v1 `EdgeKind` registry.
  - §6 Functional requirements (FR-1 … FR-22) across consumption,
    edge read-model, presence oracle, merge repointing, read API,
    governance, reconciliation/observability.
  - §7 Non-functional — eventual-consistency + freshness/divergence
    SLOs, rebuildability, performance, security/privacy, stack
    conformance.
  - §8 Architecture — hybrid topology read side; consumer / projector /
    presence / verifier / read-API / reconciliation layering; integrity
    state machine; merge-repoint rationale; planned module structure.
  - §9 API surface — read-only `/api/v1/neighbors|edges|single-view|health/freshness`,
    every graph response carrying `as_of`.
  - §10 Persistence — `edges` (bidirectional, indexed both ends),
    `entity_presence`, `consumer_offsets`, `processed_events`,
    `audit_log`; SeaORM time-type note.
  - §11 Testing strategy — un-gated / DB-gated / bus-gated / governance
    tiers.
  - §12 Compliance — `case ↔ person` high-governance posture; data
    minimisation; audit-vs-event-stream distinction.
  - §13 Tasks (T-1 … T-28, all unchecked), §14 Implementation status
    (spec-only), §15 Roadmap (v0.1 → v0.5), §16 Open questions
    (OQ-1 … OQ-8), §17 References, §18 Change control.
- **`README.md`** — user-facing intro (read API, key concepts,
  governance, status).
- **`CLAUDE.md`** — one-line `@AGENTS.md` include.
- **`AGENTS.md`** — agent guide: design-docs-are-upstream rule,
  three/four-part PR rule, load-bearing invariants (read-only,
  partition rule, closed registry, idempotency, `as_of`, governance),
  stack ground rules.

### Notes

- This is a **cross-cutting** service (no single sibling matcher or
  front-end); it consumes every entity service's event stream.
- `EntityRef` + the edge-kind registry are shared *contracts* copied
  per project (drift-accepted, OQ-4) — not a shared package.
- Upstream prerequisites (durable bus; `linked`/`unlinked` events +
  per-service `entity_links` on person + worker) are themselves at
  design / rollout stage; the interim path is in-memory transport +
  lazy verify-on-read.
- Build-out is enumerated as unchecked tasks in
  [`spec/13-tasks.md`](./spec/13-tasks.md) (T-1 … T-28), ordered after
  the design rollout: contracts → `same_identity` backbone → reads →
  affiliations + `case ↔ person` governance → hardening / durable-bus
  flip. No code yet at this release (see
  [`spec/14-implementation-status.md`](./spec/14-implementation-status.md)).
