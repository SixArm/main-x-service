## 14. Implementation Status

**Read API + LNK-4 cross-service suggestion implemented (2026-08-04).**
The read-model core, the real Fluvio consumer, reconciliation, offline
PASETO + ABAC governance, and the cross-service `same_identity`
suggestion job all compile clean, are clippy-clean
(`--all-targets --all-features`), and are unit-tested (`cargo test
--lib`: **104 passed** as of 2026-08-29, up from the v1-core-only 15 of
2026-07-09 — see §13 T-24/T-29/T-30/T-31/T-33 for the incremental
counts). Built from
the canonical design doc
[`cross-service-linking.md`](../../../agents/share/cross-service-linking.md)
(this is its §4.3 read-model aggregator) and the
[`event-bus.md`](../../../agents/share/event-bus.md) §9 consumer model.
**§13 is the live, authoritative task-by-task record; this section is
a periodically-refreshed summary of it, not a substitute — when they
disagree, §13 wins.**

### 14.1 What is implemented

- **Scaffold** — loco.rs crate (`Cargo.toml`, `src/lib.rs` +
  `src/bin/main.rs` with the family lint header, `src/app.rs` `Hooks`,
  `config/{development,test,production}.yaml`, the `migration/` bridge
  crate). Read-only to the world; no write controllers of its own
  (§1.3, §9). Depends on the sibling
  **[`entity-ref`](../../entity-ref-rust-crate)** crate for the shared
  `EntityType` / `EntityRef` / `EdgeKind` / `Sensitivity` contracts (the
  T-2/T-3 "copy" is superseded — see §13).
- **Persistence** — six tables, hand-written SQL bridged via
  `include_str!` (§10): the three v1 read-model tables `edges` (+
  `edges_from` / `edges_to` / `edges_status` indexes), `entity_presence`,
  `consumer_offsets`; plus `processed_events` (idempotency, T-6),
  `audit_log` (governed-edge access, T-17), and `suggestion_runs`
  (per-pass job audit, T-33, §10.7). SeaORM entities under
  `src/models/_entities/`. `with-chrono` time types (§10.5, OQ-5).
- **Pure projection logic** (`src/graph.rs`, DB-free, fully unit-tested)
  — `canonical` (symmetric ordering, FR-6), `edge_status`
  (unverified/verified/dangling lifecycle, FR-9), `single_view`
  (same-identity unification + affiliation walk, FR-15), `repoint`
  (merge repointing, FR-12), plus `EdgeStatus` / `Provenance` value
  types.
- **Apply seam** (`src/events.rs`) — `apply_event` / the idempotent
  `apply_event_idempotent` (T-6, dedup on `event_id` via
  `processed_events`) dispatch `created`/`deleted` → presence (+
  incident-edge status recompute), `linked` → canonical edge upsert,
  `unlinked` → edge removal, `merged` → central repointing (T-9). Typed
  `LinkedEvent` / `UnlinkedEvent` decode (closed-registry `edge_kind`
  validation) with DB-free tests. Every consumed event advances the
  per-topic freshness watermark.
- **Read API** (`src/controllers/graph.rs`, enveloped
  `{success,data,error}` + `as_of`) — `GET /api/neighbors/{ref}`
  (bounded BFS, `kind`/`direction`/`depth≤2`), `GET /api/edges`
  (from/to/kind/status), `GET /api/single-view/{ref}`,
  `GET /api/health/freshness`. `400` on malformed ref / unknown kind /
  over-cap depth. Loco `/_health` + `/_ping` retained.
- **Real Fluvio bus consumer** (T-6, BUS-2, `src/consumer.rs`, done
  2026-08-03) — one task per entity topic, behind this crate's own
  `fluvio` Cargo feature. Off with no `LINK_GRAPH_FLUVIO_ENDPOINT`
  configured; a configured endpoint without the feature refuses to
  start rather than silently doing nothing. See §13 T-6 for the resume-
  position design decision (delegated to Fluvio's own named-consumer
  offset management, not `consumer_offsets.offset_val`).
- **Lazy verify-on-read** (T-10, `src/probe.rs`) — interim integrity:
  a one-shot `GET /{id}` per unresolved endpoint, cached in
  `entity_presence`, non-redirecting (SEC-B11).
- **`case ↔ person` governance** (T-16/T-17, `src/auth.rs`) — access
  control + concealment on every read path that could surface a
  `subject_of`/`about` edge, plus audit of governed reads/writes to
  `audit_log`. Masking parity with the case service (T-18) remains open.
- **Offline PASETO v4.public + ABAC** (T-19, `src/auth.rs`) — the
  blanket read guard, gated on `LINK_GRAPH_REQUIRE_AUTH`.
- **Reconciliation worker** (T-20, `src/reconcile.rs`) — periodic diff
  of the read-model against each service's authoritative `entity_links`
  (currently case + person + worker sources configured), scoped
  per-source-entity (SEC-B1), authenticated (SEC-B7).
- **Prometheus `/metrics.prom`** (T-21, `src/metrics.rs`) — consumer
  lag, edge counts by status, processed counters, plus the LNK-4
  suggestion-run gauge (below).
- **OpenTelemetry OTLP export** (T-22, `src/observability.rs`, landed
  2026-08-05) — **the first working exporter anywhere in this family**;
  the other services either carry a commented-out stub (person, worker,
  event) or nothing. Traces and metrics over OTLP/**gRPC**, bridged from
  `tracing` and installed through loco 1.0's `Hooks::init_logger` seam so
  loco's own `EnvFilter` policy and log format are reused rather than
  re-derived; flushed from `on_shutdown`; `trace_mw` adds a per-request
  span, an `http.server.request.duration` histogram, and the W3C
  `traceparent` response header. On by default (`OTLP_ENDPOINT`,
  `OTLP_SERVICE_NAME`); `OTLP_ENDPOINT=""` turns it off. Proved against a
  real in-process OTLP/gRPC collector in `tests/otlp_export.rs` +
  `tests/otlp_middleware.rs` (neither `#[ignore]`d, neither needing a
  database).
- **Cross-service `same_identity` suggestion (LNK-4, T-29..T-33,
  `src/suggest/`, complete 2026-08-04)** — the pure `IdentityProbe`
  comparator + candidate blocking (`mod.rs`), the periodic fetch→block→
  score→POST job against person/worker (`job.rs`), scale caps
  (`LINK_GRAPH_SUGGEST_MAX_CANDIDATES`/`_MAX_EDGES_PER_RUN`), and the
  `suggestion_runs` durable per-pass audit (`src/models/suggestion_runs.rs`).
  Review/promotion lives entirely in **person-service**'s
  `review_queue` (T-32) — see that crate's own `spec/13-tasks.md`.
  Live-verified never to auto-promote regardless of score
  (`tests/live_suggest_never_promoted.rs`). See §5.5, §6.8, §10.7,
  `spec/16-open-questions.md` OQ-9.
- **Tests** — un-gated unit suite (§11.1, 104 tests as of 2026-08-29) +
  `#[ignore]`d
  DB-gated suites (§11.2: `concealment`, `governance`, `graph_endpoints`,
  `idempotency`, `lazy_verify`, `reconcile`, `suggestion_runs`) + two
  manual `#[ignore]`d live-service tests
  (`tests/live_suggest_fetch.rs`, `tests/live_suggest_full_pipeline.rs`,
  `tests/live_suggest_never_promoted.rs` — not part of any CI stage,
  driven by hand against real running person/worker services; §13 T-33
  flags giving these their own excluded tier as a follow-up, since the
  blanket `cargo test -- --ignored` `test-db` CI stage currently sweeps
  them up despite their own doc comments saying otherwise).

### 14.2 Deferred (unchecked in §13)

Check [§13](13-tasks.md) directly for the current, authoritative list —
it is kept current task-by-task; this is only a snapshot. As of
2026-08-29: graph-read privacy-masking parity with the case service
(T-18), the durable-bus flip per entity (T-23), and the
bus/governance/bench test tiers (T-26 Fluvio round-trip, T-27
governance no-leak, T-28 Criterion benchmarks). T-22 is fully done —
its OTLP half landed 2026-08-05, and its Podman `HEALTHCHECK` +
non-root `USER` directives were already in the `Dockerfile` (added
2026-08-03); a prior version of this note and of §13 T-22's own
residual text claimed the container hardening half was still open,
which reading the `Dockerfile` does not bear out (corrected
2026-08-29, PRO-P25).

### 14.3 Upstream prerequisites — status as of 2026-08-04 (both resolved)

**Note: this section originally listed two pre-implementation
blockers, both since resolved; kept for history rather than deleted,
since a future entity's onboarding follows the same shape.**

- The **durable event bus** ([event-bus.md](../../../agents/share/event-bus.md))
  is real, not just a design doc: every entity service carries a
  transactional outbox (default-off `<ENTITY>_EVENT_TRANSPORT=memory`),
  **case** is the first live Fluvio producer (BUS-1), and this
  service's own consumer (T-6, BUS-2, `src/consumer.rs`) is a real
  Fluvio subscriber behind its `fluvio` feature. Most entities still
  default to the in-memory transport family-wide, which is why the
  interim **lazy verify-on-read** path (§6 FR-11) remains live rather
  than retired (§13 T-6's own note on why turning it off globally would
  be premature).
- The **`linked` / `unlinked` events** + per-service **`entity_links`**
  write-side (design §4.1/§4.2) **have landed** in both **person** and
  **worker** (the `same_identity` backbone, design §11 step 2, LNK-2/
  LNK-3) — this aggregator consumes real traffic from both today, not
  only synthetic envelopes in the DB-gated test suite.

### 14.4 Family registration

Registered in the repo-root
[`AGENTS.md`](../../../AGENTS.md) and
[`agents/share/overview.md`](../../../agents/share/overview.md)
"Cross-cutting services" tables.
