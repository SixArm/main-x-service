# Runbook: event-bus outage and replay

The OPS-1 slice for the durable event bus's producer (each entity
service's outbox → relay → Fluvio) and consumer (link-graph) sides. See
[`event-bus.md`](../event-bus.md) for the design;
[`reconciliation-divergence.md`](reconciliation-divergence.md) for the
periodic integrity check that catches what this path misses.

**Read this first: there is no replay.** Despite the name several
design docs use for this rollout step, no CLI task, admin endpoint, or
documented SQL procedure exists anywhere in the family to re-publish
outbox rows or re-consume a topic from an earlier point. This runbook is
therefore about **detecting and containing** an outage, and the one or
two manual, undocumented-elsewhere levers that exist — not a "run this
command to replay" procedure, because that command doesn't exist yet.

## The producer side — outbox + relay

Every entity service (not link-graph, not authentication) that runs
`<ENTITY>_EVENT_TRANSPORT=outbox` writes an `event_outbox` row in the
same transaction as the entity change, then a background relay drains
it. **Delivery status is strictly binary** — `published_at IS NULL`
(pending) or not (sent) — there is no attempts counter, no last-error
column, no dead-letter table.

### What "sent" actually means, and where it can lie

`mark_published` runs after the relay's send call returns `Ok`. Three
distinct ways a row can be marked published with **no message ever
reaching a real broker**:

1. **`<ENTITY>_FLUVIO_ENDPOINT` unset.** The relay still runs (if
   `<ENTITY>_EVENT_RELAY` is on) using `LoggingSink`, which logs the
   event at INFO and returns `Ok(())` unconditionally. This is the
   intended dev/CI behaviour, and it is *also* exactly what "silently
   draining to nowhere" looks like in a real deployment that forgot the
   endpoint. The boot log line names which sink is active — check
   `fluvio_endpoint="(none — LoggingSink)"` in the `starting event-outbox
   relay` line first, always.
2. **The Fluvio send call doesn't wait for a broker acknowledgement.**
   The relay calls the producer's send and moves on without awaiting the
   returned handle's completion or flushing — so a row can be marked
   published while its record is still only locally batched, not
   confirmed by the broker. This is an upstream client-library behaviour
   this codebase's relay doesn't currently compensate for.
3. **Retention purges published rows on age alone.** A row marked
   published by either of the above is eligible for deletion after
   `<ENTITY>_EVENT_RETENTION_DAYS` (default `7`) — so a misconfigured
   relay doesn't just fail to deliver, it eventually erases the local
   evidence that it tried.

**Retention never touches an *undelivered* row** — the purge query
filters `published_at IS NOT NULL`, so an outage backs the table up
(disk growth) rather than silently losing unsent events. That is the
right trade-off, but there is no cap or alarm on outbox table size
anywhere in the code — an unbounded backlog is a real, undetected risk,
not a hypothetical one.

### Send-failure behaviour: head-of-line blocking, by design

When a send genuinely errors, the relay logs `relay send failed; will
retry` and stops draining the rest of the batch — deliberately, to
preserve per-record ordering. There is no per-row attempt count and no
backoff: the same row is retried every tick, forever, and every row
after it in publish order waits behind it. **One permanently-poisoned
row (a payload the broker rejects) stalls that entire service's outbox
indefinitely**, and the only signal is the same warning line repeating
every `<ENTITY>_EVENT_RELAY_INTERVAL_SECS` (default `5`s).

### Checks — producer side

There are **no Prometheus metrics for the bus at all** on the producer
side — no outbox-depth gauge, no relay-lag metric, no publish or
failure counter. The checks that exist:

| Check | How | What it tells you |
|---|---|---|
| Backlog size | `SELECT count(*), min(id), min(created_at) FROM event_outbox WHERE published_at IS NULL;` (cheap — backed by a partial index) | A steadily growing count, or a `min(id)` that never advances across ticks, is the head-of-line-blocked signal |
| Which sink is active | boot log, `starting event-outbox relay` line, field `fluvio_endpoint=` | `(none — LoggingSink)` means nothing is reaching a real broker, whatever `published_at` says |
| A stuck row | grep `relay send failed; will retry` | The `id` named is the head-of-line blocker; investigate that specific payload against the broker's own logs |
| Whole-pass failure | grep `relay drain pass failed` | The poll or ack query itself is failing — likely a DB issue, not a broker issue |
| Retention running | grep `relay purged old published outbox rows` (only logged when non-zero) | Confirms retention is alive; absence over `<ENTITY>_EVENT_RETENTION_DAYS` worth of time is itself worth investigating |
| Misconfigured feature | boot log, `ERROR` level: `<ENTITY>_FLUVIO_ENDPOINT is set but this binary was built without the fluvio cargo feature; the relay will NOT start` | The relay refuses to start rather than silently falling back to `LoggingSink` — but only when the transport is `outbox` *and* `<ENTITY>_EVENT_RELAY` is on; with `<ENTITY>_EVENT_TRANSPORT=memory` this check is never reached and the error is never logged even with a broken endpoint configured |

## The consumer side — link-graph

`link-graph-service` runs one Fluvio consumer task per entity topic (ten
total) plus a `processed_events` purge loop, gated entirely on
`LINK_GRAPH_FLUVIO_ENDPOINT` being set.

### Reconnect behaviour

A dead or unreachable broker — at startup or mid-stream, same code path
either way — logs `bus consumer stream ended; reconnecting` and retries
after a **fixed 5-second backoff**, forever. Not exponential, no jitter,
no circuit breaker. With ten topics, a broker outage produces roughly
120 warning lines a minute across the fleet of consumer tasks — expect
that volume and don't mistake it for something worse than "broker is
down."

A malformed envelope or a database error while applying one event is
logged (`malformed envelope on topic; skipped` / `apply_event_idempotent
failed; event skipped`) and the record is **skipped**, not retried — one
bad record can't wedge a topic, but a persistent DB error during apply
silently drops events while the Fluvio offset still advances (the
consumer uses automatic offset management), so those specific events are
not recoverable except by a reset this codebase doesn't provide (see
below).

### Dedup window

`processed_events` dedupes by `event_id` for
`LINK_GRAPH_PROCESSED_EVENTS_RETENTION_DAYS` (default `7`), purged every
`_PURGE_INTERVAL_SECS` (default `3600`). An event re-delivered *within*
that window is a no-op; **an envelope with no `event_id` at all bypasses
dedup entirely** and would be reapplied on every redelivery — this
should not occur on the current envelope shape, but is worth knowing if
you ever see a duplicate-looking edge appear repeatedly.

### The consumer-lag metric may be reading zero no matter how far behind you actually are

`link_graph_consumer_lag_seconds{entity}` is computed from
`consumer_offsets.last_occurred_at`, which is populated from each
event's `occurred_at` field — falling back to "now" when that field is
absent. At least one producer's event envelope does not currently carry
`occurred_at` on the wire at all (a documented Phase-1 simplification,
not an oversight specific to this runbook), which means that entity's
lag gauge will read close to zero **regardless of real consumption lag**.
Do not trust this metric alone for that entity; cross-check against
`GET /api/health/freshness`'s per-topic timestamp and, if you suspect
real lag, against the producer's own outbox backlog query above.

### Checks — consumer side

| Check | How | What it tells you |
|---|---|---|
| Per-topic lag | `GET /metrics.prom` → `link_graph_consumer_lag_seconds{entity=...}`, or `GET /api/health/freshness` | Real for entities whose envelope carries `occurred_at`; unreliable (reads near-zero) for any that don't — see above |
| Events actually applied | `GET /metrics.prom` → `link_graph_events_processed_total{kind=...}` | The one genuinely trustworthy consumer counter |
| Reconnect churn | grep `bus consumer stream ended; reconnecting` | Volume and duration tell you how long the broker's been unreachable |
| Skipped events | grep `apply_event_idempotent failed; event skipped` | Each one is a permanently-dropped event for that specific record — note the entity/pid from the log and consider whether reconciliation (a separate mechanism) will eventually repair it |
| Misconfigured feature | boot log, `ERROR`: `LINK_GRAPH_FLUVIO_ENDPOINT is set but this binary was built without the fluvio cargo feature; no bus consumer will start` | Same refuse-don't-fallback posture as the producer side |

## Symptoms → checks → actions

**"Outbox rows are piling up and nothing is being delivered."**
Check the boot log's `fluvio_endpoint=` field first — `(none —
LoggingSink)` means there was never a broker to begin with, whatever
`<ENTITY>_EVENT_TRANSPORT` says. If a real endpoint is configured, check
for the repeating `relay send failed; will retry` line and its `id` —
that row is blocking everything behind it; the underlying error names
what the broker rejected.

**"Rows are marked published but link-graph never applied the
corresponding edges."**
This is exactly the gap between "the relay's send call returned" and
"the broker actually has it" — the relay does not wait for a broker
acknowledgement or flush. There is no way to distinguish this from "the
consumer skipped it" after the fact from the producer side alone; check
the consumer's `event skipped` logs and `events_processed_total` for
that entity/kind first.

**"The consumer keeps reconnecting and nothing is coming through."**
Confirm the broker is actually reachable from link-graph's network
(the 5 s fixed retry will mask a genuinely-down broker as routine churn
for however long it stays down — there's no distinction in the log
between "broker restarting" and "broker gone for a day"). Once
reachable, consumption resumes from wherever the named per-topic
consumer's offset last was — there is no gap-filling.

**"I need to re-deliver events from before now — a replay."**
There isn't one. The closest latent capabilities, neither documented,
tested, nor safe to assume: (a) resetting/deleting the Fluvio-side named
consumer (`link-graph-<topic>`) via the `fluvio` CLI directly against the
broker would cause link-graph to restart consumption from offset zero
for that topic on its next connect, replaying everything the broker
still retains; (b) manually `UPDATE event_outbox SET published_at = NULL
WHERE …` on a producer would make its relay re-ship those specific rows,
safe only while they're still within `<ENTITY>_EVENT_RETENTION_DAYS` of
their original publish. Both are inferences from how the pieces work,
not supported procedures — treat them as a last resort, understand the
dedup-window and offset-strategy consequences (§ above) before using
either, and file this gap rather than normalizing it as "the way we do
it."

**"I set `<ENTITY>_FLUVIO_ENDPOINT` and nothing happened — no error,
no relay, no consumer."**
Check that the binary was actually built with the crate's own `fluvio`
Cargo feature — an endpoint configured without the feature logs an
`ERROR` and refuses to start, by design, rather than silently falling
back to `LoggingSink`/no-consumption. But that check only fires when the
transport/relay-enable gate is also satisfied — with `<ENTITY>_EVENT_
TRANSPORT=memory`, the feature check is never reached and you'll see no
error at all even with a broken configuration. Confirm the transport
mode first.

## Testing this yourself against a real broker

Neither the producer's nor the consumer's live-broker round-trip test
has ever been run in this repository's CI — both are `#[ignore]`-tagged
and verified only by compiling under the `fluvio` feature. If you want
to actually exercise the path rather than trust the code reading:

```sh
# from the relevant crate directory (case-service for the producer side,
# link-graph-service for the consumer side)
podman compose -f compose.fluvio.yaml up -d
CASE_FLUVIO_ENDPOINT=127.0.0.1:9103 \
  cargo test --features fluvio --test fluvio_relay -- --ignored
# (also needs Postgres — scripts/test-db.sh up <crate> first)
podman compose -f compose.fluvio.yaml down -v
```

The compose file stands up a real, if minimal, Fluvio cluster (an SC and
one SPU) — but it has never been exercised here either, so treat getting
it running cleanly as a real first step, not a formality. Neither test
nor compose file creates the `mxi.<entity>.events` topic explicitly;
whether Fluvio auto-creates it on first produce in this configuration is
unverified.

## What this runbook cannot help you do

- **Replay anything.** This is the headline gap — no tooling exists.
- **See outbox depth or relay health as a metric.** Only a direct SQL
  query gives you this on the producer side today.
- **Trust the consumer lag metric uniformly across entities.** At least
  one producer's envelope doesn't carry the timestamp the metric needs.
- **Distinguish a "sent but not acknowledged" row from a genuinely
  delivered one**, after the fact, from the producer's own data.

These are code gaps the family should close, not operator error to work
around indefinitely — file them as follow-ups if a real outage makes any
of them bite.
