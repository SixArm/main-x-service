# Event bus: the transactional outbox, the relay, and `/events/recent`

This tutorial exercises
[`agents/share/event-bus.md`](../agents/share/event-bus.md) end to end on
**case-service** — the family's durable-event-bus **reference
implementation** and, per `agents/share/overview.md`'s capability-matrix
footnote, the one service whose Fluvio producer side is actually wired to
a real deployment target: `CASE_EVENT_TRANSPORT=outbox`, one `event_outbox`
row written **inside** the same transaction as the entity change, a
background relay that drains it, and the operator endpoint
`GET /api/cases/events/recent`. A create, an update, and a merge all get
triggered live so the outbox shows more than one event `kind`.

This tutorial does **not** stand up a real Fluvio broker. The repo's own
docs say plainly that no automated run anywhere in this codebase does that
today (`agents/share/event-bus.md` §8 step 3: "no automated stage in this
repo stands one up"), and reproducing that gap under a tutorial's time
budget wouldn't make the honest scope note any less true. §7 below covers
the Fluvio extension as configuration — clearly labelled as not exercised
in this session.

## Prerequisites

| Tool | Why | Tested with |
|---|---|---|
| [Podman](https://podman.io/) (not Docker) | the throwaway test Postgres instance | 6.0.2, with `podman machine` running |
| Rust (this repo pins `1.96.1` in [`rust-toolchain.toml`](../rust-toolchain.toml)) | builds and runs case-service directly | `cargo` on `PATH` |
| `curl` + `python3` (for `python3 -m json.tool`) | verifies everything live | whatever your OS ships |

## A correction before starting: `cargo loco` actually does work here — for a narrower reason than it sounds

Every prior tutorial that touched `cargo loco …` (TUT-2, TUT-3, TUT-5)
found it fails with "no such command: `loco`" and used `cargo run --
…` instead, because no global `cargo-loco` plugin binary is installed in
this environment. That finding still holds **in general** — but it turns
out not to hold for this crate specifically, and the reason is worth
being precise about rather than re-asserting the old blanket claim:

```sh
cd case/case-service-with-loco
cargo loco --version
```

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.10s
     Running `target/debug/case-service --version`
```

That ran. Looking at why: `case/case-service-with-loco/.cargo/config.toml`
carries

```toml
[alias]
loco = "run --"
```

— a **repo-local Cargo alias**, not the missing global plugin. It makes
`cargo loco X` expand to exactly `cargo run -- X` for *this crate only*,
so `cargo loco start` and `cargo loco db migrate` genuinely work here
(confirmed live below) for the same reason `cargo run -- start` always
has. It is present in case-service and authentication-service (both
newer, loco-scaffolded crates) but **absent** from person/worker/place's
`.cargo/config.toml` (older, hand-converted crates) — so the "no
`cargo-loco` shim" finding from earlier tutorials was correct for the
crates those tutorials used, and this tutorial's crate is simply a
different case. This tutorial still writes every command as
`cargo run -- …` throughout, both to stay literally accurate about what
is and isn't installed, and because that form works identically on every
crate in the family regardless of which one happens to carry the alias.

## 1. Start Postgres and case-service, with the outbox transport and the relay on

```sh
scripts/test-db.sh up case/case-service-with-loco
```

```
test-db: mxi-case-test-db ready
  DATABASE_URL=postgres://loco:loco@localhost:5432/case_service_test
```

```sh
cd case/case-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5432/case_service_test cargo run -- db migrate
```

```
Applying migration 'm20220101_000004_event_outbox' … has been applied
… (11 more migrations)
```

The transport (`CASE_EVENT_TRANSPORT`, default `memory`) and the relay
(`CASE_EVENT_RELAY`, default off) are two separate switches
(`agents/share/event-bus.md` §7, `src/relay.rs`) — both are needed for
this tutorial, so both go on the environment:

```sh
DATABASE_URL=postgres://loco:loco@localhost:5432/case_service_test \
  CASE_EVENT_TRANSPORT=outbox \
  CASE_EVENT_RELAY=true \
  CASE_EVENT_RELAY_INTERVAL_SECS=5 \
  cargo run -- start
```

```
[…] INFO app: case_service::relay: starting event-outbox relay
      interval_secs=5 retention_days=7 fluvio_endpoint="(none — LoggingSink)"

environment: development
   database: automigrate
     logger: debug
compilation: debug
      modes: server

listening on http://localhost:5150
```

Note `modes: server` — **not** `modes: server, worker`. Unlike TUT-5's
bulk-job worker (a loco `BackgroundQueue` worker that only registers under
`start --server-and-worker`), the outbox relay is **not** a loco worker at
all: `src/app.rs`'s `after_routes` hook calls `crate::relay::spawn(ctx.db
.clone())` directly with `tokio::spawn`, unconditionally on every boot —
`spawn` itself is what checks `CASE_EVENT_TRANSPORT`/`CASE_EVENT_RELAY`
and no-ops if either is off. So a plain `cargo run -- start` is
sufficient here; `--server-and-worker` is neither needed nor wrong, it
would just also start the (idle, since nothing is imported) bulk-job
worker alongside. The `starting event-outbox relay` log line above,
present on this boot, is the confirmation that matters.

```sh
curl -s http://localhost:5150/_health
```

```json
{"ok":true}
```

(Case-service's health payload is the loco default `{"ok":true}` —
unlike person-service's richer `/api/health` `{"status","service",
"version"}` shape TUT-1/TUT-5 show. Different crate, different endpoint;
not a regression.)

## 2. Trigger a create event

The body below is `examples/api/case.http`'s own curl-verified create
example (case.http's header notes it produced a real pid in a prior
run — this run gets its own):

```sh
curl -s -X POST http://localhost:5150/api/cases \
  -H "Content-Type: application/json" \
  -H "Accepts-version: 1.0" \
  -d '{
  "title": "Housing benefit appeal",
  "agency_id": "dwp",
  "case_number": "HB-2024-0007",
  "subjects": ["person:abc"],
  "keywords": ["housing", "benefit"],
  "identifiers": [{ "scheme": "Docket", "value": "CV-2024-001234" }]
}'
```

```json
{"pid":"ec863465-bac0-40ee-9caf-119b7a04b127","title":"Housing benefit appeal"}
```

Bare JSON, no `{success,data,error}` envelope — case.http's own header
note about case's wire shape, reconfirmed.

## 3. The outbox row — and what it actually proves

Query `event_outbox` directly (the table name is exactly that — no
pluralization; see the migration file's own comment about a loco
`create_table` helper bug this crate works around):

```sh
podman exec mxi-case-test-db psql -U loco -d case_service_test -c \
  "SELECT id, event_id, entity, entity_pid, kind, occurred_at, actor, schema_version, published_at FROM event_outbox ORDER BY id;"
```

```
 id |               event_id               | entity |              entity_pid              |  kind   |          occurred_at          | actor | schema_version |         published_at
----+--------------------------------------+--------+--------------------------------------+---------+-------------------------------+-------+----------------+-------------------------------
  1 | f9bbb57f-1f9c-4ae7-b3b4-0a03389588c0 | case   | ec863465-bac0-40ee-9caf-119b7a04b127 | created | 2026-08-04 10:43:13.389915+00 |       |              1 | 2026-08-04 10:43:16.332477+00
(1 row)
```

`entity_pid` matches the `pid` the create response returned, and the row
already carries a `published_at` — the relay reached it before this query
ran (§4). The full envelope survives verbatim as `payload`:

```sh
podman exec mxi-case-test-db psql -U loco -d case_service_test -t -A -c \
  "SELECT payload FROM event_outbox WHERE entity_pid = 'ec863465-bac0-40ee-9caf-119b7a04b127';"
```

```json
{"pid":"ec863465-bac0-40ee-9caf-119b7a04b127","seq":1,"kind":"created","name":"Housing benefit appeal","actor":null,"entity":"case","event_id":"f9bbb57f-1f9c-4ae7-b3b4-0a03389588c0","schema_version":1}
```

**Why this is the atomicity the design promises, and what curl alone
can't show it.** `agents/share/event-bus.md` §3's whole point is that the
entity row and its outbox row commit or roll back **together** — "no
committed change without its event, and no event without a committed
change." An HTTP response can't distinguish "written in the same
transaction" from "written a moment later by luck"; the actual guarantee
lives in the code, `src/streaming.rs::create_and_emit`:

```rust
EventTransport::Outbox => {
    let txn = db.begin().await?;
    let model = CaseModel::create(&txn, case).await?;
    let env = envelope(EventKind::Created, &model.pid.to_string(), &model.title, actor);
    OutboxPublisher.publish(&txn, &env).await?;
    AuditModel::record(&txn, model.pid, "created", actor, Some(model.data.clone())).await?;
    txn.commit().await?;
    model
}
```

The `cases` insert, the `event_outbox` insert, and the `audit_logs` insert
all run on the **same** `&txn`, and only `txn.commit()` at the end makes
any of them durable — a failure anywhere in that block (a constraint
violation, a dropped connection) rolls back all three, so an
`event_outbox` row can never exist for a case that doesn't, and vice
versa. What curl and psql *can* show, and what's shown above and in §6,
is evidence consistent with that: the `cases.created_at` and this row's
`occurred_at` are 14 ms apart (both stamped inside the same handler
invocation, before the shared commit), and — more strikingly, in §6 — two
events from one merge request (`merged` + `deleted`) land 1 ms apart and
get relayed in the exact same batch. Neither observation is a *proof* the
way reading the transaction boundary in the source is; they're the
externally-visible fingerprint of one.

## 4. The relay

The relay is `src/relay.rs`'s `drain_once`: it opens its own transaction,
selects unpublished rows with `FOR UPDATE SKIP LOCKED` (so two relay
instances never double-ship — SEC-B6), sends each to an `EventSink`, and
stamps `published_at` on the ones that succeeded. With no
`CASE_FLUVIO_ENDPOINT` configured, the sink is the no-broker
**`LoggingSink`** — it logs the event and always succeeds, so the drain +
retention machinery is fully exercised without a broker. Confirm it ran,
straight from the server log:

```sh
grep "relay: published outbox event" /tmp/case-service.log
```

```
[…] INFO case_service::relay: relay: published outbox event
    topic="mxi.case.events" key="ec863465-bac0-40ee-9caf-119b7a04b127"
    payload={"actor":null,"entity":"case","event_id":"f9bbb57f-…",
    "kind":"created","name":"Housing benefit appeal",
    "pid":"ec863465-bac0-40ee-9caf-119b7a04b127","schema_version":1,"seq":1}
```

`topic="mxi.case.events"` and `key=<pid>` are exactly §7's
`mxi.<entity>.events` topic-naming and pid-as-partition-key convention,
even though nothing is actually publishing to a Fluvio topic here — the
sink interface doesn't change shape between `LoggingSink` and the real
`FluvioSink` (§7).

**Real observed timing**: the row was created at `10:43:13.389915` and
`published_at` reads `10:43:16.332477` — about **2.9 s** later, against a
5 s poll interval (`CASE_EVENT_RELAY_INTERVAL_SECS=5`) that had been
ticking since server boot at `10:42:56`. That lines up: the next tick
after the write landed roughly 3 s later, not a full 5 s, because the
poll clock runs on its own schedule independent of when a row happens to
be written.

## 5. `GET /api/cases/events/recent`

```sh
curl -s http://localhost:5150/api/cases/events/recent
```

```json
[{"kind":"created","pid":"ec863465-bac0-40ee-9caf-119b7a04b127","name":"Housing benefit appeal","seq":1}]
```

The exact path is `/api/cases/events/recent` (mounted in
`controllers/cases.rs`'s route table as `.add("/events/recent",
get(recent_events))`), and the response is the frozen flat
`EventView{kind,pid,name,seq}` projection §4 of the design doc promises —
identical shape whether the active transport is `memory` or `outbox`
(`streaming::recent_events` switches source, not shape).

## 6. Update and merge — event ordering and more than one `kind`

An update, keeping the earlier PUT-status finding in mind: `case.http`'s
own shipped `PUT` example body uses `"status": "in_progress"`, which
TUT-3 found (and this run reconfirms) 422s — `case_matcher::CaseStatus`
has no `#[serde(rename_all)]`, so the wire form is the bare Rust variant
name, `"Open"` / `"Closed"`:

```sh
curl -s -X PUT http://localhost:5150/api/cases/ec863465-bac0-40ee-9caf-119b7a04b127 \
  -H "Content-Type: application/json" \
  -d '{"title":"Housing benefit appeal (amended)","agency_id":"dwp","case_number":"HB-2024-0007","status":"Open"}'
```

```json
{"pid":"ec863465-bac0-40ee-9caf-119b7a04b127","title":"Housing benefit appeal (amended)"}
```

`200`, using the corrected casing. Now create a second, deliberately
similar case and merge it into the first — `merge_and_emit` emits
`Merged` (on the survivor) and `Deleted` (on the retired duplicate) as
one atomic pair (`agents/share/match-search-merge.md`):

```sh
curl -s -X POST http://localhost:5150/api/cases \
  -H "Content-Type: application/json" \
  -d '{"title":"Housing benefit appeal","agency_id":"dwp","case_number":"HB-2024-0007-DUP","subjects":["person:abc"],"keywords":["housing","benefit"]}'
# → {"pid":"042a7ea6-7655-4a57-bac4-e79fffd5c3fe","title":"Housing benefit appeal"}

curl -s -X POST http://localhost:5150/api/cases/merge \
  -H "Content-Type: application/json" \
  -d '{"main_pid":"ec863465-bac0-40ee-9caf-119b7a04b127","duplicate_pid":"042a7ea6-7655-4a57-bac4-e79fffd5c3fe"}'
```

```json
{"duplicate_pid":"042a7ea6-7655-4a57-bac4-e79fffd5c3fe","main_pid":"ec863465-bac0-40ee-9caf-119b7a04b127", "main":{"title":"Housing benefit appeal (amended)","status":"Open", "…":"…"}}
```

The full outbox, five rows now:

```
 id |              entity_pid              |  kind   |          occurred_at          |         published_at
----+--------------------------------------+---------+-------------------------------+-------------------------------
  1 | ec863465-…-b127 | created | 2026-08-04 10:43:13.389915+00 | 2026-08-04 10:43:16.332477+00
  2 | ec863465-…-b127 | updated | 2026-08-04 10:45:48.868449+00 | 2026-08-04 10:45:52.078844+00
  3 | 042a7ea6-…-3fe  | created | 2026-08-04 10:45:54.144187+00 | 2026-08-04 10:45:57.248598+00
  4 | ec863465-…-b127 | merged  | 2026-08-04 10:45:56.988362+00 | 2026-08-04 10:45:57.248598+00
  5 | 042a7ea6-…-3fe  | deleted | 2026-08-04 10:45:56.98935+00  | 2026-08-04 10:45:57.248598+00
```

Two real details worth calling out. First, rows 4 and 5 — `merged` and
`deleted` from the **one** merge request — have `occurred_at` values 1 ms
apart and the **exact same** `published_at`: they were written together
(`merge_and_emit` runs both on one `&txn`, mirroring `create_and_emit` in
§3) and drained together in the same relay tick, which is the same
same-transaction fingerprint as §3 but now with two rows instead of one.
Second, row 3 (the duplicate's `created`) shares that same `published_at`
too, purely because it happened to land inside the same 5 s poll window as
the merge that followed 2.8 s later — the relay batches whatever has
accumulated by the time its tick fires (`drain_once` pulls up to 100 rows
per pass), not one row per tick.

`GET /api/cases/events/recent` reflects all five, **newest first**:

```json
[
  {"kind":"deleted","pid":"042a7ea6-…-3fe","name":"Housing benefit appeal","seq":5},
  {"kind":"merged","pid":"ec863465-…-b127","name":"Housing benefit appeal (amended)","seq":4},
  {"kind":"created","pid":"042a7ea6-…-3fe","name":"Housing benefit appeal","seq":3},
  {"kind":"updated","pid":"ec863465-…-b127","name":"Housing benefit appeal (amended)","seq":2},
  {"kind":"created","pid":"ec863465-…-b127","name":"Housing benefit appeal","seq":1}
]
```

`seq` is assigned when the envelope is **built** (in the handler, before
the transaction even opens), so it orders by write time regardless of
which relay batch later shipped a row — `merged` (seq 4) sorts ahead of
its own duplicate's `created` (seq 3) precisely because the merge request
came after the duplicate-create request, not because of anything to do
with the relay.

## 7. Extending this with a real broker (Fluvio) — configuration only, not run in this session

Everything above ran against `LoggingSink`. The real-broker sink,
`FluvioSink` (BUS-1, landed 2026-08-03; rolled to all ten entity
registries as BUS-3), lives behind this crate's own `fluvio` Cargo
feature — off by default, so nothing above required it. To point this at
an actual broker, per `agents/share/event-bus.md` §7 and `src/relay.rs`:

| Var | Meaning | Default |
|---|---|---|
| `CASE_EVENT_TRANSPORT` | `memory` \| `outbox` | `memory` |
| `CASE_EVENT_RELAY` | truthy to spawn the relay loop | off |
| `CASE_EVENT_RELAY_INTERVAL_SECS` | poll interval | `5` |
| `CASE_EVENT_RETENTION_DAYS` | published-row TTL | `7` |
| `CASE_FLUVIO_ENDPOINT` | Fluvio SC address; selects `FluvioSink` over `LoggingSink` | unset |
| `CASE_EVENT_TOPIC` | topic override | `mxi.case.events` |

Rebuild with `cargo build --features fluvio`, bring up a **local** broker
with the crate's own opt-in compose file
(`case/case-service-with-loco/compose.fluvio.yaml` — a Stream Controller
+ SPU pair matching Fluvio's own documented Docker Compose layout,
completely separate from `compose.test.yaml`), then run with
`CASE_FLUVIO_ENDPOINT=127.0.0.1:9103` pointed at it. Two guardrails worth
knowing before trying this: setting `CASE_FLUVIO_ENDPOINT` on a binary
built **without** `--features fluvio` makes the relay refuse to start
outright (logged at `error`) rather than silently falling back to
`LoggingSink` — a fallback there would mark outbox rows `published_at`
without ever reaching the broker the operator explicitly asked for. And
the initial Fluvio connection retries indefinitely rather than falling
back, for the identical reason.

**Not attempted in this tutorial, stated plainly**: no broker was stood
up and `FluvioSink` was not exercised. This matches the repository's own
documented posture — `compose.fluvio.yaml`'s header comment says outright
that it "has **not** been exercised in this repo's CI," and
`agents/share/event-bus.md` §8 step 3 says the same for every one of the
ten services carrying `FluvioSink` today: "no automated stage in this
repo stands one up." The `#[ignore]`d `fluvio_relay` test this feeds is
verified by compiling under the feature, not by an actual round-trip.
This tutorial's §1–§6 are the part of the design that a live session in
this environment can actually prove; §7 is the documented, honest
boundary of that.

## 8. Tear down

```sh
# Ctrl-C the cargo run -- start process
scripts/test-db.sh down case/case-service-with-loco
rm -f /tmp/case-service.log
```

`test-db.sh down` drops the tmpfs-backed Postgres container entirely
(same clean-slate guarantee as every prior tutorial's teardown — the
`event_outbox` table and everything in it goes with it).

```sh
podman ps -a
```

```
CONTAINER ID  IMAGE                                    COMMAND   CREATED  STATUS                          PORTS  NAMES
bf1d5e630324  mcr.microsoft.com/azure-sql-edge:latest  …         …        Exited (137) …                  …      fhir-mssql-db
```

Only the pre-existing, unrelated `fhir-mssql-db` container from a
different task remains — nothing from this tutorial.

## What's next

This is the last of the six planned tutorials (TUT-1 through TUT-6). See
[`tasks.md`](../tasks.md) — the only item left in the flattened task
order is `LNK-4`, which is spec-first work, not a tutorial.
