# Event streaming

Monorepo-wide spec for the **event stream** — the append-only change
feed that every **Main X Index** service emits for its CRUD and merge
operations. This is the umbrella view across the family; the full
durable-bus design lives in
[`agents/share/event-bus.md`](../../agents/share/event-bus.md), and each
service crate's own `spec/index.md` records that crate's adoption state.

This spec clearly separates **IMPLEMENTED** (what ships today: the
in-memory Phase 1 seam) from **DESIGNED / PLANNED** (the durable
transactional-outbox + Fluvio bus). The event stream is **complementary
to, not a replacement for, the audit log** — see
[`agents/share/auditability.md`](../../agents/share/auditability.md) and
§6 below.

## 1. Purpose

The event stream is an **append-only change feed**: every create,
update, (soft-)delete, and merge of a domain record emits one event.
It exists for **downstream consumers** that need to react to changes
rather than poll the database:

- a **search re-indexer** that keeps a Tantivy index in step with DB writes;
- **cross-entity cache invalidation** (e.g. a place address change
  invalidates a worker's cached address);
- **analytics / aggregation** sinks for the change feed.

It is deliberately **not** the system of record and **not** event
sourcing: PostgreSQL remains authoritative (see
[`spec/architecture/index.md`](../architecture/index.md)); the stream is
a *derived* feed. It is also **not** the compliance record — that is the
audit log (§6). The two are written from the same operation but serve
different consumers: the audit log answers *"who changed what, when"*
for compliance; the event stream answers *"what just changed"* for
operational reactors.

| | Audit log | Event stream |
|---|---|---|
| Purpose | Compliance / forensic trail | Operational change feed |
| Audience | Auditors, operators | Re-indexer, caches, analytics |
| Store | `audit_logs` table (durable) | In-memory today; Fluvio (planned) |
| Replayable | Query by time/record | Offsets + replay (planned) |
| Spec | [auditability.md](../../agents/share/auditability.md) | this document |

## 2. Current state — IMPLEMENTED (Phase 1)

Two shapes exist today, **both process-local and volatile**.

### 2.1 Loco services — canonical envelope + publisher seam

The loco services (organization, care-pathway, case) implement **Phase
1** of the durable-bus design: the canonical versioned envelope and the
publisher trait, wired to an in-memory ring buffer. Reference
implementation: `care-pathway-service-rust-crate/src/streaming.rs`.

**`Envelope`** — the canonical, versioned event shape (one shape per
entity and transport):

| Field | Type | Notes |
|---|---|---|
| `event_id` | `Uuid` | UUID v4; end-to-end idempotency / dedup key for consumers |
| `schema_version` | `u32` | currently `SCHEMA_VERSION = 1`; bumped only on breaking change |
| `entity` | `&'static str` | snake_case entity name, e.g. `"care_pathway"` |
| `kind` | `EventKind` | `Created` \| `Updated` \| `Deleted` \| `Merged` |
| `pid` | `String` | the record's public id (partition key in Phase 2) |
| `seq` | `u64` | monotonic sequence — **per process** in Phase 1 |
| `actor` | `Option<String>` | user pid from the bearer token, when known |
| `name` | `String` | denormalised label for operator views |

`EventKind` serializes lowercase (`created` / `updated` / `deleted` /
`merged`). Phase 1 deliberately **omits** `occurred_at` and `data`: the
design places them at the outbox stage (Phase 2), and threading a
timestamp/snapshot through the in-memory path is not worth it before
durability exists.

**`EventPublisher`** — the seam every transport implements:

```rust
pub trait EventPublisher: Send + Sync {
    fn publish(&self, env: Envelope);
    fn recent(&self, limit: usize) -> Vec<EventView>;
}
```

**`InMemoryPublisher`** — the only Phase 1 implementation: a process-wide
**ring buffer** (`OnceLock<InMemoryPublisher>` global; `Mutex<VecDeque>`,
capacity **1000** — publishing past the cap evicts the oldest). It never
fails: a poisoned lock drops the event (the audit log is the durable
record). Free functions `publish(kind, pid, name)` /
`publish_with_actor(...)` / `recent(limit)` wrap the global. `seq` comes
from a process-wide `AtomicU64` starting at 1.

**`EventView`** — the operator-facing **projection** of an `Envelope`,
exposing exactly `{ kind, pid, name, seq }` (and nothing else — `actor`,
`event_id`, `schema_version`, `entity` are dropped). Its JSON is
**frozen** and byte-identical to the pre-seam wire shape, so the
operator endpoint and front-end recent-activity views are unchanged. It
is served at:

```
GET /api/<plural>/events/recent   → [ { kind, pid, name, seq }, … ]
```

(e.g. `GET /api/care-pathways/events/recent`), newest last, capped at
`limit`.

Every CRUD/merge handler in these services publishes one envelope and
writes one `audit_logs` row.

### 2.2 Legacy Axum services — `EventProducer` / `EventConsumer`

The older Axum-era services (person, worker, place, thing, event)
predate the canonical envelope. Reference:
`person-service-rust-crate/src/streaming/mod.rs`. They use a
**producer/consumer trait pair** over a richer, record-bearing event
enum:

- **`PersonEvent`** — an internally-tagged enum (`#[serde(tag =
  "event_type")]`) with variants `Created` / `Updated` / `Deleted` /
  `Merged` / `Linked` / `Unlinked`. Data-bearing variants embed the
  **full record** (so consumers need no follow-up fetch); every variant
  carries a `chrono::DateTime<Utc>`.
- **`EventProducer`** trait — `publish(&self, event) -> Result<()>`.
- **`EventConsumer`** trait (stub) — `subscribe()` / `next_event()`.
- **`InMemoryEventPublisher`** — the default in-process producer; a
  Fluvio-backed transport is the stated production target.

The durable design (§4) **unifies both shapes** behind one
`EventPublisher` trait and one `Envelope`; the loco free functions
become a thin `InMemoryPublisher`, and the legacy `EventProducer` is
adapted to the same seam.

## 3. Limitations of the in-memory stream

Both Phase 1 shapes share the same volatility, which is exactly what the
durable design exists to remove:

| Limitation | Consequence |
|---|---|
| **Not durable** | events vanish on restart; a crash *between* the DB commit and the in-memory push silently loses the event |
| **Single-process** | replicas each hold a different partial buffer, so `/events/recent` is per-replica; no horizontal fan-out |
| **No replay** | a new consumer (re-indexer, analytics, a peer service) cannot read history — there are no offsets |
| **No cross-service consumption** | the buffer can't leave the process, so the index family can't fan out across entities |

`seq` being per-process is a symptom of the same constraint: it is
unique and monotonic only within one running process.

## 4. The durable design — PLANNED

A faithful summary of [`agents/share/event-bus.md`](../../agents/share/event-bus.md)
(the primary source — read it for the full schema, SQL, and rollout).
Transport target: **Fluvio**. This design supersedes the "durable event
bus" deferral notes in the service specs.

### 4.1 Transactional outbox (no lost events)

The crash window — *"DB committed, broker publish not yet sent"* — is
closed with the **transactional outbox** pattern, which fits loco's
Postgres-backed workers exactly:

```
 request handler:                          relay worker (loco bg job):
   BEGIN                                      poll event_outbox
     INSERT/UPDATE the entity row               WHERE published_at IS NULL
     INSERT one row into event_outbox           ORDER BY id FOR UPDATE SKIP LOCKED
       (SAME transaction)                      publish each → Fluvio
   COMMIT  (atomic)                            UPDATE … SET published_at = now()
```

The entity write and the outbox write **share one transaction**, so they
commit or roll back together — **no event without a committed change, no
committed change without an event**. This also subsumes the audit write:
`audit_logs` and `event_outbox` are written in the same handler
transaction, so they can never disagree.

The `event_outbox` table (`id BIGSERIAL` global order, `event_id UUID
UNIQUE` dedup key, `entity`, `entity_pid`, `kind`, `occurred_at`,
`actor`, `schema_version`, `payload JSONB`, `published_at`) is a
**short-lived hand-off buffer**: a periodic worker deletes rows whose
`published_at` is older than the retention window (default 7 days).
Durability of *history* is Fluvio's job (topic retention), not the
outbox's.

### 4.2 Relay worker → Fluvio

A **loco Postgres-backed worker** (`queue.kind: Postgres`, per
[`agents/share/loco.md`](../../agents/share/loco.md)) drains the outbox
to a `FluvioSink` (behind a `fluvio` cargo feature). The sink — not the
request path — holds the Fluvio client, so request latency is unaffected
and a Fluvio outage only **backs up the outbox**; it never fails a
write. See §6 for the relationship to the job queue.

### 4.3 Topics, partitioning, ordering, delivery

| Aspect | Rule |
|---|---|
| **Topic** | one per entity: `mxi.<entity>.events` (e.g. `mxi.organization.events`) — independent subscriptions and retention |
| **Partition key** | `pid` → all events for one record land on one partition |
| **Ordering** | **total per `pid`**; none across `pid` (and not needed) |
| **Production** | exactly-once relative to the DB (outbox + same tx) |
| **Delivery** | **at-least-once** — the relay may re-send after a crash between Fluvio-ack and the `published_at` update |
| **Idempotency** | consumers MUST dedupe on `event_id` (e.g. a `processed_events(event_id)` table, or upsert by `pid`+`seq`) |
| **Replay** | Fluvio consumers track their own offset; a new/rebuilding consumer replays a topic from offset 0 (or a timestamp) — this is how a fresh Tantivy index or analytics store back-fills |

### 4.4 Envelope at the outbox stage

The durable envelope adds the two Phase 1 omissions — `occurred_at` and
`data` (the post-change snapshot; `deleted` carries only `{pid}`, large
payloads MAY be truncated to a reference per entity, default full) — plus
`merged_from` on `kind = merged`. The flat `{kind, pid, name, seq}` that
`/events/recent` returns today stays a **projection** of this envelope,
so the operator API is unchanged across the rollout.

### 4.5 Publisher selection (config-driven)

Selection mirrors the JWT `REQUIRE_AUTH` pattern — per service,
env-driven, defaulting to today's behaviour:

| Var | Meaning | Default |
|---|---|---|
| `<ENTITY>_EVENT_TRANSPORT` | `memory` \| `outbox` | `memory` |
| `<ENTITY>_FLUVIO_ENDPOINT` | Fluvio SC address | — |
| `<ENTITY>_EVENT_TOPIC` | topic override | `mxi.<entity>.events` |
| `<ENTITY>_EVENT_RETENTION_DAYS` | outbox row TTL | `7` |

`memory` ⇒ exactly today's in-memory behaviour (keeps tests + dev
DB-free). `outbox` ⇒ durable; the relay worker + `fluvio` feature must
be built in. Handlers call `publish(&env, tx)` and never know the
transport.

### 4.6 Staged rollout

1. **Land the envelope + trait seam** with `InMemoryPublisher` wired as
   today — pure refactor, behaviour identical, tests stay DB-free.
   *(This is Phase 1, already done in the loco services.)*
2. **Add `event_outbox`** migration + `OutboxPublisher`; switch handlers
   to write the outbox row on their existing transaction.
3. **Add the relay worker + `FluvioSink`** behind feature `fluvio`.
4. **Flip `<ENTITY>_EVENT_TRANSPORT=outbox`** per service in deployment;
   stand up consumers (search re-indexer first).
5. Adopt per entity in spec-priority order; the `memory` default means
   un-migrated crates keep working throughout.

A shared, dependency-light `mxi-events` crate SHOULD eventually own the
`Envelope` + `EventKind` types and the `topic_for` / `partition_key`
helpers; until a second consumer ships, copy-per-crate is accepted
(open question in the design doc).

## 5. Consumers — PLANNED

Each is a standalone Fluvio consumer (Rust, sharing `mxi-events`),
**idempotent on `event_id`**, tracking its own offset:

- **Search re-indexer** — keeps Tantivy in step with DB writes via the
  stream instead of inline indexing; replayable for full index rebuilds
  (see [`spec/search`](../search/index.md) for the search target).
- **Cross-entity cache invalidation** — e.g. a place address change
  notifies workers referencing it.
- **Analytics / audit aggregation** — a durable sink for the change feed
  (complements, does not replace, `audit_logs`).

## 6. Relationships

### 6.1 To background jobs

The relay worker is a **loco Postgres-backed background job**
(`queue.kind: Postgres`; per [`agents/share/loco.md`](../../agents/share/loco.md),
not SQLite-backed). It polls `event_outbox` with `FOR UPDATE
SKIP LOCKED`, which is the same Postgres concurrency primitive the job
queue itself uses. See
[`spec/postgresql/index.md`](../postgresql/index.md) §10 (Background
jobs) for the queue configuration and the SKIP-LOCKED pattern; the
outbox is effectively a second skip-locked queue drained by that worker.

### 6.2 To the audit log

The event stream and the audit log are **siblings, not substitutes**
(§1). In Phase 1 they are written in the same handler but independently
(the in-memory publish is best-effort; the audit row is the durable
record). In the durable design they become **transactionally
co-committed**: `audit_logs` and `event_outbox` are inserted in the same
transaction as the entity change, so the compliance trail and the
operational feed can never disagree. The audit log stays the
authoritative compliance record; the stream stays the operational feed.
See [`agents/share/auditability.md`](../../agents/share/auditability.md).

### 6.3 To merge

A merge emits a single `Merged` (loco) / `Merged { source, target }`
(legacy) event, alongside the merge-history record. The durable envelope
adds `merged_from` on `kind = merged`. See
[`spec/merge/index.md`](../merge/index.md).

## 7. Testing

| Tier | Gate | What it pins |
|---|---|---|
| Un-gated | none | `Envelope` (de)serialization + `schema_version`; `topic_for` / `partition_key`; `InMemoryPublisher` publish/recent (ring-buffer tests); the `Envelope → EventView` projection (frozen keys `kind`,`pid`,`name`,`seq`) |
| DB-gated | `#[ignore]` / Postgres | outbox row written in the same tx as the entity change; rolled back together on handler error; relay marks `published_at`; `recent` reads from the outbox |
| Fluvio-gated | feature `fluvio` + broker/test container | relay → topic → consumer round-trip; per-`pid` ordering; at-least-once redelivery dedup on `event_id` |

The Phase 1 in-memory tests (publish/read-back, monotonic `seq`, Serde
round-trip, frozen `EventView` keys) live in each loco service's
`src/streaming.rs` and are entirely DB-free, keeping `cargo test`
infrastructure-free.

## 8. Status summary

| Capability | State |
|---|---|
| Canonical versioned `Envelope` (loco services) | IMPLEMENTED (Phase 1) |
| `EventPublisher` seam + `InMemoryPublisher` ring buffer (cap 1000) | IMPLEMENTED (Phase 1) |
| `EventView` projection at `/events/recent` | IMPLEMENTED (Phase 1) |
| Legacy `EventProducer` / `EventConsumer` + `InMemoryEventPublisher` | IMPLEMENTED (legacy Axum services) |
| `occurred_at` + `data` snapshot in envelope | PLANNED (Phase 2 / outbox) |
| `event_outbox` table + `OutboxPublisher` (same-tx) | PLANNED (Phase 2) |
| Relay worker + `FluvioSink` (`fluvio` feature) | PLANNED (Phase 3) |
| Topic-per-entity, partition-by-`pid`, replay/offsets | PLANNED |
| Downstream consumers (re-indexer, cache invalidation, analytics) | PLANNED |
| Shared `mxi-events` crate | PLANNED (open question) |

## See also

- [`agents/share/event-bus.md`](../../agents/share/event-bus.md) — the full durable-bus design (primary source)
- [`agents/share/loco.md`](../../agents/share/loco.md) — Postgres-backed workers
- [`agents/share/auditability.md`](../../agents/share/auditability.md) — audit log + event-stream summary
- [`spec/postgresql/index.md`](../postgresql/index.md) — §10 background jobs, SKIP LOCKED
- [`spec/architecture/index.md`](../architecture/index.md) — family architecture
- [`spec/merge/index.md`](../merge/index.md) — record merge
