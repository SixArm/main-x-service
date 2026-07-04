# Durable event bus — design

How the Main X Index family moves from the current **in-memory** event
streams to a **durable, replayable** event bus, with [Fluvio](https://www.fluvio.io/)
as the transport (per [rust-loco-stack.md](rust-loco-stack.md)). This is
a design document: it fixes the envelope schema, the publisher seam, the
delivery semantics, and the rollout, so each crate can adopt it without
re-litigating the shape. It supersedes the "durable event bus" /
"durable broker" deferral notes in the service specs §13/§15.

## 1. Why change

Two implementations exist today, both **process-local and volatile**:

- **loco services** (organization, care-pathway, case) — a `OnceLock`
  ring buffer in `src/streaming.rs` with free functions `publish(kind,
  pid, name)` / `recent(limit)` and a flat `…Event { kind, pid, name, seq }`
  (cap 1000). `seq` is per-process.
- **Legacy Axum services** (person, worker, place) — an `EventProducer` /
  `EventConsumer` trait pair in `src/streaming/`, an internally-tagged
  enum event carrying the full record, and `InMemoryEventPublisher`.

Shared limitations:

- **Not durable** — events vanish on restart; a crash between the DB
  commit and the in-memory push silently loses the event.
- **Single-process** — no horizontal scaling; replicas each hold a
  different partial buffer, so `/events/recent` is per-replica.
- **No replay** — a new consumer (search re-indexer, analytics, a peer
  service) cannot read history; there are no offsets.
- **No cross-service consumption** — the index family is meant to fan
  out (e.g. a place change invalidates a worker's cached address); the
  in-memory buffer can't leave the process.

## 2. Goals & non-goals

**Goals**

- Durable, ordered-per-record, replayable event delivery.
- **No lost events**: an event is published iff its DB mutation
  committed (exactly-once *production* relative to the DB; at-least-once
  *delivery* to consumers).
- One uniform envelope across all entities; self-describing JSON.
- Pluggable transport behind a trait — in-memory stays the default for
  tests and single-node dev; Fluvio is the production target.
- `GET /<plural>/events/recent` keeps working unchanged for operators.

**Non-goals**

- Event sourcing — Postgres remains the system of record; events are a
  derived, append-only change feed, not the primary store.
- Exactly-once *delivery* — consumers must be idempotent (see §6).
- Replacing the audit log — `audit_logs` stays the compliance record
  (§12); the bus is the operational change feed. They are written from
  the same transaction (§5).

## 3. The transactional outbox (core of the design)

The crash window — "DB committed, broker publish not yet sent" — is
closed with the **transactional outbox** pattern, which fits loco's
Postgres-backed workers ([loco.md](loco.md)) exactly:

```
 ┌── request handler ──────────────────────────────┐
 │  BEGIN                                           │
 │    INSERT/UPDATE the entity row                  │
 │    INSERT one row into event_outbox  (same tx)   │
 │  COMMIT                                          │
 └─────────────────────────────────────────────────┘
                     │  (committed atomically)
                     ▼
 ┌── relay worker (loco Postgres-backed worker) ────┐
 │  poll event_outbox WHERE published_at IS NULL    │
 │     ORDER BY id  FOR UPDATE SKIP LOCKED          │
 │  publish each to Fluvio (topic+partition by pid) │
 │  UPDATE event_outbox SET published_at = now()    │
 └─────────────────────────────────────────────────┘
```

The entity write and the outbox write share one transaction, so they
commit or roll back together — no event without a committed change, no
committed change without an event. The relay then does at-least-once
delivery to Fluvio; a crash mid-publish re-publishes on restart (the row
is still unmarked), which is why consumers must dedupe on `event_id`.

This also subsumes the current best-effort audit write: `audit_logs` and
`event_outbox` are written in the same handler transaction, so they can
never disagree.

### `event_outbox` table

```sql
CREATE TABLE event_outbox (
    id            BIGSERIAL PRIMARY KEY,      -- global monotonic order
    event_id      UUID NOT NULL UNIQUE,       -- envelope id (dedup key)
    entity        TEXT NOT NULL,              -- "organization" | "care_pathway" | …
    entity_pid    UUID NOT NULL,              -- partition key (per-record order)
    kind          TEXT NOT NULL,              -- created|updated|deleted|merged
    occurred_at   TIMESTAMPTZ NOT NULL,
    actor         TEXT,                       -- user pid from the bearer token, if any
    schema_version INT NOT NULL DEFAULT 1,
    payload       JSONB NOT NULL,             -- the envelope (§4)
    published_at  TIMESTAMPTZ                 -- NULL until the relay ships it
);
CREATE INDEX event_outbox_unpublished
    ON event_outbox (id) WHERE published_at IS NULL;
```

Retention: a periodic worker deletes `published_at < now() - INTERVAL
'<retention>'` (e.g. 7 days). Durability of *history* is Fluvio's job
(topic retention); the outbox is a short-lived hand-off buffer.

## 4. Event envelope (canonical, versioned)

One shape for every entity, every transport. JSON, self-describing.

```jsonc
{
  "event_id":   "9f1c…",            // UUID v4; dedup key for consumers
  "schema_version": 1,
  "entity":     "care_pathway",      // snake_case entity name
  "kind":       "updated",           // created | updated | deleted | merged
  "pid":        "3b2a…",             // the record's public UUID
  "seq":        42,                  // per-entity_pid monotonic (from id ordering)
  "occurred_at":"2026-06-13T10:01:02Z",
  "actor":      "user-pid-or-null",  // who caused it (bearer sub), if known
  "name":       "Acute Stroke Pathway", // denormalised label for operator views
  "data":       { /* full record snapshot, or {pid} for deletes */ },
  "merged_from":"old-pid-or-absent"  // only on kind=merged
}
```

Rules:

- **`event_id`** is the idempotency key end-to-end.
- **`pid`** is the Fluvio **partition key** → all events for one record
  land on one partition → per-record total order. Cross-record order is
  not guaranteed (and not needed).
- **`data`** carries the post-change snapshot so consumers need no
  follow-up fetch (matches today's legacy `PersonEvent`). `deleted` carries
  only `{pid}`. Large payloads MAY be truncated to a reference — decide
  per entity; default is full snapshot.
- **`schema_version`** is bumped on any breaking envelope change;
  consumers switch on it. Additive fields don't bump it.
- The flat `{kind, pid, name, seq}` the loco `/events/recent` endpoint
  returns today is a **projection** of this envelope, so the operator API
  is unchanged.

A shared crate (`mxi-events`, dependency-light) SHOULD own the `Envelope`
+ `EventKind` types and the `topic_for(entity)` / `partition_key(pid)`
helpers, so producers and Rust consumers share one definition. (Until it
exists, copy the struct per crate — drift is cheap and the schema is
small; same posture as the front-end drift decision.)

## 5. Publisher seam

Unify both existing shapes behind one trait:

```rust
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Durably enqueue an event. In the outbox design this is the
    /// in-transaction INSERT into event_outbox; the relay does the
    /// actual Fluvio send. Never silently drops.
    async fn publish(&self, env: &Envelope, tx: &mut DbTx) -> Result<()>;

    /// Recent events for the operator endpoint (projection of §4).
    async fn recent(&self, limit: usize) -> Vec<EventView>;
}
```

Implementations:

- **`InMemoryPublisher`** — today's ring buffer; `publish` ignores `tx`
  and pushes to the `VecDeque`. Default for tests + single-node dev. Keeps
  `cargo test` DB-free.
- **`OutboxPublisher`** — `publish` inserts into `event_outbox` on the
  handler's transaction; `recent` reads the last N outbox rows. This is
  the durable default in deployment.
- **Relay** — a loco Postgres-backed worker (`queue.kind: Postgres`,
  [loco.md](loco.md)) that drains `event_outbox` to a **`FluvioSink`**
  (feature `fluvio`). The sink, not the request path, holds the Fluvio
  client, so request latency is unaffected and a Fluvio outage only backs
  up the outbox (it never fails a write).

Selection is config-driven (§7); handlers call `publish(&env, tx)` and
don't know the transport.

## 6. Delivery semantics

- **Production**: exactly-once relative to the DB (outbox + same tx).
- **Delivery**: at-least-once. The relay may re-send after a crash
  between Fluvio-ack and the `published_at` update.
- **Consumers MUST be idempotent**, keyed on `event_id` (e.g. a
  `processed_events(event_id)` table, or an upsert by `pid`+`seq`).
- **Ordering**: total per `pid` (single partition); none across `pid`.
  Consumers that need global order sort by `occurred_at` within a window
  and tolerate skew.
- **Offsets / replay**: Fluvio consumers track their own offset; a new or
  rebuilding consumer replays a topic from offset 0 (or a timestamp).
  This is how a fresh Tantivy index or analytics store back-fills.

## 7. Topics, partitioning, config

- **Topic per entity**: `mxi.<entity>.events` (e.g.
  `mxi.organization.events`). One topic per entity keeps consumer
  subscriptions and retention independent.
- **Partition key** = `pid`. Partition count is an ops choice (start 3–6);
  per-record order holds regardless.
- **Config** (per service, env-driven, mirroring the auth/`REQUIRE_AUTH`
  pattern):

| Var | Meaning | Default |
|---|---|---|
| `<ENTITY>_EVENT_TRANSPORT` | `memory` \| `outbox` | `memory` |
| `<ENTITY>_FLUVIO_ENDPOINT` | Fluvio SC address | — |
| `<ENTITY>_EVENT_TOPIC` | topic override | `mxi.<entity>.events` |
| `<ENTITY>_EVENT_RETENTION_DAYS` | outbox row TTL | `7` |

`memory` ⇒ exactly today's behaviour (default keeps tests + dev green).
`outbox` ⇒ durable; the relay worker + `fluvio` feature must be built in.

## 8. Rollout

1. **Land the envelope + trait seam**, with `InMemoryPublisher` wired as
   today. Pure refactor; behaviour identical; tests stay DB-free. (The
   loco free functions become a thin `InMemoryPublisher`; the legacy
   `EventProducer` is renamed/adapted to `EventPublisher`.)
2. **Add `event_outbox`** migration + `OutboxPublisher`; switch handlers
   to write the outbox row on their existing transaction. DB-gated tests
   assert the row is written with the entity change and rolled back with
   it.
3. **Add the relay worker + `FluvioSink`** behind feature `fluvio`. A
   DB-gated integration test (or a Fluvio test container) asserts an
   enqueued row reaches the topic and is marked published.
4. **Flip `<ENTITY>_EVENT_TRANSPORT=outbox`** per service in deployment;
   stand up consumers (search re-indexer first).
5. Adopt per entity in spec-priority order; the in-memory default means
   un-migrated crates keep working throughout.

## 9. Consumers (initial set)

- **Search re-indexer** — keeps Tantivy in sync with DB writes via the
  stream instead of inline indexing; replayable for full rebuilds.
- **Cross-entity cache invalidation** — e.g. a place address change
  notifies workers referencing it.
- **Analytics / audit aggregation** — a durable sink for the change feed
  (complements, doesn't replace, `audit_logs`).

Each is a standalone Fluvio consumer (Rust, sharing `mxi-events`),
idempotent on `event_id`, tracking its own offset.

## 10. Testing strategy

- **Un-gated**: envelope (de)serialization + `schema_version`;
  `topic_for` / `partition_key`; `InMemoryPublisher` publish/recent (the
  existing ring-buffer tests, retargeted); the projection from `Envelope`
  to the operator `EventView`.
- **DB-gated** (`#[ignore]`): outbox row written in the same tx as the
  entity change; rolled back together on handler error; relay marks
  `published_at`; `recent` reads from the outbox.
- **Fluvio-gated** (feature `fluvio` + a broker/test container): relay →
  topic → consumer round-trip; partition-key ordering per `pid`;
  at-least-once redelivery dedup on `event_id`.

## 11. Open questions

- Shared `mxi-events` crate now, or copy-per-crate until a second
  consumer exists? (Lean: extract when the first real consumer ships.)
- Full snapshot in `data` vs reference-only for large records (case,
  person) — per-entity decision; default full.
- Schema-registry / Avro vs JSON envelope — JSON for v1 (matches today's
  self-describing Serde wire form); revisit if payloads grow.
- One relay worker per service vs a shared relay reading all outboxes —
  per-service is simpler and matches the deploy unit; start there.
