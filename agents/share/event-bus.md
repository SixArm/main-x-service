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
   it. *(Storage layer landed 2026-07-06 in the **care-pathway** service
   as the reference: the `event_outbox` migration, the SeaORM entity, and
   `models::event_outbox` — the pure `OutboxInsert::from_envelope`
   envelope→row mapping (DB-free unit-tested), a `ConnectionTrait`-generic
   `enqueue` (so a handler passes its own transaction), and the relay
   `unpublished` / `mark_published` poll+ack. Remaining: the tx-aware
   `OutboxPublisher` behind the seam + switching handlers onto an explicit
   transaction.)*
3. **Add the relay worker + `FluvioSink`** behind feature `fluvio`. A
   DB-gated integration test (or a Fluvio test container) asserts an
   enqueued row reaches the topic and is marked published. *(Relay +
   `LoggingSink` landed 2026-08-02 in **case**, adapted from the
   organization reference. `FluvioSink` itself landed 2026-08-03, also in
   case (BUS-1): `fluvio` 0.50, one producer per topic, partitioned on
   `pid`. An endpoint configured (`<ENTITY>_FLUVIO_ENDPOINT`) without the
   `fluvio` feature refuses to start the relay rather than falling back
   to `LoggingSink` — that fallback would mark outbox rows `published_at`
   without ever reaching a real broker. `compose.fluvio.yaml` +
   `Dockerfile.fluvio-cli` (case) provision a local SC+SPU broker
   (Fluvio's own documented Docker Compose layout) for opt-in manual
   runs; no automated stage in this repo stands one up, so the
   feature-gated, `#[ignore]`d round-trip test is verified by compiling
   under `--features fluvio`, not by an actual execution — the "Fluvio
   test container" this step originally envisioned remains a follow-up.
   **`FluvioSink` itself is now rolled to all ten entity registries**
   (BUS-3, 2026-08-03 — person, worker, place, thing, event, course,
   organization, care-pathway, project-portfolio-management, joining
   case from BUS-1): same feature-gated + no-silent-fallback shape, an
   opt-in `compose.fluvio.yaml` local broker, and a feature-gated,
   `#[ignore]`d live round-trip test per crate, none ever executed in
   this repo's CI. The "reconcile the five older crates' dormant
   `fluvio` Cargo deps" sub-task BUS-3 also carried turned out to be
   moot — no `Cargo.toml` in the repo mentioned `fluvio` before BUS-1
   landed it, so there was nothing dormant to reconcile.
4. **Flip `<ENTITY>_EVENT_TRANSPORT=outbox`** per service in deployment;
   stand up consumers (search re-indexer first). *(The first real
   consumer landed 2026-08-03, BUS-2, in **link-graph-service** — the §9
   aggregator, ahead of a search re-indexer: one task per entity topic,
   behind a `fluvio` feature, calling a new `apply_event_idempotent`
   (dedup on `event_id` via a new `processed_events` table). Resume
   position is delegated to Fluvio's own named-consumer offset
   management rather than reconstructed in Postgres — see
   `link-graph-service-with-loco/spec/10-persistence.md` §10.3 for the
   full reasoning. Only **case** had `FluvioSink` wired at BUS-2 time
   (BUS-1); now that BUS-3 has rolled it to all ten, the aggregator
   consumes ten live topics but nine still see no traffic until a
   deployment actually sets `<ENTITY>_FLUVIO_ENDPOINT` and
   `<ENTITY>_EVENT_TRANSPORT=outbox` for them — this step remains a
   per-deployment decision, not something the family repo does for
   anyone.)*
5. Adopt per entity in spec-priority order; the in-memory default means
   un-migrated crates keep working throughout. **Done for the producer
   side (BUS-1/BUS-3) and the aggregator consumer (BUS-2)** — what
   remains is rolling *other* consumers (search re-indexer, cache
   invalidation, analytics — §9) and actually flipping any deployment's
   transport to `outbox` with a real broker, both outside this repo's
   own CI/test posture.

## 9. Consumers (initial set)

- **Cross-service link aggregator** ([link-graph-service](../../link/link-graph-service-with-loco))
  — **landed 2026-08-03, BUS-2**, the first real consumer in the family.
  See [cross-service-linking.md](cross-service-linking.md) §4.3 for its
  role and `link-graph-service-with-loco/src/consumer.rs` for the
  implementation.
- **Search re-indexer** — keeps Tantivy in sync with DB writes via the
  stream instead of inline indexing; replayable for full rebuilds.
- **Cross-entity cache invalidation** — e.g. a place address change
  notifies workers referencing it.
- **Analytics / audit aggregation** — a durable sink for the change feed
  (complements, doesn't replace, `audit_logs`).

Each is a standalone Fluvio consumer (Rust, copying the `Envelope`
shape per crate — see §4, §11), idempotent on `event_id`, tracking its
own offset.

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

## 12. Outbound webhook sink (`WebhookSink`)

A **best-effort notification** fan-out to operator-configured third-party
URLs, alongside (not instead of) the durable-bus sinks above (repo
`tasks.md` EV-3). [project-portfolio-management-service](../../project-portfolio-management/project-portfolio-management-service-with-loco)
is the first adopter (`src/webhooks.rs`, its own `spec/13-tasks.md`
T-28m); other registries copy this shape when a consumer asks for it —
the contract below is family-shaped, not portfolio-specific.

**Why "beside", not "instead of".** The relay's primary sink
(`LoggingSink`/`FluvioSink`) is the at-least-once path §5/§6 already
depend on: a send failure there leaves the outbox row unpublished, and
`drain_once` retries it — correct, because the durable bus is meant to
never silently drop an event. Webhook delivery must **never** gate that:
an operator's slow or unreachable receiver would otherwise stall the
entire outbox forever over a target the family has no control over. So
`WebhookSink::send` never returns an error — each matching target gets
its own bounded retry-with-backoff on a spawned task, and a target still
failing once that is exhausted is recorded in a delivery log and moved
past, never re-blocking the row it fanned out from. The reference
composes it via a `CompositeSink`: the **primary** sink's result still
governs `drain_once`'s retry exactly as a lone sink would; every
**secondary** sink (webhook or otherwise) is delivered to only after the
primary succeeds, and its own failure is swallowed (logged, never
propagated).

**Configuration** — `<ENTITY>_WEBHOOKS` (inline JSON) or
`<ENTITY>_WEBHOOKS_FILE` (a path; takes precedence when both are set,
mirroring the ABAC policy loader's precedence, §5 of
[authorization-attributes.md](authorization-attributes.md)) — a JSON
array of targets:

```json
[
  { "url": "https://ops.example.com/hook" },
  { "url": "https://audit.example.com/hook", "kinds": ["created", "merged"] }
]
```

`kinds` omitted or `null` means every kind (`created`/`updated`/
`deleted`/`merged`). Unset ⇒ no targets (the feature is off); malformed
JSON, or a file that fails to read, is logged and treated as no targets
— an optional feature's config typo must not block boot. Every target
must additionally be `https://`, or `http://` to a **loopback** host
(`127.0.0.1`/`::1`/`localhost`, for local dev) — the same SEC-V1/SEC-B11
posture the PASETO key fetch and the link-graph presence probe already
use; a target failing this check is dropped at load time with a
warning, not silently sent to in plaintext.

**Signing.** Every delivery carries `X-Mxi-Event-Id: <uuid>` (the
envelope's dedup key, per §6) and `X-Mxi-Signature: <tag>`, where `<tag>`
is the shared `integrity-mac` crate's `KeySet::tag` under its own
domain (`"webhook"`) — i.e. `"<scheme>.<key id>:<hex>"`, HMAC-SHA256
over the **exact request body bytes** (the compact JSON serialization of
the outbox envelope), signed before sending and never re-serialized
afterwards, so signer and sender always agree on what was signed. **This
is the published pre-image format** EV-3 requires for a receiver to
verify at all: HMAC-SHA256(subkey, raw POST body) compared against the
hex after the scheme/key-id prefix. A receiver never holds the service's
**root** MAC key — only the `webhook`-domain subkey, which an operator
derives offline (HKDF-SHA256 over the root key, `info` string
`mxi/<service>/webhook/<scheme>` — see `integrity-mac`'s module docs,
"Domain separation") and hands to the receiver as *their* configured
verification secret; deriving it under its own domain means handing it
out cannot be turned into a forged tag in any other domain (audit rows,
record digests, …). **No MAC key configured** ⇒ the relay refuses to
start webhook delivery at all (logged `error`) when targets are
configured, rather than send unsigned deliveries — the primary relay
sink is unaffected. Verified live in the reference implementation
(2026-09-08): a receiver-side HMAC recomputed independently from the
root key, the published `info` string, and the exact captured request
body matched the sent `X-Mxi-Signature` byte-for-byte.

**Retry policy.** A `5xx` response, or a transport-level failure (no
response at all — connection refused, timeout, TLS failure), is
**retried** with exponential backoff: the receiver or its infrastructure
is presumed to be having a transient problem. A `4xx` response is **not
retried**: it is the receiver rejecting this exact request, and
resending it unchanged would just repeat the rejection. Verified live in
the reference implementation: a receiver returning `503` twice then
`200` was attempted three times and recorded `delivered`; a receiver
returning `400` was attempted once and recorded `failed` — pinning this
section's own acceptance criterion.

**Delivery log.** One row per target's final outcome for one event
(not one row per HTTP attempt) — `event_id`, `entity`, `kind`, `url`,
`attempts`, `status` (`delivered`/`failed`), `status_code`, `error`. It
is the operator-facing record of what was (or was not) delivered; it
does not gate outbox progression.

## 13. Open questions

- Shared `mxi-events` crate now, or copy-per-crate? The stated trigger
  ("extract when the first real consumer ships") fired 2026-08-03 when
  link-graph-service (BUS-2) became the first real consumer, but no
  crate was extracted — `link-graph-service`'s `Envelope` is still
  defined locally in `src/events.rs`. Still open in practice; either
  extract now or drop the trigger condition as decided-against.
- Full snapshot in `data` vs reference-only for large records (case,
  person) — per-entity decision; default full.
- Schema-registry / Avro vs JSON envelope — JSON for v1 (matches today's
  self-describing Serde wire form); revisit if payloads grow.
- One relay worker per service vs a shared relay reading all outboxes —
  per-service is simpler and matches the deploy unit; start there.
