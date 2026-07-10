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
- [ ] T-6: Per-topic bus consumers (`mxi.<entity>.events`), idempotent
  on `event_id` via `processed_events`; per-topic offset + freshness
  watermark to `consumer_offsets` (§6 FR-1/2/3). **v1 provides the
  `apply_event` seam + per-topic offset/freshness upsert; the Fluvio
  consumer loop and `processed_events` idempotency are deferred.**
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
- [ ] T-23: Flip transport to the durable bus per entity as Fluvio
  topics go live; retire lazy verify-on-read per entity (design §5.1,
  event-bus.md §8).

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
