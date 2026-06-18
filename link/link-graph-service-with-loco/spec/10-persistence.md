## 10. Persistence

PostgreSQL via SeaORM. The schema source of truth is the hand-written,
numbered SQL under `migrations/` (`up.sql` / `down.sql` per step),
bridged into the loco SeaORM `Migrator` (each Rust migration wraps the
SQL pair via `include_str!`), consistent with the sibling service crates.

All tables here hold a **derived read-model**; none is a system of
record. The whole schema is reconstructable from a topic replay
(§7 NFR-4).

### 10.1 `edges` — the bidirectional, queryable graph

```sql
CREATE TABLE edges (
    edge_id      UUID PRIMARY KEY,       -- = source linked event's edge_id
    from_ref     TEXT NOT NULL,          -- EntityRef URN; canonical "from"
    to_ref       TEXT NOT NULL,          -- EntityRef URN
    kind         TEXT NOT NULL,          -- closed registry (§5.4 / design §9)
    directed     BOOLEAN NOT NULL,       -- false for symmetric kinds
    role         TEXT,                   -- e.g. job title for employed_by
    confidence   DOUBLE PRECISION,       -- 1.0 operator-asserted; <1 suggested
    provenance   TEXT NOT NULL,          -- operator | import | matcher_suggested
    valid_from   DATE,
    valid_to     DATE,
    status       TEXT NOT NULL,          -- unverified | verified | dangling
    observed_at  TIMESTAMPTZ NOT NULL,   -- when the linked event was consumed
    source_event_id UUID NOT NULL
);
CREATE INDEX edges_from ON edges (from_ref, kind);
CREATE INDEX edges_to   ON edges (to_ref,   kind);   -- inverse lookups
CREATE INDEX edges_status ON edges (status);          -- /edges?status= + metrics
```

- **Symmetric kinds** (`same_identity`) are canonicalised to one row
  with the lexicographically smaller ref as `from_ref` (§6 FR-6), so the
  pair is stored once.
- **Neighbours** in both directions = an index lookup on `from_ref`
  *and* `to_ref`. **Multi-hop** is a Postgres recursive CTE; v1 caps
  depth (§16).

### 10.2 `entity_presence` — the existence oracle

```sql
CREATE TABLE entity_presence (
    ref       TEXT PRIMARY KEY,          -- EntityRef URN
    alive     BOOLEAN NOT NULL,          -- created => true; deleted => false
    last_seq  BIGINT NOT NULL            -- per-entity_pid monotonic (envelope seq)
);
```

Fed by every entity's `created` / `deleted` events (§6 FR-8); the source
of the edge `status` lifecycle (§6 FR-9/10) and of lazy verify-on-read
caching (§6 FR-11).

### 10.3 Consumer offsets & dedup

```sql
CREATE TABLE consumer_offsets (
    topic      TEXT PRIMARY KEY,         -- mxi.<entity>.events
    offset_val BIGINT NOT NULL,          -- last committed bus offset
    last_occurred_at TIMESTAMPTZ NOT NULL -- freshness watermark source (§6 FR-16/17)
);

CREATE TABLE processed_events (
    event_id   UUID PRIMARY KEY,         -- envelope event_id; idempotency key
    processed_at TIMESTAMPTZ NOT NULL
);
```

`processed_events` enforces idempotency under at-least-once delivery
(§6 FR-2); a periodic retention worker trims old rows. `consumer_offsets`
holds per-topic position and the freshness watermark backing `as_of`.

### 10.4 Governance / audit (for `case ↔ person`)

```sql
CREATE TABLE audit_log (                  -- reads & state changes touching subject_of/about
    id          UUID PRIMARY KEY,
    actor       TEXT,                      -- bearer sub, if any
    action      TEXT NOT NULL,             -- read_edge | read_single_view | apply_linked | …
    edge_kind   TEXT,
    from_ref    TEXT, to_ref TEXT,
    occurred_at TIMESTAMPTZ NOT NULL,
    user_ip     TEXT, user_agent TEXT
);
```

Inherits the case service's audit posture (§6 FR-19, §12).

### 10.5 SeaORM time-type feature

Follow the prevailing family convention at scaffold time. The shared
stack note ([rust-loco-stack.md](../../../agents/share/rust-loco-stack.md))
prescribes `with-chrono` as the loco-service default; this new crate
SHOULD adopt `with-chrono` from the start rather than inheriting the
older `with-time` carry-over that the first-converted services are
still reconciling (course-service spec §13 T-17). Recorded as OQ-5 (§16)
in case a sibling-uniformity constraint forces otherwise.

### 10.6 What is NOT persisted here

- No `entity_links` (that is the per-service **write-side** table,
  design §4.1 — authoritative, lives in each entity service; this
  service only diffs against it during reconciliation, §6 FR-21).
- No within-entity `relationships` (those stay on each domain model;
  the partition rule, design §7).
