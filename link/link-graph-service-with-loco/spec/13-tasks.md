## 13. Tasks

> **v1 core landed 2026-07-09.** The compiling, clippy-clean,
> unit-tested read-model core is in place (scaffold, three tables,
> pure projection logic, the `apply_event` seam, the four read
> endpoints, and both test tiers). Items below are checked as they
> land; the bus consumer, integrity/interim, governance, auth, and
> hardening tiers remain deferred. The order follows the design's
> rollout
> ([cross-service-linking.md §11](../../../agents/share/cross-service-linking.md#11-rollout)):
> contracts → backbone → aggregator reads → affiliations → hardening.

### Contracts & scaffold

- [x] T-1: Scaffold the loco service skeleton (Cargo.toml, `src/`,
  `migrations/` + loco `migration/` bridge, config). Read-only-to-world:
  no write controllers. **Done** — Dockerfile / docker-compose deferred.
- [x] T-2 / T-3: ~~Copy the `EntityRef` value type~~ / ~~Copy the closed
  v1 `EdgeKind` registry~~. **Deviation (2026-07-09):** these predate the
  standalone [`entity-ref`](../../entity-ref-rust-crate) crate (design
  §11 step 1, landed 2026-07-06). The crate now owns `EntityType` /
  `EntityRef` / `EdgeKind` / `Sensitivity` and is **depended on**
  (`entity-ref = { path = "../entity-ref-rust-crate" }`, `use
  entity_ref::…`) rather than re-copied, so there is one tested copy.
  `EdgeStatus` / `Provenance` (this crate's own value types) live in
  `src/graph.rs`.
- [x] T-4: SeaORM entity modules + migration SQL pairs for `edges`,
  `entity_presence`, `consumer_offsets` (the three v1 tables, incl. the
  `edges` from/to/status indexes). `processed_events` + `audit_log`
  (§10.3/§10.4) deferred with their consumers (T-6 / T-16..18).

### Bus consumption & projection

- [x] T-5: Envelope decode for the `linked` / `unlinked` event `data`
  shape (design §4.2) + the `created` / `deleted` / `merged` envelope
  (`src/events.rs`). DB-free tests. (`schema_version` switch deferred
  with the durable bus, T-23.)
- [x] T-6: Per-topic bus consumers (`mxi.<entity>.events`), idempotent
  on `event_id` via `processed_events`; per-topic offset + freshness
  watermark to `consumer_offsets` (§6 FR-1/2/3). *(Done 2026-08-03,
  BUS-2.)* One task per entity topic (`src/consumer.rs`,
  `entity_ref::EntityType::ALL`), behind this crate's own `fluvio`
  Cargo feature (off by default) and gated further by
  `LINK_GRAPH_FLUVIO_ENDPOINT` — unset ⇒ unchanged v1 behaviour (lazy
  verify-on-read + reconciliation remain the integrity path); **set
  without the feature** ⇒ the consumer refuses to start (logged at
  `error`, not a silent no-op). New `processed_events` table (§10.3)
  backs a new `events::apply_event_idempotent`, which every real
  consumer call goes through instead of `apply_event` directly — dedup
  on `event_id`, applying unconditionally when the envelope carries none
  (v1's optional field). **Design decision (recorded so it isn't
  re-litigated):** resume position is delegated to Fluvio's own
  named-consumer offset management (`offset_consumer` +
  `OffsetManagementStrategy::Auto`) rather than reconstructed from
  `consumer_offsets.offset_val` — see §10.3 below and `src/consumer.rs`'s
  module docs for the full reasoning; that column keeps writing exactly
  what `apply_event` always has (the envelope's own per-`entity_pid`
  `seq`), now understood as a freshness/diagnostic value, not a literal
  Fluvio partition offset.
  Tests: `tests/idempotency.rs` (DB-gated, 3 tests — redelivery doesn't
  duplicate, distinct event_ids both apply, a no-`event_id` envelope
  applies every time); `tests/fluvio_consumer.rs` (feature-gated,
  `#[ignore]`d live-broker round-trip via `compose.fluvio.yaml`,
  verified by compiling under `--features fluvio`, not by an actual
  execution — no automated run in this repo stands up a broker). Green:
  `cargo build`/`test --lib`/`clippy --all-targets -D warnings`/`fmt
  --check` under both default features and `--features fluvio`; `cargo
  deny check` shows only the pre-existing `rsa` advisory; the full
  DB-gated suite (19 pre-existing + 3 new idempotency tests) against
  real Postgres, zero regressions. **Retiring lazy verify-on-read for
  entities with a live topic** (this task's original "keep it for the
  rest" note) is **not** implemented — `LINK_GRAPH_LAZY_VERIFY` stays a
  single global flag, and only `case` has a real producer today (BUS-1);
  turning it off globally now would leave every other entity's presence
  permanently unresolvable. Revisit once BUS-3 rolls `FluvioSink` out
  further.
- [x] T-7: Graph projector — `linked` upsert / `unlinked` remove +
  symmetric canonicalisation for `same_identity` (§6 FR-4/5/6).
  `graph::canonical` + `edges::Model::apply_linked`/`apply_unlinked`.
- [x] T-8: Presence oracle — `created` / `deleted` → `entity_presence`;
  recompute incident-edge `status` (§6 FR-8/9/10). `graph::edge_status`
  + `edges::Model::recompute_status_for`.
- [x] T-9: Merge repointing handler — `merged{pid, merged_from}` rewrites
  edges centrally, re-canonicalises, de-duplicates (§6 FR-12). *(Done:
  pure `graph::repoint` (endpoint swap + re-canonicalise + self-loop
  drop, unit-tested) + `edges::Model::repoint_all` (per-edge repoint,
  de-dup against an existing canonical edge, status recompute) wired into
  the `apply_event` `merged` branch, which also marks the duplicate's
  presence deleted. DB-gated repoint + de-dup tests.)*

### Integrity (interim)

- [x] T-10: Lazy verify-on-read client — one-shot `GET /{id}` to the
  source service; cache verdict in `entity_presence`; supersede per-entity
  as topics go durable (§6 FR-11, design §5.1). *(Done: `src/probe.rs` —
  the `PresenceProbe` trait (mockable) + `HttpPresenceProbe` (one-shot
  `GET`: 2xx⇒alive, 404⇒absent, else unknown), the per-entity URL
  **template** from `LINK_GRAPH_PROBE_URL_<ENTITY>` (`{id}` substituted —
  no hardcoded hosts/paths), and `verify_unknown` which probes only
  unknown endpoints, caches the verdict, and recomputes incident edge
  status. Wired into `neighbors` / `edges` behind `LINK_GRAPH_LAZY_VERIFY`
  (off by default; re-reads only when something resolved). Unit tests
  (URL resolution) + DB-gated `tests/lazy_verify.rs` with a mock probe
  (alive⇒verified, absent⇒dangling, idempotent second call). The real
  HTTP path is compile-checked. `single-view` is not wired — its response
  carries no status.)*
  - [x] **SEC-B11 (security fix): non-redirecting probe + freshness guard
    pin.** The probe used `reqwest::get` (follows redirects) — a source
    service could `3xx` to an internal address and the aggregator would
    follow it (SSRF-via-redirect). It now uses a shared **non-redirecting**
    client (`redirect::Policy::none()`); a `3xx` ⇒ `Unknown`
    (`outcome_from_status`), so the only host contacted is the
    operator-configured `LINK_GRAPH_PROBE_URL_<ENTITY>` template (that config
    is the host allow-list). Plus a regression test pinning
    `GET /api/health/freshness` is not a public path and stays behind the
    blanket guard (`401` when enforcement is on). (Repo tasks.md Phase 5
    SEC-B11.)

### Read API

- [x] T-11: `GET /api/neighbors/{ref}` — both-direction index
  lookups; `kind` / `direction` / `depth` (capped at 2) filters;
  `as_of` (§6 FR-13/17). Malformed ref / unknown kind / over-cap depth
  ⇒ `400`.
- [x] T-12: `GET /api/edges` — `from` / `to` / `kind` / `status`
  filters; `as_of` (§6 FR-14).
- [x] T-13: `GET /api/single-view/{ref}` — `same_identity`
  unification + affiliation walk (`person → worker → org`); `as_of`
  (§6 FR-15). `graph::single_view`.
- [x] T-14: `GET /api/health/freshness` — per-topic lag (§6 FR-16).
- [x] T-15: OpenAPI + Swagger UI at `/swagger-ui`; raw spec at
  `/api-docs/openapi.json`. *(Done: a **hand-written** OpenAPI 3.0.3 doc
  (`src/openapi.rs::spec`) covering the four read endpoints + the
  enveloped schemas — dependency-light, no `utoipa`, matching the sibling
  services; served by `controllers::docs` (JSON + a CDN-loaded Swagger UI
  page). Both paths are in `auth::is_public_path`, so the blanket guard
  never gates the docs. Unit-tested for well-formedness + endpoint
  coverage.)*

### Governance (`case ↔ person`)

- [x] T-16: Access control on every read path that could surface a
  `subject_of` / `about` edge (incl. `single-view`), with concealment of
  existence to unauthorised callers (§6 FR-18/20, §12). *(Done:
  `auth::is_governed` (keyed on the registry's `Sensitivity::High`),
  `auth::may_see_governed` (ABAC `read`/`case` decision on the verified
  claims; unauthenticated ⇒ denied), and `auth::conceal_governed` strip
  governed edges from `neighbors` / `edges` / `single-view` — a direct
  `?kind=subject_of` returns an empty list rather than revealing them.
  Gated on `LINK_GRAPH_REQUIRE_AUTH`. Unit tests + DB-gated
  `tests/concealment.rs` (minted tokens + a restrictive policy: a
  `dept=cases` caller sees the `subject_of` edge and is audited; a
  `dept=hr` caller has it concealed).
  Audit of governed reads (T-17) + masking parity (T-18) still pending.)*
- [x] T-17: Audit every read/write touching governed edges to
  `audit_log` (§6 FR-19). *(Done: the `audit_log` table (§10.4) +
  `models::audit_log` (`AuditContext` + `record`). Reads audit each
  governed edge **surfaced** post-concealment — `read_edge` on
  `neighbors`/`edges`, `read_single_view` on `single-view` — with the
  caller `sub` + `User-Agent`; a concealed read audits nothing. Writes
  audit `apply_linked` for governed `subject_of` edges (no actor —
  bus-driven). DB-gated test pins the write-audit row. `user_ip` capture
  (ConnectInfo) deferred.)*
- [ ] T-18: Privacy masking parity with the case service on graph
  responses (§6 FR-20).

### Auth, hardening, observability

- [x] T-19: Offline PASETO v4.public verification via
  `authentication-verifier` (NFR-10, per
  [authentication-sessions.md](../../../agents/share/authentication-sessions.md)),
  coordinated with the family-wide auth rollout
  ([jwt-enforcement.md](../../../agents/share/jwt-enforcement.md)).
  *(Done: `src/auth.rs` — env-configured `Verifier` (`LINK_GRAPH_PASETO_KEYS`,
  fail-closed empty key set), `MaybeAuthUser` extractor, ABAC `policy()`
  from `LINK_GRAPH_ABAC_POLICY[_FILE]`; used by T-16 governance. The
  **blanket read guard** (§9.4) is now wired: `auth::enforce` +
  `is_public_path` + the `require_auth_mw` layer in `app.rs::after_routes`,
  gated on `LINK_GRAPH_REQUIRE_AUTH` — every non-public read needs a valid
  token (401) the ABAC policy grants `read` (403); read-only ⇒ action
  always `Read`; per-record case↔person concealment (§10) stacks on top.
  Unit + DB-gated tests. Deferred: key-rotation refresh and the boot-time
  keys-over-HTTP fetch.)*
- [x] T-20: Reconciliation — diff read-model vs each service's
  authoritative `entity_links` (bulk-read or replay); emit divergence
  metric; repair (§6 FR-21, design §8). *(`src/reconcile.rs`: the pure
  `diff` (missing/extra by `edge_id`, unit-tested), the
  `AuthoritativeSource` trait, `reconcile` (diff → set the
  `link_graph_reconciliation_divergence` gauge → repair: upsert missing /
  remove extra), the **`HttpAuthoritativeSource`** (a bearer-authed `GET`
  of `LINK_GRAPH_RECONCILE_URL_<ENTITY>` → the canonical §4.2 edge list),
  and `run_periodic` — spawned from `after_routes` when a source is
  configured (interval `LINK_GRAPH_RECONCILE_SECS`, default 300). The
  authoritative source is the **case** service's `GET /api/cases/links`
  (its `entity_links` bulk read). DB-gated `tests/reconcile.rs` (mock
  source) + a unit test pinning that the case bulk-links JSON deserializes
  into the aggregator's `LinkedEvent` (the cross-service seam). Now **live**
  for case; other services follow as they gain a bulk-links endpoint.)*
  - [x] **LNK-2: worker reconcile source.** `after_routes` now spawns a
    source for **worker** too (`["case", "person", "worker"]`), so with
    `LINK_GRAPH_RECONCILE_URL_WORKER` set the aggregator pulls the worker
    service's authoritative `same_identity` edges (worker → person, the
    inverse of person's direction) via the generic `HttpAuthoritativeSource`;
    `edge_valid_for_source` already accepts a worker-origin `same_identity`
    edge and `graph.rs` dedupes the symmetric pair. Seam test
    `bulk_response_deserializes_the_worker_same_identity_shape` pins the
    `GET /api/workers/links` body → `LinkedEvent`.
  - [x] **SEC-B1 (security fix): scope reconciliation to the source
    entity.** `reconcile` diffed the source's edges against the **global**
    read-model (`all_edge_ids`), so each per-entity pass marked every
    *other* entity's edges as "extra" and deleted them — the case and person
    passes wiped each other's edges and the graph never converged.
    `AuthoritativeSource` now declares its `entity()`, and `reconcile` diffs
    only the read-model edges whose canonical `from_ref` is owned by that
    entity (`edges::Model::edge_ids_from_entity`, exact `<entity>:` prefix so
    `course`/`courseinstance` and the `_` in `care_pathway` don't collide).
    Correct for both live sources: `subject_of` (from=case) and canonical
    `same_identity` (person < worker ⇒ from=person). DB-gated
    `reconcile_is_scoped_to_the_source_entity` proves a case pass leaves a
    person `same_identity` edge intact; pure `from_ref_scoping_*` unit tests.
    (Repo tasks.md Phase 5 SEC-B1.)
  - [x] **SEC-B7 (security fix): authenticate the source + validate its
    edges.** `HttpAuthoritativeSource::from_env_for` refuses to build an
    **unauthenticated remote** source: a non-loopback URL requires
    `LINK_GRAPH_RECONCILE_TOKEN` (`source_auth_ok`/`is_loopback_url`,
    fail-closed on an unparseable URL); only a loopback URL may be token-less.
    Before applying each authoritative edge, `reconcile` validates it with
    `edge_valid_for_source`: it must originate from the source's own entity
    **and** its endpoint types must be permitted for its kind
    (`EdgeKind::permits`), so a compromised/buggy source cannot inject a
    cross-typed or foreign-origin edge (ill-typed edges are skipped and stay
    visible as divergence). Pure helpers unit-tested. (Repo tasks.md Phase 5
    SEC-B7.)
- [x] T-21: Prometheus `/metrics.prom` — consumer lag, edge counts by
  `status`, processed counters (§6 FR-22). *(Done: `src/metrics.rs` — a
  process-wide registry with `link_graph_events_processed_total{kind}`
  (incremented in `apply_event`) and two gauges refreshed from the DB at
  scrape time, `link_graph_edges{status}` + `link_graph_consumer_lag_seconds
  {entity}`; served at the root `GET /metrics.prom` (public — in
  `is_public_path`). Unit test (render) + DB-gated endpoint test.
  Reconciliation **divergence** is deferred with the reconciliation worker,
  T-20.)*
- [ ] T-22: Tracing + OpenTelemetry OTLP wiring; loco `/_health` /
  `/_ping`; graceful shutdown; Podman health check; non-root container.
  *(2026-08-01, while completing AU-3: **there is no working OTLP export
  anywhere in the family to copy**. Three crates — person, worker,
  event — carry an `src/observability/` module that builds an OTel
  `Resource` and then installs a plain JSON `tracing` subscriber, with
  the exporter and the `tracing_opentelemetry` layer commented out
  behind `// TODO: Initialize OTLP exporter`. Every other service,
  including this one, has nothing. Wiring a real exporter is therefore
  new work and a family-wide decision — which crate first, and what the
  collector story is in compose — not a copy job, so it is left open
  rather than half-done here. The shared capability matrix has been
  corrected to stop claiming it.)*
- [ ] T-23: Flip transport to the durable bus per entity as Fluvio
  topics go live; retire lazy verify-on-read per entity (design §5.1,
  event-bus.md §8).

### Cross-service identity suggestion (LNK-4)

Suggests `same_identity` (person ↔ worker) links by comparison, emitting
`matcher_suggested` edges an operator confirms — the design is
[§16 OQ-9](16-open-questions.md) (`cross-service-linking.md` §5.2). **Spec
round done and fully pinned 2026-08-04 — T-29 code may start.**

- [x] T-29: Cross-type `IdentityProbe` + comparator reusing the matcher
  crates' scoring primitives (pure, deterministic, unit-tested: a
  person/worker sharing an NHS number / name / DOB scores high; unrelated
  records score low; never consumes cross-service edges — §7 partition rule).
  *(Done: `src/suggest.rs` — `IdentityProbe { name, birth_date, gender,
  identifiers }` + `ProbeName` + `ProbeIdentifier` (exact match on a
  normalised `(scheme, value)` pair; blank identifiers are rejected at
  construction) + `compare_identity` → `IdentityMatchScore` (overall
  confidence plus the per-component breakdown, mirroring the family's
  score-breakdown convention). Depends on `person-matcher` 0.6.1 (path
  dependency, new `[dependencies]` entry) for `Scorer::jaro_winkler_similarity`
  and `Gender` — not `worker-matcher`, whose `Scorer`/`Gender` are
  near-duplicates and add nothing `IdentityProbe` needs, since both
  services' records reduce to the same lean probe before scoring.
  Weights: name 0.45 (family 0.6 / given 0.4), DOB 0.45, gender 0.10
  (documented in the module doc, mirroring
  `person-service-with-loco/AGENTS/matching.md`'s table style). A shared
  coded identifier short-circuits to `IDENTIFIER_MATCH_CEILING` (`0.99`);
  the weighted probabilistic path is capped below that at
  `PROBABILISTIC_CEILING` (`0.97`) so a perfect demographic agreement can
  never outrank a real identifier match. `score_dob_pair` here is a
  **fresh** implementation of the full six-row table documented in
  `AGENTS/matching.md` ("Birth Date Matching") rather than a reuse of
  `person-matcher`'s own private `score_dob_pair`, which only implements
  two of those six rows — a pre-existing doc/code drift in person-matcher
  this task did not want to either propagate or silently fix by reaching
  into private internals. No `pub` change was needed in person-matcher or
  worker-matcher. Every component is `None` (excluded, not zero) when
  either side lacks the field, and blank strings never spuriously match
  (SEC-M2/M3 "no spurious identity on absence"). 17 new unit tests
  (identifier ceiling, identifier mismatch on scheme or value, name+DOB
  match below the identifier ceiling, unrelated pair scores low, a
  gender-only mismatch costs less than a name mismatch, both-`Unknown`
  gender is a soft `0.5` not a spurious `1.0`, missing DOB degrades
  gracefully, wholly-absent fields never manufacture a match, blank names
  never spuriously match, blank identifiers rejected at construction,
  plus the six `score_dob_pair` table rows) + a `compile_fail` doctest
  pinning the §7 partition rule (`IdentityProbe` has no `From`/`Into`
  conversion to `person_matcher::Person`, so feeding a suggestion into a
  within-entity `MatchingEngine` cannot compile). `cargo test --lib`: 65
  passed (48 pre-existing + 17 new — the crate's own §11.1 "15 tests"
  note was already stale before this task, unrelated to T-29 and left
  alone). `cargo fmt --check` / `cargo clippy --all-targets -- -D
  warnings` clean. T-30 (candidate blocking) and T-31 (the periodic job)
  build on this module next.)*
- [x] T-30: Candidate blocking over the person / worker read feeds
  (OQ-9(a)): block on an exact shared coded identifier (NHS/SSN/other)
  when present, else `Soundex(family)` + birth-year; only same-block
  pairs are scored (bounds comparison to O(n + m + Σ|block|²) rather
  than O(n·m)). Score `< 0.7` is discarded (not stored); `>= 0.7` is a
  `matcher_suggested` candidate at that confidence — no auto-merge tier,
  every candidate (even an exact-identifier hit) still needs an operator
  confirm (T-32/T-33). Depends: T-29.
  *(Landed 2026-08-04: `generate_candidates(&[(EntityRef,
  IdentityProbe)], &[(EntityRef, IdentityProbe)]) -> Vec<IdentityCandidate>`
  in `src/suggest.rs`, still pure/offline like T-29 — no database, no
  HTTP, no clock; T-31 is what actually fetches the feeds and POSTs.
  Private `block_keys` computes the two-tier key exactly as pinned: each
  normalised `(scheme, value)` identifier pair when identifiers are
  present (a probe with several belongs to several identifier-blocks),
  else a single `Soundex(family) + birth_year` block when a usable
  family name and birth date are present; a probe with neither yields no
  key and is never compared to anything. The Soundex primitive is
  **reused**, not reimplemented: `person_matcher::Normalizer::phonetic_code`
  was already `pub` (wraps a real American-Soundex `soundex` module) —
  no `pub` change needed in `person-matcher`. Candidate generation
  buckets both sides by block key (`HashMap<BlockKey, Vec<usize>>`),
  only scores same-key person/worker index pairs, and dedupes on the
  `(person_index, worker_index)` pair (via a `HashSet`) so a pair
  reachable through more than one shared identifier is scored and
  returned at most once — matching the pinned `O(n + m + Σ|block|²)`
  bound, never the full `O(n·m)`. `SUGGESTION_THRESHOLD` (`0.7`, reused
  from `BatchDeduplicationRequest::threshold`'s default /
  `IMPORT_REVIEW_THRESHOLD`, not invented fresh) filters the output;
  everything below is discarded, never returned; there is no auto-merge
  tier — a `0.99` identifier-ceiling hit comes out the same
  `IdentityCandidate` shape as any other, for T-32/T-33 to route. 6 new
  unit tests, including the load-bearing blocking-boundary proof
  (`pairs_in_different_blocks_are_never_compared_even_when_score_would_qualify`):
  constructs a pair that scores `>= 0.7` (`>= 0.90` in the fixture) via
  a **direct** `compare_identity` call, then shows `generate_candidates`
  never returns it because a one-year birth-date difference puts the two
  probes in different `Soundex(family) + birth_year` blocks — proving
  the blocking bound is real, not merely that scoring is correct. Also
  covered: the identifier-block finds the sharing person and excludes an
  unrelated third record in a different block; the phonetic+birth-year
  fallback block scores a pair with no identifiers (returned confidence
  matches a direct `compare_identity` call); a low-scoring same-block
  pair (same family/Soundex/year, disagreeing given name, month/day, and
  gender) is compared but discarded; two shared identifiers produce one
  candidate, not two; every empty/unblockable-input combination returns
  an empty list without panicking. `cargo test --lib`: 71 passed (65
  pre-existing + 6 new). `cargo fmt --check` / `cargo clippy
  --all-targets -- -D warnings` clean. T-31 (the periodic job) builds on
  this next.)*
- [ ] T-31: The periodic suggestion job (aggregator-hosted, mirroring the
  reconcile worker's shape with the verb flipped): pull person + worker,
  block, compare, and POST `matcher_suggested` `same_identity` edges to
  person's `POST /api/persons/{id}/links`. Target URL
  `LINK_GRAPH_SUGGEST_URL_PERSON`, bearer `LINK_GRAPH_SUGGEST_TOKEN` — a
  **dedicated** token, not `LINK_GRAPH_RECONCILE_TOKEN` (different blast
  radius: reconcile only reads, this writes), same SEC-B7
  loopback-token-optional/remote-token-required rule as
  `src/reconcile.rs::source_auth_ok`. Interval
  `LINK_GRAPH_SUGGEST_SECS` (default 3600), same skip-first-tick
  pattern as `run_periodic`. The aggregator stays read-only to the
  world — it calls person's write API as an authenticated client; it
  never gains a write endpoint of its own (OQ-9(c)). Depends: T-30.
- [ ] T-32: Review + promotion, reusing **person's existing
  `review_queue` table/endpoints** (OQ-9(b)) — no new aggregator
  endpoint. Suggestions land as ordinary review-queue rows
  (`record_id_a` = person pid, `record_id_b` = worker pid — the column
  carries no FK, so a cross-service pair stores cleanly),
  `provenance = "matcher_suggested"` (the BLK-2 column, already wired),
  `detection_method = "cross_service_same_identity"`. Extend
  `review_decision`'s `confirmed` branch to also call
  `entity_links::upsert` with `provenance="operator", confidence=1.0`
  (idempotent — reasserts the same `edge_id`); extend its `rejected`
  branch to soft-delete the edge (`unlinked`). A reviewing client
  resolves the worker-side summary with its own `GET /api/workers/{id}`
  call (front-end-drift-accepted, no shared package). Depends: T-31.
- [ ] T-33: Governance + tests — suggested edges are `unverified` and
  never auto-promoted regardless of score (OQ-9(a)); the suggestion job
  audits every POST it makes (mirroring `audit_ctx` on person's link
  writes) and every run's counts; scale controls (OQ-9(d)):
  `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` (default 50, same shape as
  `BatchDeduplicationRequest::max_candidates`) bounds same-block
  comparisons per anchor record, `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN`
  (default 200) caps suggestions POSTed per run.

### Tests

- [x] T-24: Un-gated unit suite (§11.1) — `cargo test --lib` (15 tests:
  `graph::canonical` / `edge_status` / `single_view`, event decode,
  topic helpers, status/provenance token round-trips).
- [x] T-25: DB-gated integration suite (§11.2), `#[ignore]`-tagged
  (`tests/graph_endpoints.rs`) — boots the app, drives `apply_event`,
  and exercises the four read endpoints (projection, canonicalisation,
  status lifecycle, single-view, unlink, freshness, `400`s).
- [ ] T-26: Bus-gated round-trip + replay-rebuild suite (§11.3), feature
  `fluvio`.
- [ ] T-27: Governance tests — no-leak + audit (§11.4).
- [ ] T-28: Criterion benchmarks for `neighbors` / `edges` /
  `single-view` (§11.6).
