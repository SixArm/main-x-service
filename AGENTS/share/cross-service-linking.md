# Cross-service entity linking — design

How the Main X Index family represents **typed links between records that
live in different services** — e.g. "this person *is the same human as*
that worker", "this person *works at* that organization", "this case is
*about* that person". This is a design document: it fixes the reference
format, the storage topology, the event shape, the integrity lifecycle,
and the rollout, so each crate adopts it without re-litigating. It builds
directly on the [durable event bus](event-bus.md) (the aggregator is one
of its §9 consumers) and is distinct from the **within-entity**
`relationships` field on each domain model (§7 below).

## 1. Why change

Today every `relationships[]` is **within one entity** — a person relates
to another *person*, a place *contains* another *place*. There is no way
to say a person *is* a worker, or *works at* an organization, because
those records live in separate services with separate databases. The
federated index is meant to fan out across entities; the missing piece is
a cross-service edge.

The chosen shape is the **hybrid** topology: services own their link
**writes** locally (no service calls another), and a standalone **read-model
aggregator** consumes the event stream to build the queryable graph. This
keeps writes decoupled and gives one place to traverse the graph, at the
cost of eventual consistency and a reconciliation duty (§6, §8).

> Architecture chosen over the alternatives — a dedicated synchronous
> link service, or extending the within-entity `relationships` field — for
> local-write decoupling plus a first-class graph query surface. The two
> costs that shape this spec: **two sources of truth** (per-service writes
> + the aggregated graph → §8 reconciliation) and **eventual consistency**
> (→ §6 freshness). See §11 for the rejected options.

## 2. Goals & non-goals

**Goals**

- A typed, directed (or symmetric) edge between two records in different
  services, with provenance, confidence, and optional validity dates.
- **Local writes**: an entity service records its outbound edges in its
  own DB and never makes a cross-service call on the write path.
- **One queryable graph**: neighbours, paths, and a "single view" of one
  real-world identity, served by one aggregator.
- Integrity that tolerates target-service downtime (optimistic + async).
- Clean separation from within-entity matching (§7): cross-service links
  are **never** a match signal.

**Non-goals**

- Replacing within-entity `relationships` (those stay; they ARE matcher
  signals — §7).
- Strong write-time referential integrity (no FK across services; §5).
- Event sourcing — Postgres per service stays the system of record; the
  aggregator's graph is a derived read-model.
- A shared runtime library — only the `EntityRef` value type and the edge-
  kind registry are shared *contracts*, copied per project (drift-accepted,
  same posture as [event-bus.md](event-bus.md) `mxi-events`).

## 3. The `EntityRef` — the one shared contract

A record in another service is named by an opaque **URN string**:

```
<entity_type>:<id>          e.g.  person:0c4f1e2a-…
                                  organization:9a2f-…
                                  courseinstance:7b3d-…   (type, not service)
```

- `entity_type` is globally unique across the family (`person`, `worker`,
  `organization`, `case`, `place`, `thing`, `event`, `course`,
  `courseinstance`, `care_pathway`). The owning **service** is a static
  `entity_type → service` lookup, so the ref need not encode the service.
  Multi-entity services (course hosts `course` + `courseinstance`) are why
  the **type**, not the service, is the discriminator.
- `id` is the record's public UUID (`pid`), matching the event envelope's
  `pid` ([event-bus.md §4](event-bus.md)).
- A tiny value type owns `parse` / `Display` and the `entity_type → service`
  map. It is pure data with no behaviour; **copy it per project** rather
  than packaging it (drift is cheap; the format is frozen here).

```rust
pub struct EntityRef { pub entity_type: EntityType, pub id: Uuid }
// Display => "person:0c4f…"; FromStr parses & validates the type.
```

This single string is also what makes the aggregator's graph indexable: it
is one `TEXT` column, indexed on both endpoints.

## 4. Topology (the hybrid model)

```
 write path (local, per service — no cross-service calls)
 ┌── person-svc ──┐  ┌── worker-svc ─┐  ┌── org-svc ──┐  ┌── case-svc ──┐
 │ entity_links   │  │ entity_links  │  │ entity_links│  │ entity_links │
 │  (outbound)    │  │  (outbound)   │  │  (outbound) │  │  (outbound)  │
 │  + emit        │  │  + emit       │  │  + emit     │  │  + emit      │
 │  linked /      │  │  linked /     │  │  linked /   │  │  linked /    │
 │  unlinked      │  │  unlinked     │  │  unlinked   │  │  unlinked    │
 └──────┬─────────┘  └──────┬────────┘  └─────┬───────┘  └──────┬───────┘
        │ (outbox → bus, event-bus.md)        │                 │
        └───────────────┬─────────────────────┴─────────────────┘
                        ▼
            ┌── link-graph-service-with-loco ──────────────┐
            │  Fluvio consumer of ALL entity topics +      │
            │  the new linked/unlinked events              │
            │                                              │
            │  read-model: edges (bidirectional, indexed)  │
            │  + entity_presence (from created/deleted)    │
            │  verification (§5) · merge-repoint (§5.3)    │
            │  reconciliation (§8) · freshness (§6)        │
            │                                              │
            │  GET /neighbors /edges /single-view /freshness│
            └──────────────────────────────────────────────┘
```

### 4.1 Write side — `entity_links` (per participating service)

Each service that can *originate* an edge gets one table. This is its
**outbound** edges only; the inverse is the other endpoint's concern (and
the aggregator stores both directions). It is **separate** from the within-
entity `relationships` JSONB (§7).

```sql
CREATE TABLE entity_links (
    id           UUID PRIMARY KEY,
    from_pid     UUID NOT NULL,          -- this service's record (FK, local)
    kind         TEXT NOT NULL,          -- edge kind (§9 registry)
    to_ref       TEXT NOT NULL,          -- EntityRef URN of the far record
    role         TEXT,                   -- e.g. job title for employed_by
    confidence   DOUBLE PRECISION,       -- 1.0 operator-asserted; <1 suggested
    provenance   TEXT NOT NULL,          -- operator | import | matcher_suggested
    valid_from   DATE,                   -- affiliation start (nullable)
    valid_to     DATE,                   -- affiliation end ("former …")
    created_at   TIMESTAMPTZ NOT NULL,
    deleted_at   TIMESTAMPTZ,            -- soft-delete (withdrawn edge)
    UNIQUE (from_pid, kind, to_ref, valid_from)   -- idempotent upsert key
);
```

REST surface (per service, mirroring its existing controller style):

```
POST   /api/<plural>/{pid}/links        create/upsert an outbound edge
GET    /api/<plural>/{pid}/links        list this record's outbound edges
DELETE /api/<plural>/{pid}/links/{id}   soft-delete (emits unlinked)
```

The write is **optimistic** (§5): it stores the assertion and emits an
event; it does **not** call the target service. Verification status is not
a write-side property — it is the aggregator's view, because only the
aggregator sees both ends.

### 4.2 The `linked` / `unlinked` events

Two new `kind` values on the existing event envelope
([event-bus.md §4](event-bus.md)); same outbox/relay path, no new
transport. The envelope's `entity`/`pid` are the **from** side; the edge
detail rides in `data`:

```jsonc
{
  "entity": "person", "pid": "0c4f…", "kind": "linked",
  "data": {
    "edge_id": "…", "from_ref": "person:0c4f…", "to_ref": "organization:9a2f…",
    "edge_kind": "works_at", "role": "Nurse",
    "confidence": 1.0, "provenance": "operator",
    "valid_from": "2019-04-01", "valid_to": null
  }
}
```

`unlinked` carries `{edge_id}` (and the refs) so the aggregator can remove
the edge. Consumers dedupe on the envelope `event_id` (at-least-once
delivery, [event-bus.md §6](event-bus.md)).

### 4.3 Read side — `link-graph-service-with-loco` (the aggregator)

A new standalone loco service, modelled on the service-crate template but
**read-only to the world** (its writes are event-driven). It is the §9
consumer foreshadowed in [event-bus.md](event-bus.md).

```sql
-- the bidirectional, queryable graph (derived)
CREATE TABLE edges (
    edge_id      UUID PRIMARY KEY,       -- = source linked event's edge_id
    from_ref     TEXT NOT NULL,
    to_ref       TEXT NOT NULL,
    kind         TEXT NOT NULL,
    directed     BOOLEAN NOT NULL,
    role         TEXT,
    confidence   DOUBLE PRECISION,
    provenance   TEXT NOT NULL,
    valid_from   DATE, valid_to DATE,
    status       TEXT NOT NULL,          -- unverified | verified | dangling (§5)
    observed_at  TIMESTAMPTZ NOT NULL,   -- when the linked event was consumed
    source_event_id UUID NOT NULL
);
CREATE INDEX edges_from ON edges (from_ref, kind);
CREATE INDEX edges_to   ON edges (to_ref,   kind);   -- inverse lookups

-- existence oracle, fed by entity created/deleted events
CREATE TABLE entity_presence (ref TEXT PRIMARY KEY, alive BOOLEAN NOT NULL,
                              last_seq BIGINT NOT NULL);
```

- **Symmetric kinds** (`same_identity`) are canonicalised to one row with
  the lexicographically smaller ref as `from_ref`, so the pair is stored
  once regardless of which side asserted it.
- **Neighbours** in both directions = index lookup on `from_ref` *and*
  `to_ref`. **Multi-hop** is a Postgres recursive CTE; v1 caps depth (§11).

Read API:

```
GET /api/neighbors/{ref}?kind=&direction=out|in|both&depth=1
GET /api/edges?from=&to=&kind=&status=
GET /api/single-view/{ref}     -- golden-record walk over same_identity + affiliations
GET /api/health/freshness      -- per-entity consumer lag (§6)
```

Every graph response carries an `as_of` timestamp (§6).

## 5. Integrity — optimistic + async verify

There is no foreign key across services, so referential integrity is a
**lifecycle**, not a write-time check:

```
write (local) → emit linked → aggregator: both endpoints present?
                                  ├ both alive          → verified
                                  ├ endpoint not yet seen→ unverified
                                  └ endpoint was deleted → dangling
```

- **No cross-service call on write.** The originating service accepts the
  edge and emits the event; latency and availability are unaffected by the
  target service's state.
- **Async verification.** The aggregator decides `status` from its
  `entity_presence` oracle, which is fed by every entity's `created` /
  `deleted` events. A target deleted *after* the edge was created flips the
  edge to `dangling` (surfaced, not silently broken) — the failure mode a
  synchronous write-time check could never catch.

### 5.1 Interim before the durable bus

The async oracle is only reliable once the [durable bus](event-bus.md) is
live; the current in-memory bus loses events on restart. Until then the
aggregator uses **lazy verify-on-read**: on a `neighbors`/`single-view`
query, any endpoint with unknown presence is resolved by a one-shot
`GET /{id}` to its source service and the verdict is cached in
`entity_presence`. This needs no durable log, at the cost of first-read
latency. The event-driven path supersedes it per-entity as topics go
durable.

### 5.2 Provenance & the suggestion queue

`provenance = matcher_suggested` edges (e.g. a cross-service
`same_identity` proposed by a future cross-service matcher) enter at
`confidence < 1.0` and surface in a **review queue** — the same pattern as
the existing within-service duplicate review. Operator confirmation
promotes them to `confidence = 1.0, provenance = operator`.

### 5.3 Merge repointing (why one aggregator helps)

Every entity service already publishes `merged {pid, merged_from}` on a
record merge. The aggregator consumes it and **repoints** every edge
referencing `merged_from` to `pid`, centrally, in one handler. (With
decentralised links this fix-up would have to fan out to every service
holding such an edge — a strong reason the graph is aggregated.)

## 6. Eventual consistency & freshness

The read-model trails the writes by the bus delivery lag. The spec makes
this **visible** rather than hiding it:

- `GET /health/freshness` returns, per entity topic, the `occurred_at` of
  the last consumed event and the lag versus now.
- Every graph response includes `as_of` = the read-model's freshness
  watermark, so a UI can show "graph as of 10:42:05" and explain why a
  just-created link is not yet present.

## 7. Relationship to within-entity matching (the partition rule)

This is the load-bearing rule that keeps cross-service links from
corrupting the matchers:

- **Within-entity `relationships`** (the `Vec<…Relationship>` on each
  domain model) stays exactly as is: same-service references, and **a
  matcher signal** — `relationships_score` is a Jaccard component in every
  matcher.
- **Cross-service links** live **only** in `entity_links`, the
  `linked`/`unlinked` events, and the aggregator. They are **never** stored
  in `relationships` and **never** fed to any matcher. A matcher scores
  two records' *sameness*; "person works at org" is not sameness evidence.
- The one permitted flow is the **reverse**: a confirmed `same_identity`
  edge MAY in future be *produced* by a cross-service matcher (roadmap),
  but is still not *consumed* by any within-entity matcher.

Each participating entity's domain model and matcher spec states this
partition explicitly so the boundary is not eroded by a later edit.

## 8. Reconciliation (the cost of two sources of truth)

Because the per-service `entity_links` and the aggregator's `edges` are two
stores, they can diverge (a dropped event, a relay bug). A periodic
reconciliation worker:

1. Pulls each service's authoritative `entity_links` (a bulk read endpoint
   or a topic replay from offset 0).
2. Diffs against the read-model `edges`.
3. Emits a **divergence metric** (count of edges present in one and not the
   other) and repairs the read-model.

Divergence is an SLO: steady-state should be ~0; a rising count is the
signal that the relay or a consumer is unhealthy.

## 9. v1 edge-kind registry

The edge kinds are a closed, centrally-defined set (in this doc + the
aggregator). Each kind fixes its endpoint types, direction, inverse,
whether it is time-bounded, and its sensitivity.

| Kind | From → To | Direction | Card. | Temporal | Inverse | Sensitivity |
|---|---|---|---|---|---|---|
| `same_identity` | person ↔ worker | symmetric | 1:1 | no | (self) | medium — identity assertion; operator-asserted/high-confidence |
| `works_at` / `member_of` | person → organization | directed | M:N | yes | `has_member` | medium |
| `employed_by` | worker → organization | directed | M:N | yes (+`role`) | `employs` | medium |
| `subject_of` / `about` | case → person | directed | M:N | sometimes | `is_subject_of` | **high** — see §10 |

Notes:
- `same_identity` is the **federation backbone**: it resolves one human
  across the general (person) and workforce (worker) registries and powers
  `single-view`. With `same_identity` + `employed_by`, a person's employer
  is *derivable* (person → worker → org).
- Adding a kind later (e.g. `course` `taught_by` `worker`) is just a new
  registry row + endpoint-type pair + inverse; the topology is unchanged.

## 10. Governance — `case ↔ person`

The `subject_of` edge is itself **sensitive data**: it asserts a person is
the subject of a government case (benefits, legal, investigation). It
therefore carries the case service's compliance posture, not the lighter
affiliation posture:

- **Access control** on both creating and reading the edge — at least the
  authorisation required to read the case.
- **Audit** every read/write of these edges (the link, and any
  `single-view` that surfaces it), consistent with the case service's
  audit trail.
- **Privacy masking**: the aggregator's `single-view` and `neighbors`
  responses MUST honour the same masking/authorisation as the case service;
  an unauthorised caller does not learn that the edge exists.
- This is why `case ↔ person` is the highest-governance v1 kind even though
  it is technically the same edge shape as the others.

## 11. Rollout

1. **Contracts.** ✅ *Landed 2026-07-06* as the standalone reference crate
   [`link/entity-ref-rust-crate`](../../link/entity-ref-rust-crate) —
   `EntityType` (+ `service()` map), `EntityRef` (URN parse/`Display`/serde
   as one `TEXT` column), and the §9 `EdgeKind` registry
   (`is_symmetric`/`is_temporal`/`inverse`/`sensitivity`/`permits`). Pure,
   dependency-light, fully unit-tested; copy per project (or depend on it)
   as the other rollout steps land. No behaviour yet.
2. **Backbone.** Add `entity_links` write-side + `linked`/`unlinked` events
   to **person** and **worker**; ship `same_identity`.
   - **Partial / reordered — ✅ case `subject_of` write-side landed
     2026-07-10** as the **reference implementation**
     ([`case/case-service-with-loco`](../../case/case-service-with-loco):
     `entity_links` migration, `POST`/`GET`/`DELETE
     /api/cases/{pid}/links`, idempotent upsert, `linked`/`unlinked` on
     the additive `Envelope.data` via the transactional emit seam, §10
     read-the-case authz + audit, depends on the `entity-ref` crate). This
     deviates from the nominal step-2 wording (person + worker
     `same_identity` first): case is the first **loco** service that both
     *originates* a v1 edge (§9) AND already has the durable-bus
     outbox/streaming to emit the events, whereas person/worker are older
     axum-style services with no event bus. Their `same_identity`
     write-side therefore **awaits their own event infrastructure**; the
     contract and the emit pattern are now proven on case.
   - **✅ person `same_identity` write-side landed 2026-07-10**
     ([`person/person-service-with-loco`](../../person/person-service-with-loco):
     `entity_links` migration, `POST`/`GET`/`DELETE
     /api/persons/{id}/links`, idempotent upsert, plus the aggregator's
     reconciliation pull `GET /api/persons/links[?since=]` returning the
     canonical §4.2 `{ "edges": [EdgeDetail…] }`; `validate_edge` accepts
     only `same_identity` person → worker; person record-level authz +
     audit; depends on the `entity-ref` crate). The bulk endpoint is the
     sync path — cross-service `linked`/`unlinked` **event** emission is
     deferred (person's durable `Envelope` has no link kind/`data`).
     **Worker's symmetric side is the remaining follow-up.**
3. **Aggregator.** Stand up `link-graph-service-with-loco` consuming the
   in-memory→outbox stream; `neighbors` + `single-view` reads; lazy
   verify-on-read (§5.1).
4. **Affiliations.** Add `person↔org` and `worker↔org`; then `case↔person`
   with its §10 authz/audit/masking story.
5. **Hardening.** Reconciliation worker (§8) + freshness metrics (§6); flip
   to the durable bus per entity as Fluvio topics go live (§5.1).

## 12. Open questions

- **Symmetric-write ownership.** For `same_identity`, may either side
  assert (person *or* worker), with the aggregator canonicalising and
  deduping on the ordered pair? (Lean: yes — both emit; dedupe on canonical
  pair.)
- **Shared `mxi-links` crate** vs copy-per-project for `EntityRef` + the
  registry. (Lean: copy until a second non-aggregator consumer exists —
  same call as `mxi-events`.)
- **Traversal depth.** Cap `neighbors` at depth 1–2 in v1, or expose
  arbitrary-depth recursive CTE? (Lean: cap at 2; revisit with real query
  patterns.)
- **Bulk-read vs replay** for reconciliation (§8) — a per-service
  `GET /links?since=` endpoint, or a topic replay? (Lean: replay once the
  bus is durable; a bulk endpoint in the interim.)

## 13. Rejected alternatives

- **Dedicated synchronous link service** — services write straight to a
  central link store; strong consistency and the simplest single graph, but
  every link write becomes a cross-service call (write availability coupled
  to the link service). Rejected for write coupling.
- **Extend within-entity `relationships`** — cheapest (no new service) but
  pollutes the matcher signal (every matcher would have to partition
  scored/unscored kinds — 9 crates, a permanent footgun), gives no global
  graph (fan-out reads), and erodes the federation boundary. Rejected on
  §7 grounds.
