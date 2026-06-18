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
 └──────┬─────────┘ └──────┬────────┘ └─────┬───────┘ └──────┬───────┘
        │ (outbox → bus, event-bus.md)      │                │
        └──────────────┬────────────────────┴────────────────┘
                       ▼  mxi.<entity>.events  (created/deleted/merged/linked/unlinked)
       ┌── link-graph-service-with-loco (THIS SERVICE) ──────────────┐
       │  bus consumers (one per topic, idempotent on event_id)      │
       │     ├─ linked/unlinked   → edges upsert/remove (FR-4/5/6)   │
       │     ├─ created/deleted   → entity_presence     (FR-8)       │
       │     ├─ merged            → repoint edges        (FR-12)     │
       │     └─ any               → advance freshness watermark      │
       │                                                             │
       │  read-model: edges (bidirectional, indexed both ends)       │
       │              entity_presence (existence oracle)             │
       │  integrity: status lifecycle from presence (FR-9/10)        │
       │             lazy verify-on-read (interim, FR-11)            │
       │  workers: reconciliation (FR-21) · retention                │
       │                                                             │
       │  read API (read-only to the world):                         │
       │    GET /neighbors /edges /single-view /health/freshness     │
       │    every response carries as_of                             │
       └─────────────────────────────────────────────────────────────┘
```

### 8.1 Component layering

| Layer | Responsibility |
|---|---|
| **Bus consumers** | One subscription per `mxi.<entity>.events` topic; deserialize the [envelope](../../../agents/share/event-bus.md#4-event-envelope-canonical-versioned); dispatch by `kind`; dedupe on `event_id`; advance the per-topic freshness watermark. |
| **Graph projector** | Applies `linked` / `unlinked` to `edges`; canonicalises symmetric kinds; applies `merged` repointing. |
| **Presence oracle** | Applies `created` / `deleted` to `entity_presence`; recomputes affected edge `status`. |
| **Verifier (interim)** | Lazy verify-on-read: resolves unknown presence with a one-shot `GET /{id}` to the source service via the `entity_type → service` map; caches the verdict. |
| **Read API** | loco.rs controllers serving `/neighbors`, `/edges`, `/single-view`, `/health/freshness`; attaches `as_of`; enforces `case ↔ person` governance (§12). |
| **Reconciliation worker** | Periodic loco Postgres-backed worker; diffs read-model vs each service's authoritative `entity_links`; emits divergence; repairs. |
| **Observability** | Prometheus metrics (lag, divergence, status counts), tracing, OTLP. |

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

### 8.5 Module structure (planned)

```
src/
├── ref/            # EntityRef value type + entity_type→service map (copied contract)
├── registry/       # closed EdgeKind registry (copied contract)
├── consume/        # bus consumers (per topic), envelope decode, dedupe
├── projector/      # edges upsert/remove, symmetric canonicalisation, merge repoint
├── presence/       # entity_presence oracle + status recompute
├── verify/         # interim lazy verify-on-read client
├── api/rest/        # neighbors / edges / single-view / freshness controllers
├── workers/        # reconciliation worker, outbox retention
├── db/             # SeaORM entities, repositories, migrations bridge
├── observability/  # metrics, tracing, OTLP
└── lib.rs
```
