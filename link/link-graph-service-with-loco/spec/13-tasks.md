## 13. Tasks

> This service is **spec-only**: no code exists yet. Every task below is
> unchecked. The order follows the design's rollout
> ([cross-service-linking.md §11](../../../agents/share/cross-service-linking.md#11-rollout)):
> contracts → backbone → aggregator reads → affiliations → hardening.

### Contracts & scaffold

- [ ] T-1: Scaffold the loco service skeleton (Cargo.toml, `src/`,
  `migrations/` + loco `migration/` bridge, Dockerfile, docker-compose,
  config). Read-only-to-world: no write controllers.
- [ ] T-2: Copy the `EntityRef` value type + `entity_type → service`
  map (design §3) into `src/ref/`. Pure data; `parse` / `Display`;
  unit-tested (§11.1).
- [ ] T-3: Copy the closed v1 `EdgeKind` registry (design §9) into
  `src/registry/` — endpoint-type pairs, direction, inverse,
  temporality, sensitivity. Unit-tested.
- [ ] T-4: SeaORM entity modules for `edges`, `entity_presence`,
  `consumer_offsets`, `processed_events`, `audit_log` (§10) + the
  migration SQL pairs.

### Bus consumption & projection

- [ ] T-5: Envelope decode for the `linked` / `unlinked` event `data`
  shape (design §4.2) + `created` / `deleted` / `merged`; `schema_version`
  switch (event-bus.md §4). DB-free tests.
- [ ] T-6: Per-topic bus consumers (`mxi.<entity>.events`), idempotent
  on `event_id` via `processed_events`; per-topic offset + freshness
  watermark to `consumer_offsets` (§6 FR-1/2/3).
- [ ] T-7: Graph projector — `linked` upsert / `unlinked` remove +
  symmetric canonicalisation for `same_identity` (§6 FR-4/5/6).
- [ ] T-8: Presence oracle — `created` / `deleted` → `entity_presence`;
  recompute incident-edge `status` (§6 FR-8/9/10).
- [ ] T-9: Merge repointing handler — `merged{pid, merged_from}` rewrites
  edges centrally, re-canonicalises, de-duplicates (§6 FR-12).

### Integrity (interim)

- [ ] T-10: Lazy verify-on-read client — one-shot `GET /{id}` to the
  source service via the `entity_type → service` map; cache verdict in
  `entity_presence`; supersede per-entity as topics go durable (§6
  FR-11, design §5.1).

### Read API

- [ ] T-11: `GET /api/v1/neighbors/{ref}` — both-direction index
  lookups; `kind` / `direction` / `depth` (capped) filters; `as_of`
  (§6 FR-13/17).
- [ ] T-12: `GET /api/v1/edges` — `from` / `to` / `kind` / `status`
  filters; `as_of` (§6 FR-14).
- [ ] T-13: `GET /api/v1/single-view/{ref}` — `same_identity`
  unification + affiliation walk (`person → worker → org`); `as_of`
  (§6 FR-15).
- [ ] T-14: `GET /api/v1/health/freshness` — per-topic lag (§6 FR-16).
- [ ] T-15: OpenAPI via utoipa derives + Swagger UI at `/swagger-ui`;
  raw spec at `/api-docs/openapi.json`.

### Governance (`case ↔ person`)

- [ ] T-16: Access control on every read path that could surface a
  `subject_of` / `about` edge (incl. `single-view`), with concealment of
  existence to unauthorised callers (§6 FR-18/20, §12).
- [ ] T-17: Audit every read/write touching governed edges to
  `audit_log` (§6 FR-19).
- [ ] T-18: Privacy masking parity with the case service on graph
  responses (§6 FR-20).

### Auth, hardening, observability

- [ ] T-19: Offline RS256 JWT verification via
  `authentication-verifier` (NFR-10), coordinated with the family-wide
  auth rollout
  ([jwt-enforcement.md](../../../agents/share/jwt-enforcement.md)).
- [ ] T-20: Reconciliation worker — diff read-model vs each service's
  authoritative `entity_links` (bulk-read or replay); emit divergence
  metric; repair (§6 FR-21, design §8).
- [ ] T-21: Prometheus `/metrics.prom` — consumer lag, edge counts by
  `status`, divergence, processed counters (§6 FR-22).
- [ ] T-22: Tracing + OpenTelemetry OTLP wiring; loco `/_health` /
  `/_ping`; graceful shutdown; Podman health check; non-root container.
- [ ] T-23: Flip transport to the durable bus per entity as Fluvio
  topics go live; retire lazy verify-on-read per entity (design §5.1,
  event-bus.md §8).

### Tests

- [ ] T-24: Un-gated unit suite (§11.1).
- [ ] T-25: DB-gated integration suite (§11.2), `#[ignore]`-tagged.
- [ ] T-26: Bus-gated round-trip + replay-rebuild suite (§11.3), feature
  `fluvio`.
- [ ] T-27: Governance tests — no-leak + audit (§11.4).
- [ ] T-28: Criterion benchmarks for `neighbors` / `edges` /
  `single-view` (§11.6).
