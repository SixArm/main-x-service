## 8. Architecture

This service is the **read side** of the hybrid topology fixed in
[`cross-service-linking.md` §4](../../../agents/share/cross-service-linking.md#4-topology-the-hybrid-model).
It owns no entity writes; its only state changes come from the bus.

```
 write path (in each entity service — NOT here)
 ┌── person-svc ──┐ ┌── worker-svc ─┐ ┌── org-svc ──┐ ┌── case-svc ──┐
 │ entity_links   │ │ entity_links  │ │ entity_links│ │ entity_links │
 │  (outbound)    │ │  (outbound)   │ │  (outbound) │ │  (outbound)  │
 │  + emit linked │ │  + emit linked│ │  + emit …   │ │  + emit …    │
 │  review_queue  │ │               │ │             │ │              │
 │  (T-32)        │ │               │ │             │ │              │
 └──────┬─────────┘ └──────┬────────┘ └─────┬───────┘ └──────┬───────┘
        │ (outbox → bus, event-bus.md)      │                │
        └──────────────┬────────────────────┴────────────────┘
                       ▼  mxi.<entity>.events  (created/deleted/merged/linked/unlinked)
       ┌── link-graph-service-with-loco (THIS SERVICE) ──────────────────┐
       │  bus consumers (one per topic, idempotent on event_id)          │
       │     ├─ linked/unlinked   → edges upsert/remove (FR-4/5/6)       │
       │     ├─ created/deleted   → entity_presence     (FR-8)           │
       │     ├─ merged            → repoint edges        (FR-12)         │
       │     └─ any               → advance freshness watermark          │
       │                                                                 │
       │  read-model: edges (bidirectional, indexed both ends)           │
       │              entity_presence (existence oracle)                 │
       │  integrity: status lifecycle from presence (FR-9/10)            │
       │             lazy verify-on-read (interim, FR-11)                │
       │  periodic workers (Tokio tasks, not loco `worker` jobs):         │
       │    reconciliation (FR-21) — GET peer entity_links, diff, repair │
       │    suggestion job (FR-23..28, LNK-4) — GET person+worker,       │
       │      block+score, POST matcher_suggested edges to person   ─┐  │
       │                                                              │  │
       │  read API (read-only to the world — no route of its own writes):│
       │    GET /neighbors /edges /single-view /health/freshness         │
       │    every response carries as_of                                 │
       └───────────────────────────────────────────────────────────┼───┘
                                                                     │
              ────────────────────────────────────────────────────┘
              (HTTP client, not a bus event: GET person+worker's
               list endpoints, POST person's link-write endpoint)
```

The suggestion job's `GET`/`POST` arrow and the reconciliation worker's
`GET` arrow (not drawn, same shape) both leave this service as an
**HTTP client**, not as a route it exposes — person's own `review_queue`
(T-32, shown above) is where a suggestion is decided, never here.
Neither is a violation of "read-only to the world" — that invariant is
about this service's own inbound surface (§1.3,
`spec/16-open-questions.md` OQ-9(c)).

### 8.1 Component layering

| Layer | Responsibility |
|---|---|
| **Bus consumers** | One subscription per `mxi.<entity>.events` topic; deserialize the [envelope](../../../agents/share/event-bus.md#4-event-envelope-canonical-versioned); dispatch by `kind`; dedupe on `event_id`; advance the per-topic freshness watermark. |
| **Graph projector** | Applies `linked` / `unlinked` to `edges`; canonicalises symmetric kinds; applies `merged` repointing. |
| **Presence oracle** | Applies `created` / `deleted` to `entity_presence`; recomputes affected edge `status`. |
| **Verifier (interim)** | Lazy verify-on-read: resolves unknown presence with a one-shot `GET /{id}` to the source service via the `entity_type → service` map; caches the verdict. |
| **Read API** | loco.rs controllers serving `/neighbors`, `/edges`, `/single-view`, `/health/freshness`; attaches `as_of`; enforces `case ↔ person` governance (§12). |
| **Reconciliation worker** | Periodic Tokio task (`src/reconcile.rs`, `run_periodic`, spawned from `after_routes`, not a loco `worker` job); diffs read-model vs each service's authoritative `entity_links` via an authenticated `HttpAuthoritativeSource`; emits divergence; repairs. |
| **Suggestion job (LNK-4)** | Periodic Tokio task (`src/suggest/job.rs`, `run_periodic`, spawned from `after_routes`); fetches person + worker via `HttpIdentitySource`, blocks + scores via `src/suggest/mod.rs`, `POST`s survivors to person via `HttpSuggestionSink`; writes one `suggestion_runs` row per completed pass (§6.8, §10.7). |
| **Observability** | Prometheus metrics (`src/metrics.rs`: lag, divergence, status counts, suggestion-run gauge) **and real OpenTelemetry OTLP export** (`src/observability.rs`, T-22 — the family's first): a `tracing_opentelemetry` bridge over an OTLP/gRPC `SdkTracerProvider` + `SdkMeterProvider`, installed through loco's `Hooks::init_logger` alongside loco's own fmt layer and `EnvFilter`, flushed from `on_shutdown`; `trace_mw` opens one span per request and returns its W3C `traceparent`. On by default at `OTLP_ENDPOINT` (`http://localhost:4317`); `OTLP_ENDPOINT=""` disables. |

### 8.2 Integrity lifecycle (state machine)

```
linked event → projector inserts edge
                     │
   presence of both endpoints?
     ├─ both alive ............................. verified
     ├─ an endpoint not yet observed ........... unverified
     └─ an endpoint observed deleted ........... dangling

created(ref) → presence[ref]=alive  → re-evaluate incident edges (may verify)
deleted(ref) → presence[ref]=dead   → re-evaluate incident edges (may dangle)
```

There is **no cross-service call on the write path** — writes happen in
the entity services. This service decides `status` purely from its own
presence oracle (event-driven), falling back to lazy verify-on-read only
while a topic is not yet durable
([design §5](../../../agents/share/cross-service-linking.md#5-integrity--optimistic--async-verify)).

### 8.3 Why aggregate (merge repointing)

A record merge in any service emits `merged{pid, merged_from}`. Because
the graph is aggregated **here**, repointing every affected edge is one
handler in one place (FR-12). With decentralised links this fix-up would
fan out to every service holding such an edge — the central reason the
read-model is aggregated
([design §5.3](../../../agents/share/cross-service-linking.md#53-merge-repointing-why-one-aggregator-helps)).

### 8.4 Transport selection (config-driven)

Mirrors the [event-bus config](../../../agents/share/event-bus.md#7-topics-partitioning-config):
`memory` (interim, in-memory + lazy verify-on-read) vs `outbox`/Fluvio
(durable, event-driven verification). Selection is env-driven (§9);
consumers don't know the transport.

### 8.5 Module structure (as implemented — supersedes the pre-scaffold plan)

The pre-scaffold plan this section previously described (`ref/`,
`registry/`, `consume/`, `projector/`, `presence/`, `verify/`,
`api/rest/`, `workers/`, `db/`, `observability/`) never matched what
was actually built; this is the real layout. Notably, `EntityRef` and
`EdgeKind` are **not** copied modules here: this crate **depends on**
the standalone [`entity-ref`](../../entity-ref-rust-crate) crate
(`use entity_ref::…`) for both — superseded 2026-07-06 per §13 T-2/T-3,
before the "copy per project" plan was ever built.

```
src/
├── lib.rs               crate root, family lint header
├── bin/main.rs           loco entry point
├── app.rs                loco `Hooks` — routes, workers, boot init;
│                         spawns the reconciliation + suggestion
│                         periodic tasks from after_routes
├── graph.rs              pure projection logic: canonical (symmetric
│                         ordering), edge_status (integrity lifecycle),
│                         single_view (golden-record walk), repoint
├── events.rs              apply_event / apply_event_idempotent — the
│                         bus-consumer seam; typed LinkedEvent/
│                         UnlinkedEvent envelope decode
├── envelope.rs            the shared event envelope shape
├── consumer.rs            real Fluvio bus consumer (feature `fluvio`,
│                         T-6/BUS-2) — one task per entity topic
├── probe.rs               lazy verify-on-read: PresenceProbe trait +
│                         HttpPresenceProbe (interim integrity, T-10)
├── reconcile.rs           reconciliation worker: diff, AuthoritativeSource
│                         trait, HttpAuthoritativeSource, run_periodic
├── suggest/
│   ├── mod.rs             LNK-4 comparator: IdentityProbe, ProbeName/
│   │                     ProbeIdentifier, compare_identity,
│   │                     generate_candidates[_bounded] — pure, offline
│   └── job.rs             LNK-4 periodic job: IdentitySource/
│                         SuggestionSink traits, Http* implementations,
│                         run_suggestion_pass, run_periodic
├── auth.rs                offline PASETO v4.public verification (via
│                         authentication-verifier) + ABAC policy +
│                         case↔person governance (is_governed,
│                         may_see_governed, conceal_governed)
├── compliance/
│   ├── mod.rs
│   ├── mac.rs             integrity MAC (via integrity-mac)
│   └── audit_integrity.rs
├── controllers/
│   ├── mod.rs
│   ├── graph.rs            GET /neighbors /edges /single-view /health/freshness
│   ├── docs.rs             OpenAPI JSON + Swagger UI
│   ├── metrics.rs          GET /metrics.prom
│   └── compliance.rs
├── models/
│   ├── mod.rs
│   ├── edges.rs             the edges read-model + repository helpers
│   ├── entity_presence.rs
│   ├── consumer_offsets.rs
│   ├── processed_events.rs
│   ├── audit_log.rs
│   ├── suggestion_runs.rs   LNK-4 T-33 durable per-pass audit (§10.7)
│   └── _entities/           SeaORM-generated entities (one per table)
├── openapi.rs               hand-written OpenAPI 3.0.3 doc
└── metrics.rs                Prometheus registry (process-wide)
```

Migrations live in the crate-root `migration/` directory (a
`sea-orm-migration` migrator over hand-written SQL pairs, §10), not
under `src/`. There is no `db/` module — SeaORM entities live under
`src/models/_entities/`, hand-written models beside them under
`src/models/`, matching the family's loco-idiomatic layout
([architecture.md](../../../agents/share/architecture.md)) rather than
the person-style `src/db/` layout.
