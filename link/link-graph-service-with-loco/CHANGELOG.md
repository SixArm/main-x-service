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
  the aggregator's `LinkedEvent` (the cross-service seam). Other services
  follow as they gain a bulk-links endpoint.

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

## [Unreleased]

- Build-out is enumerated as unchecked tasks in
  [`spec/13-tasks.md`](./spec/13-tasks.md) (T-1 … T-28), ordered after
  the design rollout: contracts → `same_identity` backbone → reads →
  affiliations + `case ↔ person` governance → hardening / durable-bus
  flip. No code yet (see
  [`spec/14-implementation-status.md`](./spec/14-implementation-status.md)).

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
