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
- [x] T-22: Tracing + OpenTelemetry OTLP wiring; loco `/_health` /
  `/_ping`; graceful shutdown; Podman health check; non-root container.
  *(Done 2026-08-05 — **the family's first working OTLP exporter**.)*

  The 2026-08-01 note below was right that there was nothing to copy:
  person, worker and event carry an `src/observability/` module whose
  exporter and `tracing_opentelemetry` layer are commented out behind
  `// TODO: Initialize OTLP exporter`, and every other service — this one
  included — had no such module at all. So this is new work, written
  against
  [`rust-tracing-opentelemetry-stack.md`](../../../agents/share/rust-tracing-opentelemetry-stack.md)
  rather than ported from a sibling.

  **What landed** (`src/observability.rs`): `OTLP_SERVICE_NAME` /
  `OTLP_ENDPOINT` config with the shared doc's defaults; one OTel
  `Resource`; an OTLP/**gRPC** `SdkTracerProvider` (batch) and
  `SdkMeterProvider` (periodic); a `tracing_opentelemetry` bridge layer
  installed **alongside** loco's own fmt layer and `EnvFilter`, via loco
  1.0's `Hooks::init_logger` seam (which exists precisely so an app can
  compose its own layers — `logger::init_env_filter` / `init_layer` are
  public for this); `Hooks::on_shutdown` flushing both providers; and a
  `trace_mw` layer that opens one span per request, records an
  `http.server.request.duration` histogram, and stamps the W3C
  `traceparent` on the response (the shared doc's *Where to look first*
  line, previously aspirational).

  **Versions**: `opentelemetry`/`_sdk`/`-otlp`/`-semantic-conventions`
  0.32, `tracing-opentelemetry` 0.33 — *not* the 0.27/0.28 person's
  `Cargo.toml` pins. Those pins were never exercised (their exporter is
  still commented out) and 0.27's `install_batch(runtime::Tokio)`
  pipeline API no longer exists upstream.

  **Two behaviours worth not re-litigating.** Export is **on by
  default** (the shared doc's `OTLP_ENDPOINT` default is a real endpoint,
  and it describes no activation flag — unlike `<ENTITY>_REQUIRE_AUTH`);
  the escape hatch is `OTLP_ENDPOINT=""`, a value of the documented
  variable rather than a second flag, and it makes `init_logger` return
  `false` so loco's untouched logger runs. And **`RUST_LOG` now also
  governs what is exported**, since the filter sits above both sinks:
  loco's module whitelist is what keeps the trace stream to this
  service's own spans.

  **Verified, not inferred.** `tests/otlp_export.rs` and
  `tests/otlp_middleware.rs` stand up a real in-process OTLP/gRPC
  collector (the generated `TraceServiceServer`/`MetricsServiceServer`
  on an ephemeral port) and assert on the decoded protobuf: the span
  arrives with the configured `service.name`, its `tracing` fields as
  OTel attributes, and a trace id that **matches the `traceparent` the
  response carried**. Plus a live boot against Postgres with **no**
  collector: `GET /_health` → `200` with a `traceparent`, and a clean
  `SIGTERM` shutdown. Neither test is `#[ignore]`d and neither needs a
  database, so both run in a plain `cargo test`.

  **One thing this found.** With no collector, the first live boot logged
  *nothing* — loco's `EnvFilter` whitelist has no `opentelemetry*` entry,
  so every failed export was invisible, which looks exactly like success.
  `with_exporter_diagnostics` widens the filter for those targets (only
  when the operator has not supplied their own `RUST_LOG` /
  `override_filter`); a failing export now logs
  `BatchSpanProcessor.ExportError` once per batch interval.

  *Correction (2026-08-29, PRO-P25):* this note previously read "still
  open from this task's original wording: the Podman health check and
  non-root container hardening" — stale. Reading the `Dockerfile`
  confirms both are present and have been since it was added
  (2026-08-03, `Add a production Containerfile for link-graph-service`):
  `USER linkgraph` (non-root) and a `HEALTHCHECK --interval=30s
  --timeout=3s --start-period=10s --retries=3 CMD curl --silent --fail
  http://localhost:5160/_health || exit 1` instruction (with a documented
  caveat that Podman's default OCI image format ignores `HEALTHCHECK`
  unless built with `--format docker`, which is why the
  `examples/compose/` stacks also set an equivalent compose-level
  `healthcheck:`). T-22 has no open container-hardening work; `/_health`
  / `/_ping` and graceful shutdown are loco's, as already noted.
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
  `person-service-with-loco/agents/matching.md`'s table style). A shared
  coded identifier short-circuits to `IDENTIFIER_MATCH_CEILING` (`0.99`);
  the weighted probabilistic path is capped below that at
  `PROBABILISTIC_CEILING` (`0.97`) so a perfect demographic agreement can
  never outrank a real identifier match. `score_dob_pair` here is a
  **fresh** implementation of the full six-row table documented in
  `agents/matching.md` ("Birth Date Matching") rather than a reuse of
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
- [x] T-31: The periodic suggestion job (aggregator-hosted, mirroring the
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
  *(Landed 2026-08-04: `src/suggest.rs` became `src/suggest/mod.rs`
  (unchanged T-29/T-30 content) plus the new `src/suggest/job.rs`.
  `IdentitySource`/`SuggestionSink` traits (mirroring
  `reconcile::AuthoritativeSource`) keep the fetch→block→compare→post
  pipeline testable against mocks; `HttpIdentitySource`/
  `HttpSuggestionSink` are the real `reqwest` implementations. **A
  gap this task surfaced rather than assumed:** neither service has a
  plain "list everything" endpoint — only `search?q=…`, and an empty
  `q` parses to an empty Tantivy `BooleanQuery` (zero hits). The query
  grammar's `*` token, though, parses to `UserInputLeaf::All` →
  `AllQuery`, matching every document (confirmed against the vendored
  `tantivy-query-grammar` 0.22 source) — `q=*` plus the existing
  `limit`/`offset` pagination is how this job enumerates a full
  collection today; a real bulk-list endpoint is a
  person/worker-service change, out of this task's scope. Introduced
  **`LINK_GRAPH_SUGGEST_URL_WORKER`** beyond OQ-9(c)'s literal text
  (which names only the person write target) since the job cannot
  produce a candidate without also reading worker's collection;
  named to match the established `LINK_GRAPH_RECONCILE_URL_<ENTITY>`
  convention. `reconcile::source_auth_ok` was widened to `pub(crate)`
  and reused verbatim (not reimplemented) for SEC-B7 on all three
  URLs. Person/worker → `IdentityProbe`: `name.given` space-joined
  (mirroring both services' own Tantivy indexers); an identifier's
  scheme prefers the FHIR `system` URI (the cross-service-stable
  field, per person's own `matching::adapter::route_identifier`),
  falling back to `identifier_type` only when `system` is blank;
  gender tokens map one-for-one, `"unknown"` → the real `Unknown`
  variant, anything unrecognised → `None`. Wired into
  `App::after_routes`, spawned only when `sources_from_env()`
  resolves; needs no database handle (pure HTTP client traffic).
  `LINK_GRAPH_SUGGEST_MAX_CANDIDATES`/`_MAX_EDGES_PER_RUN`, per-POST
  audit, and the non-auto-promotion governance test are deliberately
  **not** here — T-33's job; this task applies only its own defensive
  `MAX_FETCH_OFFSET = 10_000` fetch-pagination cap (mirroring person's
  SEC-G7 `MAX_SEARCH_OFFSET`). 14 new unit tests (wire-mapping fixtures,
  every gender token, the SEC-B7 accept/reject matrix, and the mocked
  end-to-end pipeline). `cargo test --lib`: 85 passed (71 pre-existing
  + 14 new). `cargo fmt --check` / `cargo clippy --all-targets -- -D
  warnings` clean. No live HTTP round-trip test was added — mirroring
  reconcile.rs's own posture of only unit-testing the pure/mocked
  logic, not the real `reqwest` fetch loop; a live round-trip is left
  to T-33's governance-test pass, which needs live services anyway to
  prove non-auto-promotion. T-32 (person's review-queue promotion) and
  T-33 (governance/scale/audit) build on this next.)*
  **Correction, landed the same day, verified live:** the `q=*`
  enumeration mechanism above turned out to be unreliable against a
  real running person-service — a live test (not a mock) found a
  small-`limit` `q=*` page could come back **empty** even with matching
  data present, because person's Tantivy index (a separate artefact
  from its database) had drifted from it. `HttpIdentitySource::fetch_all`
  now pages person's and worker's new database-backed `GET
  /<plural>?limit=&offset=` list endpoints instead
  (`person-service`/`worker-service`'s own `CHANGELOG.md` entries carry
  the investigation and the fix on that side); the Tantivy search index
  is no longer consulted by this job at all. Also added: two
  `#[ignore]`d live tests
  (`tests/live_suggest_fetch.rs`, `tests/live_suggest_full_pipeline.rs`)
  that drive this crate's real `HttpIdentitySource`/`HttpSuggestionSink`/
  `run_suggestion_pass` against genuinely running peer services — proven
  by hand against real person-service + worker-service instances: fetch
  enumerated 25/21 real records across 4 pages each with zero loss/
  duplication, and the full pipeline (fetch both sides → block/score →
  POST) found the one seeded shared-identifier candidate and confirmed
  the `matcher_suggested` `same_identity` edge landed on person's real
  `entity_links` via an independent follow-up `GET .../links`. `cargo
  test --lib`: still 85 passed (unit-test count unchanged — the fix is
  in the HTTP call shape, not the pure logic). `cargo fmt --check` /
  `cargo clippy --all-targets -- -D warnings` clean.
- [x] T-32: Review + promotion, reusing **person's existing
  `review_queue` table/endpoints** (OQ-9(b)) — no new aggregator
  endpoint. *(done 2026-08-04; landed entirely in
  `person-service-with-loco` — see that crate's own `spec/13-tasks.md`
  T-32 entry and `CHANGELOG.md` for the full breakdown; this entry
  records the decisions from the link-graph side.)*
  - [x] **Entity-type-ambiguity wrinkle, resolved.** OQ-9(b)'s premise —
    `record_id_a` = person pid, `record_id_b` = worker pid — turned out
    to require more than "the column carries no FK": person's existing
    `db::review_queue::upsert` **normalizes** `(record_id_a,
    record_id_b)` by raw `Uuid` comparison before insert (correct for
    within-entity dedup, where the two ids are the same entity type and
    which column holds which is meaningless; actively wrong for a
    cross-service pair, where reordering would silently swap the
    person/worker columns for roughly half of all pairs — whichever
    side happens to sort first as a `Uuid`). Resolved by adding a
    **second, non-reordering** insert path,
    `db::review_queue::upsert_cross_service`, used only for
    cross-service rows; the fixed person-then-worker convention is
    honoured by never reordering, not by relying on `upsert`'s
    normalization to happen to preserve it.
  - [x] **Where the review-queue write happens: person's own
    `create_link` handler**, not a second call from this crate's
    suggestion job (T-31). Considered and rejected: a follow-up POST
    from link-graph's job would leave a real failure mode (edge
    created, review-queue write fails independently, aggregator has no
    transaction spanning both calls) and would complicate T-31's job
    for no benefit, since `review_queue` is person's own table and no
    other service writes into it. Keeping the write inside
    `create_link` — gated on `kind = "same_identity"` +
    `provenance = "matcher_suggested"` — means T-31's job needed no
    behavioural change at all beyond one addition (below): it already
    POSTs everything the write needs.
  - [x] The one addition on this crate's side: `src/suggest/job.rs`'s
    `HttpSuggestionSink::post_suggestion` now sends the T-29
    `IdentityMatchScore` breakdown as a `score_breakdown` JSON object
    alongside `kind`/`to_ref`/`confidence`/`provenance`, so person's
    review-queue row carries the per-component evidence (identifier /
    name / DOB / gender), not just the final confidence number.
    `tests/live_suggest_full_pipeline.rs` (manual, `#[ignore]`d, not in
    any CI stage) extended to also confirm the review-queue row landed
    with its breakdown, closing the T-31→T-32 loop end to end against
    two real running services.
  - [x] `review_decision`'s `confirmed` branch reasserts the edge via
    person's own `upsert_and_emit` (same idempotent
    `(from_pid, kind, to_ref, valid_from)` key `create_link` uses —
    the SAME edge id, never a new one) rather than a bare
    `entity_links::upsert` call, so promotion also emits the normal
    `linked` event under the active transport, exactly as an operator
    asserting the edge directly would; `rejected` soft-deletes it
    (`unlinked`) via a new `entity_links::find_active_by_key` natural-key
    lookup (a review row carries no `edge_id`). Both gated on **both**
    `provenance` and `detection_method`, pinned by a DB-gated regression
    test proving an ordinary within-entity decision is unaffected.
  - [x] A reviewing client resolves the worker-side summary with its own
    `GET /api/workers/{id}` call (front-end-drift-accepted, no shared
    package) — unchanged from OQ-9(b)'s original plan; nothing in this
    crate needed to change to support it.
  - **Acceptance:** `cargo test --lib` on this crate: still 85 passed
    (unchanged — the change here is the POST body shape, not the pure
    comparator/blocking logic already covered by T-29/T-30's suite);
    `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings`
    clean. Person-service's own DB-gated suite (4 new tests +
    pre-existing 21 `--lib` + 25 `api_integration_test`, all green
    against Postgres 18) is the acceptance evidence for the actual
    review/promotion behaviour, since that is where it lives.
- [x] T-33: Governance + tests — suggested edges are `unverified` and
  never auto-promoted regardless of score (OQ-9(a)); the suggestion job
  audits every POST it makes (mirroring `audit_ctx` on person's link
  writes) and every run's counts; scale controls (OQ-9(d)):
  `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` (default 50, same shape as
  `BatchDeduplicationRequest::max_candidates`) bounds same-block
  comparisons per anchor record, `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN`
  (default 200) caps suggestions POSTed per run. *(done 2026-08-04,
  closing LNK-4.)*
  - [x] **`LINK_GRAPH_SUGGEST_MAX_CANDIDATES`.** `suggest.rs` gained
    `generate_candidates_bounded(persons, workers, max_candidates)`; the
    T-29/T-30 `generate_candidates` is now a thin wrapper calling it with
    `DEFAULT_MAX_CANDIDATES` (`50`). Investigated
    `BatchDeduplicationRequest::max_candidates`'s actual semantics before
    copying them (`persons[i + 1..].iter().take(req.max_candidates)`):
    a fixed number of candidates off the front of an order-preserving
    slice, **per anchor**, not a globally-shared budget and not
    score-sorted. `generate_candidates_bounded` matches exactly —
    `worker_indexes.iter().take(max_candidates)` per person anchor
    within a block, `worker_indexes` in the `workers` slice's own input
    order — so truncation is deterministic (always the same prefix), not
    `HashMap`-iteration-order-dependent. New load-bearing test
    (`max_candidates_caps_same_block_comparisons_per_anchor_deterministically`):
    a 10-worker identifier-sharing block capped at 3 returns exactly
    `workers[..3]`, twice in a row.
  - [x] **`LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN`.** `job.rs`'s
    `run_suggestion_pass` now takes `max_candidates` and
    `max_edges_per_run` explicitly (threaded from `run_periodic`'s env
    reads); when `generate_candidates_bounded` returns more candidates
    than the cap, only the **highest-confidence** `max_edges_per_run`
    survivors are `POSTed` (descending `IdentityMatchScore::confidence`,
    ties broken on the `(person, worker)` id pair for full
    determinism) — chosen deliberately over posting order or first-N,
    since an identifier-ceiling match is stronger evidence than a
    borderline probabilistic one and should survive a cut-short run.
    Dropped candidates are not lost: the next pass re-fetches and
    re-scores the same data (idempotent) and finds them again. Two new
    tests: the cap threading through to blocking, and the
    highest-confidence-survives-the-cut proof (three independent pairs
    at three known confidences via the DOB-proximity table, capped to 2,
    proving the lowest-confidence pair is the one dropped).
  - [x] **Audit — every POST.** Investigated before building: person's
    `create_link` handler already writes an **unconditional**
    best-effort `person_link` audit row
    (`state.audit_log.log_create("person_link", link.id, new_values,
    &audit_ctx(&caller))`) for every link creation, `matcher_suggested`
    included — this job's only write goes through that exact handler.
    Building a second audit trail here would have been a redundant audit
    of the same event from the wrong side of the wire. Regression-pinned
    with a new test on person's side rather than left as an
    unverified doc claim: `person-service-with-loco`'s
    `tests/cross_service_link_review.rs::matcher_suggested_link_creation_is_audited`
    POSTs a `matcher_suggested` link and confirms a `CREATE`/
    `person_link` audit row exists naming the same link id with
    `provenance = "matcher_suggested"` in its `new_values` snapshot.
  - [x] **Audit — every run's counts.** `crate::reconcile`'s own
    periodic pass — the closest sibling — records its summary only as a
    live Prometheus gauge plus a `tracing::info!` line, sufficient there
    because "did the last pass find drift" only needs the *current*
    value. This job's summary is richer and OQ-9(d) asks for it to
    survive a missed scrape or a restart, which a gauge cannot do. New
    `suggestion_runs` table (migration
    `m20260804_000001_suggestion_runs`) + model
    (`src/models/suggestion_runs.rs`, `Model::record`): one durable row
    per **completed** pass (a fetch failure records nothing, matching
    `run_periodic`'s existing log-and-retry posture). `run_periodic` now
    takes a `DatabaseConnection` (threaded from `ctx.db.clone()` in
    `app.rs::after_routes`) purely for this write — the job's actual
    work stays HTTP-only. Also added, mirroring `reconcile`'s existing
    metrics idiom for live/alertable visibility on top of the durable
    history: a `link_graph_suggestion_last_run` gauge vec (labelled
    `stat`) set from each pass's stats. DB-gated test
    `tests/suggestion_runs.rs` proves two completed passes accumulate
    two rows (a history, not a last-value slot) and the stored counts
    round-trip through Postgres exactly.
  - [x] **Governance capstone: unverified/never-auto-promoted, live.**
    Ran live rather than mocked — a single real running person-service
    was enough (the property under test lives entirely on person's
    write/review-queue side; a live worker-service was not needed since
    the worker side of the pipeline is synthetic-probe-injectable
    without weakening the proof). New
    `tests/live_suggest_never_promoted.rs`
    (`near_ceiling_identifier_match_is_never_auto_promoted`, manual,
    `#[ignore]`d): a real seeded person sharing a coded identifier with
    an in-test synthetic worker probe drives the real
    fetch→block→compare→POST pipeline
    (`HttpIdentitySource`/`HttpSuggestionSink`, no mocks on the write
    side); the resulting edge is confirmed at
    `IDENTIFIER_MATCH_CEILING` (`0.99`) with `provenance =
    "matcher_suggested"` (never `operator`/`1.0`), and the review-queue
    row is confirmed `pending` (never `confirmed`/`automerged`) despite
    scoring well above the family's own within-entity
    `auto_merge_threshold` (`0.95`) — proving OQ-9(a)'s "no auto-merge
    tier for cross-service identity" live rather than by inspection. A
    second identical pass (idempotent fetch + upsert) reasserts the same
    edge and leaves the row still `pending`, ruling out any background
    promotion path. Verified by hand against a real running
    person-service (test-db-backed): pass 1 and pass 2 both green.
    Promotion itself (`review_decision`'s `confirmed` branch) stays
    regression-pinned on person's own side
    (`cross_service_link_review.rs::confirming_promotes_the_edge_without_duplicating_it`)
    rather than duplicated here.
  - **Acceptance:** `cargo test --lib` on this crate: 95 passed (was 85;
    +10: 3 in `suggest.rs`, 4 in `suggest/job.rs`, 2 in `metrics.rs`, 1
    in `models/suggestion_runs.rs`). `cargo fmt --check` / `cargo
    clippy --all-targets -- -D warnings` clean. DB-gated tests green against a real Postgres 18
    (`concealment`, `governance`, `graph_endpoints`, `idempotency`,
    `lazy_verify`, `reconcile`, `suggestion_runs` — 19 tests total, run
    individually; `scripts/ci-check.sh test-db` for this crate currently
    aborts after the first test binary because the **pre-existing**
    `live_suggest_fetch.rs`/`live_suggest_full_pipeline.rs` manual live
    tests (landed under T-31/T-32, same day) are swept up by the
    blanket `cargo test -- --ignored` the `test-db` CI stage runs,
    despite their own doc comments saying they are "not part of any CI
    stage" — confirmed via `git stash` to already fail identically at
    T-32's landing commit, so this is not a T-33 regression, but is
    worth a follow-up task to give live/manual tests their own tier
    excluded from the `--ignored` sweep). Person-service's own full
    DB-gated suite (21 `--lib` + 25 `api_integration_test` + 5
    `cross_service_link_review`, the new audit test included, + 1
    `enforcement` + 1 `seed_examples_db`) is green against Postgres 18;
    `cargo test --lib`: 315 passed (unchanged from T-32 — no new `src/`
    unit tests added on person's side, only one new `tests/`
    integration test).

**LNK-4 complete (2026-08-04).** All five sub-tasks (T-29 comparator,
T-30 blocking, T-31 periodic job, T-32 review-queue bridge, T-33
governance/scale/audit) are landed. The cross-service `same_identity`
suggestion feature is real end to end: person + worker records are
compared with blocking, scored candidates are POSTed to person as
`matcher_suggested` edges, they surface in person's review queue for an
operator to confirm or reject, confirmation promotes to
`operator`/`1.0` via the normal link-write path, and — T-33's
contribution — the whole pipeline is scale-bounded on both axes,
audited (POST-level via person's existing audit trail, run-level via
this crate's new `suggestion_runs` table), and live-proven to never
auto-promote regardless of score. `DOC-6` (documentation harmonisation
for this crate) was queued behind this chain finishing and is now
unblocked.

### Reconciliation gauge hardening

- [x] T-34 (M): *(resolved 2026-09-04.)* Label `link_graph_reconciliation_divergence` per entity,
  closing the runbook's first "sharp edge". *(verified:
  `agents/share/runbooks/reconciliation-divergence.md` states plainly —
  "Both entity workers write the same metric name, so its value is
  'whatever the *most recently completed* pass of *either* entity
  found' — not a sum, not per-entity. A converged `case` pass can
  overwrite a diverging `person` pass's `47` with `0` a moment later,
  and you'd never know from the metric alone." `src/metrics.rs` confirms
  the code: `reconciliation_divergence` is a bare `IntGauge` (no
  labels), the **only** reconciliation-adjacent metric in this crate
  built that way — `edges` is an `IntGaugeVec` labelled `["status"]`,
  and `consumer_lag_seconds` is an `IntGaugeVec` labelled `["entity"]`
  for the exact same "per source entity" reason. `src/reconcile.rs`'s 7
  unit tests (`diff_finds_missing_and_extra_by_edge_id`,
  `bulk_response_deserializes_*`, `source_auth_requires_a_token_*`)
  cover none of this — no test touches `reconciliation_divergence` at
  all)*. Change it to an `IntGaugeVec` labelled `["entity"]`, mirroring
  `consumer_lag_seconds`'s existing pattern exactly; `reconcile()`
  already knows its `AuthoritativeSource::entity()`, so the label is
  free at the call site. Spec (this file) + code + test: a unit or
  DB-gated test that a `case` pass's divergence value and a `person`
  pass's divergence value are independently readable (one converging to
  `0` must not zero the other's stale-but-real count). Update the
  runbook's "sharp edge" framing once fixed (out of scope for this
  crate's own files, but note it in the PR).
  **Acceptance:** `GET /metrics.prom` exposes
  `link_graph_reconciliation_divergence{entity="person"}` and
  `{entity="case"}` (etc.) as independent series; `cargo test --lib`
  green.
  - **Resolved.** `reconciliation_divergence` in `src/metrics.rs` is now
    an `IntGaugeVec` labelled `["entity"]` (mirroring
    `consumer_lag_seconds`'s existing pattern exactly, as suggested);
    `reconcile()` sets it via `source.entity().as_str()` at the call
    site. Two new `src/metrics.rs` unit tests pin the labels are
    independent; a new DB-gated
    `tests/reconcile.rs::reconciliation_divergence_gauge_is_independent_per_entity`
    proves a converged `person` pass does not zero a diverging `case`
    pass's real count. The shared runbook
    (`agents/share/runbooks/reconciliation-divergence.md`) is updated
    in the same PR (a four-part change per this crate's own
    shared-contract rule) to record the sharp edge as closed.

- [x] T-35 (S): *(resolved 2026-09-04.)* Add a per-entity "last reconciliation pass" gauge,
  closing the runbook's second "sharp edge". *(verified: the same
  runbook section — "It is also only updated on a successful fetch. A
  pass that fails (timeout, non-2xx, malformed JSON) leaves the gauge
  exactly where it was — a genuine `0` and a 'hasn't run since boot' `0`
  look identical. The *only* per-pass signal, success or failure, is
  the log line." `src/reconcile.rs`'s `reconciliation pass complete` /
  `reconciliation pass failed` `tracing` lines are real (confirmed at
  the `run_periodic`-style call site, line ~277) but nothing exports
  them as a metric)*. Add an `IntGaugeVec` (e.g.
  `link_graph_reconciliation_last_success_unixtime`, labelled
  `["entity"]`, mirroring `suggestion_last_run`'s existing
  `IntGaugeVec` pattern) set on every successful pass, so an operator
  can distinguish "converged 2 minutes ago" from "never run" from
  Prometheus alone, without cross-referencing logs. Optionally also
  export a per-entity pass outcome counter (success/failure) for
  alerting on a run of failures. Spec + code + test: a test that a
  failed pass leaves the divergence gauge unchanged while the
  last-success gauge also stays unchanged (proving the two together
  disambiguate what one gauge alone cannot).
  **Acceptance:** `GET /metrics.prom` exposes a per-entity
  last-successful-pass signal; a DB-gated or unit test pins that a
  failed pass does not advance it; `cargo test --lib` green.
  - **Resolved.** Added `reconciliation_last_success_unixtime`
    (`IntGaugeVec` labelled `["entity"]`) to `src/metrics.rs`;
    `reconcile()` sets it via `Utc::now().timestamp()` immediately
    after computing the divergence count — i.e. only once the fetch and
    read-model query have both already succeeded, so an early `?`
    on either leaves it untouched. The optional pass-outcome counter is
    not added (out of scope for this pass — the two gauges together
    already satisfy the acceptance criterion). A new DB-gated
    `tests/reconcile.rs::reconciliation_last_success_gauge_is_unchanged_by_a_failed_pass`
    proves a failed pass advances neither gauge while a prior real
    (non-zero) divergence value survives untouched.

- [x] T-36 (S): Add an operator-forceable reconciliation pass.
  *(verified: the same runbook states "There is **no** endpoint, task,
  or admin route to force a pass on demand, list the last-run time per
  entity, or see a pass/fail counter — confirmed absent, not merely
  undocumented. The only lever an operator has is restarting the
  process… or restarting with a smaller interval temporarily." — an
  explicitly documented operational gap, not a design choice recorded
  as final)*. Add a `destructive`-gated admin endpoint or loco CLI task
  (mirroring case-service's `subject_of` bulk-dump gating pattern — a
  machine peer `svc=true` or an `access=admin` caller only, per
  `agents/share/authorization-attributes.md` §4/§9) that runs one
  reconciliation pass for a named entity immediately, reusing
  `reconcile()`'s existing logic rather than duplicating it. Spec + code
  + test: a DB-gated test that the forced pass updates the T-34/T-35
  gauges exactly as the periodic worker does; ABAC tests (401/403/200)
  matching the family's existing guard-test matrix.
  **Acceptance:** an authorized operator can trigger and observe one
  reconciliation pass without restarting the process; the runbook's
  "confirmed absent" line is no longer true.
  **Resolution (2026-09-05):** chose the admin-endpoint path (a loco
  CLI task cannot be triggered from a running operator tool without
  process/exec access, and case-service's own reference pattern for
  this class of privileged action is an HTTP endpoint, not a CLI task).
  `src/auth.rs::authorize_reconcile` (a new `Action::Destructive`
  check against the `link_graph` entity, alongside the existing
  `may_see_governed`/`enforce`) gates the new `POST
  /api/admin/reconcile/{entity}` (`src/controllers/admin.rs`), which
  builds a `reconcile::HttpAuthoritativeSource::from_env_for(&entity)`
  — `404` when `entity` is unrecognised or has no
  `LINK_GRAPH_RECONCILE_URL_<ENTITY>` configured, since there is
  nothing to force — and otherwise calls
  `reconcile::reconcile(&ctx.db, &source)` directly, the exact same
  call the periodic worker (`run_periodic`) makes, so the two paths
  cannot drift. This is a control-plane action, not a new **link**-write
  endpoint of this service's own (the `AGENTS.md` "read-only to the
  world" invariant governs edge creation/withdrawal, which still lives
  only in each owning entity service): it triggers a repair of this
  aggregator's *own* read-model against a source it is already
  configured to trust, the same category of action the periodic worker
  already performs unattended. New DB-gated test
  (`tests/force_reconcile.rs`, its own binary — mints real PASETO
  tokens against a throwaway key set, same shape as `tests/
  concealment.rs`): no token → `401`; an authenticated caller without
  `access=admin`/`svc=true` → `403` (the blanket guard's coarse `read`
  check on `link_graph` would admit the path, but the handler's own
  destructive check still refuses it); an unconfigured entity → `404`
  even for an authorised admin; an authorised admin against a
  mocked-HTTP `case` source (a tiny local Axum server spun up inside
  the test, serving one fixed `subject_of` edge) → `200`, reporting
  `divergence_count: 1` and advancing the same
  `reconciliation_last_success_unixtime` /
  `reconciliation_divergence` gauges T-34/T-35 already cover; a second
  forced pass against the now-repaired read-model converges to `0`,
  proving this calls the real repair logic, not a stub. Verified:
  `cargo build --lib`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --lib` (106 passed), `cargo test -- --ignored` against a
  real Postgres (all suites green, including the new binary), `cargo
  fmt --check`.

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
- [x] T-27: Governance tests — no-leak + audit (§11.4). *(resolved
  2026-09-05.)* `tests/concealment.rs` covered only `/api/edges`, but
  §11.4 requires no-leak proof across `/edges`, `/neighbors`, **and**
  `/single-view`. *(verified: `src/controllers/graph.rs` — all three
  handlers call `auth::conceal_governed` (lines 264/321/346), but
  `tests/concealment.rs` only drove `GET /api/edges`.)* Extended the
  existing test (same fixtures, tokens, and restrictive ABAC policy) to
  also assert: a case-authorised caller's `GET /api/neighbors/{person}`
  and `GET /api/single-view/{person}` both surface the `subject_of`
  edge/affiliation, while a non-case caller's calls to the same two
  endpoints do not — and that `single-view`'s surfaced read is audited
  under `read_single_view` exactly once (matching `read_edge`'s existing
  once-per-surfaced-read pin), with no further row on the concealed
  read. Verified: `cargo build --all-targets`, `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, and `scripts/ci-check.sh
  test-db link/link-graph-service-with-loco` all clean (106 lib tests +
  the extended `concealment.rs` test, plus the rest of the DB-gated
  suite, all passing); no `Cargo.lock` churn.
- [ ] T-28: Criterion benchmarks for `neighbors` / `edges` /
  `single-view` (§11.6).
