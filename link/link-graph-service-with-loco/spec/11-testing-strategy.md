## 11. Testing Strategy

Tiered after the family convention: un-gated unit tests run DB-free in
`cargo test --lib`; DB-gated and bus-gated integration tests are
`#[ignore]`-tagged and run explicitly against provisioned backends.

### 11.1 Un-gated (DB-free)

- **`EntityRef`** — `parse` / `Display` round-trip; rejects unknown
  `entity_type`; the `entity_type → service` map resolves every v1
  type (incl. `courseinstance` distinct from `course`).
- **Edge-kind registry** — every v1 kind's endpoint-type pair,
  direction, inverse, temporality, and sensitivity match
  [design §9](../../../agents/share/cross-service-linking.md#9-v1-edge-kind-registry);
  unknown kinds rejected.
- **Symmetric canonicalisation** — `same_identity` from either side
  produces the same canonical `(from_ref, to_ref)` (lexicographic
  order, §6 FR-6).
- **Envelope decode** — the `linked` / `unlinked` event `data` shape
  (`edge_id`, `from_ref`, `to_ref`, `edge_kind`, `role`, `confidence`,
  `provenance`, `valid_from`, `valid_to`) deserialises; `schema_version`
  honoured ([event-bus.md §4](../../../agents/share/event-bus.md#4-event-envelope-canonical-versioned)).
- **Integrity lifecycle** — given presence inputs, `status` resolves to
  `verified` / `unverified` / `dangling` correctly, including the
  "deleted after edge formed ⇒ dangling" transition (§6 FR-9/10).
- **Merge repointing** — repointing `merged_from → pid` rewrites both
  `from_ref` and `to_ref`, re-canonicalises symmetric edges, and
  de-duplicates colliding edges (§6 FR-12).
- **Idempotency** — applying the same `event_id` twice is a no-op (§6
  FR-2).
- **`as_of` projection** — freshness watermark derives the `as_of`
  attached to graph responses.

### 11.2 DB-gated (`#[ignore]`)

- Consumer applies `linked` → `edges` row; `unlinked` → removal;
  `created` / `deleted` → `entity_presence` flips; status recompute on
  incident edges.
- `neighbors` both-direction lookups via `edges_from` / `edges_to`;
  `edges` filters (`from` / `to` / `kind` / `status`); depth-cap
  enforcement.
- `single-view` golden-record walk (`same_identity` unification +
  `person → worker → org` employer derivation).
- `processed_events` dedup under simulated redelivery; `consumer_offsets`
  advance + freshness watermark.
- Reconciliation worker: seed a divergence (drop an edge), run, assert
  the divergence metric reflects it and the read-model is repaired (§6
  FR-21).

### 11.3 Bus-gated (feature `fluvio` + broker/test container)

- Relay → topic → this consumer round-trip for `linked` / `unlinked` /
  `merged`; partition-key ordering per `pid`; at-least-once redelivery
  deduped on `event_id`
  ([event-bus.md §10](../../../agents/share/event-bus.md#10-testing-strategy)).
- Replay-from-offset-0 reconstructs an empty read-model to the same
  state as incremental application (§7 NFR-4 rebuildability).

### 11.4 Governance tests (`case ↔ person`)

- A caller lacking case-read authorisation gets **no leak** of a
  `subject_of` / `about` edge via `/edges`, `/neighbors`, or
  `/single-view` (§6 FR-18/20).
- Every read/write of such an edge writes an `audit_log` row (§6 FR-19).

### 11.5 Interim verify-on-read

- With transport `memory`, an unknown-presence endpoint triggers a
  one-shot source-service `GET /{id}` (mocked), and the verdict is
  cached in `entity_presence` (§6 FR-11).

### 11.6 Benchmarks (Criterion, where useful)

- `neighbors` (depth 1) and `edges` filter latency on a representative
  graph; `single-view` multi-hop at the depth cap.
