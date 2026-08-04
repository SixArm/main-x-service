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
(§6 FR-2); a periodic retention worker trims old rows
(`LINK_GRAPH_PROCESSED_EVENTS_RETENTION_DAYS`, default 7). `consumer_offsets`
holds per-topic position and the freshness watermark backing `as_of`.

**Resume position (BUS-2, §13 T-6): delegated to Fluvio, not this
table.** The real consumer (`src/consumer.rs`) resumes each topic via
Fluvio's own **named-consumer offset management**
(`offset_consumer("link-graph-<topic>")` +
`OffsetManagementStrategy::Auto`), which the SC persists and resumes
server-side across restarts — the idiomatic mechanism for exactly what
this row's "per-topic offset resume" originally asked for, rather than
this crate reconstructing it against `offset_val`.
`Offset::beginning()` is only the fallback for a brand-new named
consumer that has never committed a position.

This leaves `offset_val` exactly what `apply_event` has always written
to it: the envelope's own per-`entity_pid` `seq` — read now as a
freshness/diagnostic value ("last committed **event**"), not a literal
Fluvio partition byte offset. Threading a real Fluvio-record offset
through `apply_event` instead would have touched roughly two dozen
existing test call sites for no behavioural gain, since resume does not
depend on it. Idempotency (`processed_events`) is a second, independent
layer required regardless of which mechanism resumes a topic, since
delivery is at-least-once either way.

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
- No `review_queue` — a `matcher_suggested` candidate's review/promotion
  state lives entirely in **person's** own `review_queue` table (§6.8,
  §10.7 below). This service is not where a suggestion is decided.

### 10.7 `suggestion_runs` — cross-service suggestion job audit (LNK-4)

```sql
CREATE TABLE suggestion_runs (
    id                  UUID PRIMARY KEY,
    started_at          TIMESTAMPTZ NOT NULL,
    completed_at        TIMESTAMPTZ NOT NULL,
    persons_fetched     BIGINT NOT NULL,
    workers_fetched     BIGINT NOT NULL,
    candidates          BIGINT NOT NULL,
    posted              BIGINT NOT NULL,
    failed              BIGINT NOT NULL,
    dropped             BIGINT NOT NULL,
    max_candidates      BIGINT NOT NULL,
    max_edges_per_run   BIGINT NOT NULL
);
CREATE INDEX suggestion_runs_completed_at ON suggestion_runs (completed_at);
```

Migration `m20260804_000001_suggestion_runs` (T-33); model
`src/models/suggestion_runs.rs`, `Model::record`. One row per
**completed** suggestion pass (§6 FR-28) — a pass that fails at the
fetch step records nothing, matching `run_periodic`'s existing
log-and-retry posture for that case. This is a **history**, not a
last-value slot (contrast the reconciliation worker, §10.6, whose
summary lives only in a live Prometheus gauge — sufficient there
because only the *current* divergence matters; insufficient here,
since OQ-9(d) asks the suggestion job's summary to survive a missed
scrape or a restart). A `link_graph_suggestion_last_run` gauge vec
(labelled `stat`) mirrors the latest row's counts for live/alertable
visibility on top of this durable history. Distinct from `audit_log`
(§10.4): `audit_log` records governed-edge **access** (who read/wrote
a `subject_of`/`about` edge); `suggestion_runs` records **job
run** counts, unrelated to `case ↔ person` governance.
