## 14. Implementation Status

**v1 core implemented (2026-07-09).** The read-model core compiles
clean, is clippy-clean (`--all-targets --all-features`), and is
unit-tested (`cargo test --lib`, 15 tests). Built from the canonical
design doc
[`cross-service-linking.md`](../../../agents/share/cross-service-linking.md)
(this is its §4.3 read-model aggregator) and the
[`event-bus.md`](../../../agents/share/event-bus.md) §9 consumer model.

### 14.1 What is implemented

- **Scaffold** — loco.rs crate (`Cargo.toml`, `src/lib.rs` +
  `src/bin/main.rs` with the family lint header, `src/app.rs` `Hooks`,
  `config/{development,test,production}.yaml`, the `migration/` bridge
  crate). Read-only to the world; no write controllers. Depends on the
  sibling **[`entity-ref`](../../entity-ref-rust-crate)** crate for the
  shared `EntityType` / `EntityRef` / `EdgeKind` / `Sensitivity`
  contracts (the T-2/T-3 "copy" is superseded — see §13).
- **Persistence** — three derived-read-model tables with hand-written
  SQL bridged via `include_str!`: `edges` (+ `edges_from` / `edges_to` /
  `edges_status` indexes), `entity_presence`, `consumer_offsets` (§10.1
  /§10.2/§10.3). SeaORM entities under `src/models/_entities/`.
  `with-chrono` time types (§10.5, OQ-5).
- **Pure projection logic** (`src/graph.rs`, DB-free, fully unit-tested)
  — `canonical` (symmetric ordering, FR-6), `edge_status`
  (unverified/verified/dangling lifecycle, FR-9), `single_view`
  (same-identity unification + affiliation walk, FR-15), plus
  `EdgeStatus` / `Provenance` value types.
- **Apply seam** (`src/events.rs`) — `apply_event(db, envelope)`
  dispatches `created`/`deleted` → presence (+ incident-edge status
  recompute, FR-10), `linked` → canonical edge upsert, `unlinked` →
  edge removal, `merged` → acknowledged (repointing deferred). Typed
  `LinkedEvent` / `UnlinkedEvent` decode (closed-registry `edge_kind`
  validation) with DB-free tests. Every consumed event advances the
  per-topic freshness watermark.
- **Read API** (`src/controllers/graph.rs`, enveloped
  `{success,data,error}` + `as_of`) — `GET /api/neighbors/{ref}`
  (bounded BFS, `kind`/`direction`/`depth≤2`), `GET /api/edges`
  (from/to/kind/status), `GET /api/single-view/{ref}`,
  `GET /api/health/freshness`. `400` on malformed ref / unknown kind /
  over-cap depth. Loco `/_health` + `/_ping` retained.
- **Tests** — un-gated unit suite (§11.1) + `#[ignore]`d DB-gated
  request suite (`tests/graph_endpoints.rs`, §11.2).

### 14.2 Deferred (unchecked in §13)

The Fluvio bus consumer loop + `processed_events` idempotency (T-6),
merge-repointing (T-9), lazy verify-on-read (T-10), OpenAPI/Swagger
(T-15), `case ↔ person` governance / audit / masking (T-16..18),
offline PASETO auth (T-19), the reconciliation worker (T-20),
Prometheus `/metrics.prom` + OTLP (T-21/22), the durable-bus flip
(T-23), and the bus/governance/bench test tiers (T-26..28). The
`apply_event` seam is the integration point a future bus consumer will
call.

### 14.3 Upstream prerequisites (not in this crate)

- The **durable event bus** ([event-bus.md](../../../agents/share/event-bus.md))
  is still a design doc; today's transport is in-memory and volatile,
  hence the interim **lazy verify-on-read** path (§6 FR-11) remains
  deferred.
- The **`linked` / `unlinked` events** + per-service **`entity_links`**
  write-side (design §4.1/§4.2) must land in **person** + **worker**
  first (the `same_identity` backbone, design §11 step 2) before this
  aggregator consumes real traffic. The `apply_event` seam and the
  DB-gated tests already exercise the projection against synthetic
  envelopes.

### 14.4 Family registration

Registered in the repo-root
[`AGENTS.md`](../../../AGENTS.md) and
[`agents/share/overview.md`](../../../agents/share/overview.md)
"Cross-cutting services" tables.
