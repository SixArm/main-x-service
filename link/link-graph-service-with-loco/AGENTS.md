# Agent guide — link-graph-service-with-loco

The **read-model aggregator** for cross-service entity linking — the
read side of the hybrid topology. A loco.rs backend service that
consumes every entity's event stream and serves one queryable graph.
**Read-only to the world**; all its writes are event-driven.

## Design docs are the source of truth (above this crate)

This service **realises** two shared design docs. They sit above this
crate's own `spec.md`:

- [`../../agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md)
  — this service is its **§4.3 read-model aggregator**. Fixes
  `EntityRef` (§3), topology (§4), the integrity lifecycle (§5),
  freshness (§6), the **partition rule** (§7), reconciliation (§8), the
  **closed edge-kind registry** (§9), `case ↔ person` governance (§10),
  rollout (§11).
- [`../../agents/share/event-bus.md`](../../agents/share/event-bus.md)
  — the durable bus this service consumes; envelope (§4), delivery
  semantics (§6), the §9 consumer model.

If a contract changes upstream in those docs, follow it here via a §13
task. **Do not** diverge from the shared contracts in this spec alone.

## This crate's own spec

[`spec/index.md`](spec/index.md) (§1–§18) holds service-specific
decisions: domain model, read API, persistence, testing, compliance.
Live work queue in [`spec/13-tasks.md`](spec/13-tasks.md); open questions
in [`spec/16-open-questions.md`](spec/16-open-questions.md). There is no
`plan.md` and no `tasks.md`.

## Status

**Read API implemented** (see
[`spec/14-implementation-status.md`](spec/14-implementation-status.md)).
The crate exists and builds: `Cargo.toml`, `src/` (the four read
endpoints in `src/controllers/graph.rs` — `/api/neighbors/{ref}`,
`/api/edges`, `/api/single-view/{ref}`, `/api/health/freshness` — plus
the pure projection logic in `src/graph.rs`, the `apply_event` /
`apply_event_idempotent` seam in `src/events.rs`, the real Fluvio bus
consumer in `src/consumer.rs` (T-6, BUS-2, behind this crate's own
`fluvio` Cargo feature), lazy verify-on-read in `src/probe.rs`,
reconciliation in `src/reconcile.rs`, offline PASETO auth in
`src/auth.rs`, the cross-service `same_identity` suggestion comparator
+ periodic job in `src/suggest/` (`mod.rs` — pure `IdentityProbe`
comparison/blocking; `job.rs` — the `reqwest`-backed fetch → block →
score → POST pipeline against person/worker, LNK-4 T-29..33), OpenAPI/
Swagger, and Prometheus `/metrics.prom`), and the
`m20260709_000001_edges` … `_000004_audit_log`,
`m20260803_000001_processed_events`, `m20260804_000001_suggestion_runs`
migrations (the last backs `src/models/suggestion_runs.rs`, one durable
row per completed suggestion pass). The cross-service `same_identity`
matcher round (LNK-4, T-29..33) is **complete** — see
[`spec/13-tasks.md`](spec/13-tasks.md) and `spec/16-open-questions.md`
OQ-9. `cargo test --lib`: 104 passed (2026-08-29). Remaining (see
[`spec/13-tasks.md`](spec/13-tasks.md) and spec §14): graph-read
privacy-masking parity with the case service (T-18), the durable-bus
flip (T-23), the bus/governance/bench test tiers (T-26..28), and (this
pass) the documentation-harmonisation sweep LNK-4's completion
unblocked (DOC-6). T-22 itself is fully done — both its **OTLP export**
(landed 2026-08-05, `src/observability.rs`, the family's first working
exporter) and its Podman `HEALTHCHECK` + non-root `USER` directives
(present in the `Dockerfile` since it was added, 2026-08-03) — see the
correction in `spec/13-tasks.md` T-22.

## Three-part change rule

A behavioural change is one PR with three parts:

1. **Spec edit** — `spec/` (the relevant § + §13 task status).
2. **Code edit** — `src/`.
3. **Test edit** — the matching tier in
   [`spec/11-testing-strategy.md`](spec/11-testing-strategy.md).

A change that also touches a **shared contract** is **four parts**: edit
the shared design doc first, then the three above.

## Load-bearing invariants — do not erode

- **Read-only to the world.** Never add a link-write endpoint of this
  service's own. Edge creation / withdrawal lives in the owning entity
  service (`POST/DELETE /<plural>/{pid}/links`, design §4.1). This
  service only consumes the resulting `linked` / `unlinked` events —
  **from the outside looking in.** The invariant is about this crate's
  own *inbound* HTTP surface, not about whether it may ever call a
  peer's API: the `src/suggest/job.rs` periodic job (LNK-4) *does* make
  outbound `POST`s to person's `POST /api/persons/{id}/links` as an
  authenticated client, exactly as `src/reconcile.rs` already makes
  outbound `GET`s to read peers' authoritative edges. Neither adds a
  route here; both are this aggregator acting as a client of someone
  else's write/read API. Pinned in
  [`spec/16-open-questions.md`](spec/16-open-questions.md) OQ-9(c) —
  read it before assuming a new outbound call here is a violation.
- **Partition rule.** Cross-service links are **never** a matcher signal
  and are **never** stored in a within-entity `relationships` field
  (design §7). They live only here, in per-service `entity_links`, and
  in the events. An edit that violates this is rejected.
- **Closed edge-kind registry.** Adding a kind is a design §9 registry
  change first, then a copied row + endpoint-type pair + inverse here.
  Never an ad-hoc extension.
- **Idempotency.** Bus consumers dedupe on the envelope `event_id`
  (at-least-once delivery). The read-model must be rebuildable from a
  replay (it holds no un-derivable state).
- **`as_of` on every graph response.** Eventual consistency is made
  visible, not hidden.
- **`case ↔ person` governance.** `subject_of` / `about` edges carry the
  case service's posture: access control + audit + privacy masking on
  every read path that could surface them, including `single-view`.

## Tech-stack ground rules

- Family [Rust + Loco stack](../../agents/share/rust-loco-stack.md):
  Rust 2024, Tokio, Axum + Loco (backend-only — no Tera/HTMX/views),
  SeaORM + PostgreSQL, Fluvio for the bus, tracing + OpenTelemetry,
  MiMalloc on MUSL.
- **OpenAPI is hand-written, not Utoipa-generated.** `src/openapi.rs`
  builds the `OpenAPI` 3 document by hand with `serde_json::json!` —
  there is no `utoipa` dependency in this crate's `Cargo.toml` and no
  `#[derive(ToSchema)]`/`OpenApi` macro anywhere in `src/`. Deliberate
  (see the module doc comment): dependency-light and easier to keep
  accurate to the enveloped wire format than a derive macro would be.
- **Podman not Docker. PostgreSQL not SQLite. Tokio not async_std.**
- Prefer `with-chrono` for SeaORM time types on this new crate (OQ-5).
- **`EntityRef` is a real Cargo `path` dependency, not copied.**
  `Cargo.toml` carries `entity-ref = { path = "../entity-ref-rust-crate"
  }`; this crate takes the `EntityType`/`EntityRef`/`EdgeKind` types from
  [`link/entity-ref-rust-crate`](../entity-ref-rust-crate) rather than
  vendoring its own copy. This matches the shared design doc's resolved
  position (`cross-service-linking.md` §12: "RESOLVED by practice" —
  every real consumer, this aggregator included, took the path
  dependency rather than the copy-per-project plan §2/§3 originally
  described). Do not fork a local copy without explicit approval.

## What does NOT live here

- Link writes (in each entity service).
- Any matcher or matcher signal (partition rule).
- Within-entity `relationships` (on each domain model).
- A system of record — Postgres-per-service stays authoritative; this is
  a derived, rebuildable projection.

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`, `entity-ref`) live outside
`link/link-graph-service-with-loco/`:

```sh
podman build -f link/link-graph-service-with-loco/Dockerfile \
  -t link-graph-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /_health` returns `200`. See `.containerignore` at
the repository root (excludes every crate's `target/`, or the build
context would try to copy hundreds of GB of build artifacts). The wired
multi-service `examples/compose/` stacks (DEP-1) that build on this are
not yet written.
