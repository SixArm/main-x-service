## 6. Functional Requirements

Requirements are grouped by the service's responsibilities. Each is
realised against the contracts in
[`cross-service-linking.md`](../../../agents/share/cross-service-linking.md)
and [`event-bus.md`](../../../agents/share/event-bus.md).

### 6.1 Bus consumption

- **FR-1** — Subscribe to every entity topic `mxi.<entity>.events` and
  process the `created`, `deleted`, `merged`, `linked`, `unlinked`
  event kinds; ignore other kinds (`updated`) for graph state, but
  advance the freshness watermark for every consumed event.
- **FR-2** — Consumers MUST be **idempotent**, deduping on the envelope
  `event_id`. Re-delivery (at-least-once) MUST NOT double-apply an edge
  or a presence flip.
- **FR-3** — Track per-topic offsets; a fresh or rebuilding consumer
  MUST be able to replay a topic from offset 0 to reconstruct the
  read-model (durable-bus path) — the read-model is fully derivable.

### 6.2 Edge read-model

- **FR-4** — On `linked`, upsert one `edges` row keyed by `edge_id`
  (idempotent), populating `from_ref` / `to_ref` / `kind` / `directed`
  / `role` / `confidence` / `provenance` / `valid_from` / `valid_to` /
  `observed_at` / `source_event_id`.
- **FR-5** — On `unlinked`, remove (or tombstone) the edge identified by
  `edge_id`.
- **FR-6** — **Symmetric** kinds (`same_identity`) MUST be canonicalised
  to one row with the lexicographically smaller ref as `from_ref`, so
  the pair is stored once regardless of which side emitted the event.
- **FR-7** — Neighbour lookups MUST work in **both** directions via the
  `edges_from` (`from_ref`) and `edges_to` (`to_ref`) indexes.

### 6.3 Presence oracle & integrity lifecycle

- **FR-8** — On `created`, set `entity_presence(ref).alive = true`; on
  `deleted`, set `alive = false`. Record `last_seq` for ordering.
- **FR-9** — Derive each edge's `status` from the presence of both
  endpoints: both alive ⇒ `verified`; an endpoint not yet seen ⇒
  `unverified`; an endpoint known-deleted ⇒ `dangling`.
- **FR-10** — A target deleted **after** the edge formed MUST flip the
  edge to `dangling` (surfaced, never silently broken).
- **FR-11** (interim) — When presence is unknown and the durable bus is
  not yet live for that entity, **lazy verify-on-read**: a one-shot
  `GET /{id}` to the source service resolves presence, cached in
  `entity_presence`. The event-driven path supersedes this per entity
  as topics go durable.

### 6.4 Merge repointing

- **FR-12** — On `merged{pid, merged_from}`, **repoint** every edge
  referencing `merged_from` (as `from_ref` or `to_ref`) to `pid`, in
  one handler. Re-canonicalise symmetric edges after repointing and
  de-duplicate any edge that collides with an existing one.

### 6.5 Read API

- **FR-13** — `GET /api/v1/neighbors/{ref}` returns edges incident to
  `ref`, filterable by `kind`, `direction` (`out` | `in` | `both`), and
  `depth` (capped, §16). Every response includes `as_of`.
- **FR-14** — `GET /api/v1/edges` returns edges filtered by any of
  `from`, `to`, `kind`, `status`. Every response includes `as_of`.
- **FR-15** — `GET /api/v1/single-view/{ref}` returns the golden-record
  walk: follow `same_identity` to unify person ↔ worker, then collect
  affiliations (`employed_by` / `works_at` / `member_of`) for the
  unified identity. Includes `as_of`.
- **FR-16** — `GET /api/v1/health/freshness` returns, per entity topic,
  the `occurred_at` of the last consumed event and the lag versus now.
- **FR-17** — Every graph response (FR-13/14/15) carries an `as_of`
  watermark = the read-model's freshness watermark, so callers can
  display "graph as of …" and explain a not-yet-present link.

### 6.6 Governance (`case ↔ person`)

- **FR-18** — Reading any `subject_of` / `about` edge (directly via
  `/edges`/`/neighbors`, or transitively via `/single-view`) MUST
  enforce **access control** at least equal to reading the underlying
  case. An unauthorised caller MUST NOT learn the edge exists.
- **FR-19** — Every read/write touching a `subject_of` / `about` edge
  MUST be **audited** (who / what / when), consistent with the case
  service's audit trail.
- **FR-20** — `single-view` / `neighbors` responses MUST honour the
  same **privacy masking / authorisation** as the case service for
  these edges.

### 6.7 Reconciliation & observability

- **FR-21** — A periodic worker MUST pull each service's authoritative
  `entity_links` (bulk-read endpoint, or topic replay from offset 0),
  diff against the read-model `edges`, **emit a divergence metric**
  (count present in one and not the other), and repair the read-model.
- **FR-22** — Expose Prometheus metrics: per-entity consumer lag, edge
  counts by `status`, reconciliation divergence, and `linked` /
  `unlinked` / `merged` processed counters.
