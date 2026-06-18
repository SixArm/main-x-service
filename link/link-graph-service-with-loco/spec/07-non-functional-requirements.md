## 7. Non-Functional Requirements

### 7.1 Consistency & freshness

- **NFR-1** — The read-model is **eventually consistent**; it trails
  the writes by the bus delivery lag. This is made **visible**, not
  hidden: every graph response carries `as_of`, and `/health/freshness`
  reports per-topic lag.
- **NFR-2** — **Freshness SLO.** Under the durable bus, steady-state
  consumer lag SHOULD be under a small bound (target: seconds);
  sustained breach is an alert. Under the interim in-memory bus, lazy
  verify-on-read trades first-read latency for correctness.
- **NFR-3** — **Divergence SLO.** Reconciliation divergence (§6 FR-21)
  SHOULD be ≈ 0 in steady state; a rising count is the signal that the
  relay or a consumer is unhealthy.

### 7.2 Correctness & integrity

- **NFR-4** — **Rebuildability.** The entire read-model MUST be
  reconstructable from a replay of the entity topics from offset 0
  (durable bus) — it holds no un-derivable state.
- **NFR-5** — **Idempotency.** Event processing is idempotent on
  `event_id`; replays and at-least-once redelivery converge to the same
  graph.
- **NFR-6** — No write-time foreign keys across services; integrity is
  the `status` lifecycle (§8). A `dangling` edge is a surfaced,
  queryable state — never a silent break.

### 7.3 Performance

- **NFR-7** — `GET /neighbors` (depth 1) and `GET /edges` are
  single-index lookups (`edges_from` / `edges_to`); target p95 under a
  small bound on a representative graph.
- **NFR-8** — Multi-hop (`single-view`, depth > 1) is a depth-capped
  recursive CTE; the depth cap (§16) bounds worst-case fan-out.

### 7.4 Security & privacy

- **NFR-9** — `case ↔ person` edges carry the case service's
  compliance posture (access control + audit + masking, §12); the
  default-open affiliation posture does **not** apply to them.
- **NFR-10** — Offline RS256 JWT verification via the
  [authentication-verifier](../../../authentication/authentication-verifier-rust-crate/)
  (JWKS-based, no introspection hop), consistent with the sibling
  services.

### 7.5 Availability & operability

- **NFR-11** — Stateless request path (graph reads hit Postgres);
  horizontally scalable behind a load balancer. The consumer / relay /
  reconciliation workers are the only stateful-position components
  (offsets), tracked durably.
- **NFR-12** — Health endpoints for orchestration (loco `/_health`,
  `/_ping`); graceful shutdown; Podman health checks; non-root
  container.
- **NFR-13** — Database connection pooling with configurable limits.

### 7.6 Stack conformance

- **NFR-14** — Follows the family
  [Rust + Loco stack](../../../agents/share/rust-loco-stack.md): Rust
  2024 edition, Tokio, Axum + Loco (backend-only), SeaORM + PostgreSQL,
  Tantivy where search is needed (not required for v1 graph reads),
  Fluvio for the bus, Utoipa OpenAPI, tracing + OpenTelemetry,
  MiMalloc on MUSL. Podman not Docker; PostgreSQL not SQLite.
