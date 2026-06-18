## 16. Open Questions

These inherit the design doc's open questions
([cross-service-linking.md §12](../../../agents/share/cross-service-linking.md#12-open-questions))
and add service-specific ones. Each carries a current lean; resolution
opens a §13 task.

- **OQ-1 — Traversal depth cap.** Cap `neighbors` / `single-view` at
  depth 1–2 in v1, or expose an arbitrary-depth recursive CTE?
  *(Lean: cap at 2; revisit with real query patterns — design §12.)*
  Resolution fixes the `depth` validation in §6 FR-13 / §9.2.

- **OQ-2 — Symmetric-write ownership for `same_identity`.** May either
  side assert (person *or* worker), with this aggregator canonicalising
  and deduping on the ordered pair? *(Lean: yes — both emit; dedupe on
  canonical pair, §6 FR-6.)* Affects how many duplicate `linked` events
  the projector must absorb.

- **OQ-3 — Reconciliation input: bulk-read vs replay.** Diff against a
  per-service `GET /links?since=` bulk endpoint, or a topic replay from
  offset 0? *(Lean: replay once the bus is durable; a bulk endpoint in
  the interim — design §12.)* Affects §6 FR-21 / §13 T-20.

- **OQ-4 — Shared `mxi-links` crate vs copy-per-project.** Package the
  `EntityRef` + registry contracts, or copy them here (and into each
  participating service)? *(Lean: copy until a second non-aggregator
  consumer exists — same call as `mxi-events`, design §12.)* Affects
  §13 T-2 / T-3.

- **OQ-5 — SeaORM time-type feature.** Adopt `with-chrono` from the
  start (the [rust-loco-stack.md](../../../agents/share/rust-loco-stack.md)
  default), or match whatever the first-converted siblings are running
  to avoid drift while course-service §13 T-17 is open? *(Lean:
  `with-chrono` — this is a brand-new crate with no carry-over to
  preserve.)* See §10.5.

- **OQ-6 — `dangling`-edge retention.** Keep `dangling` edges
  indefinitely (surfaced as a data-quality signal), tombstone them after
  a grace period, or hard-delete? *(Lean: keep + surface; let
  reconciliation / an operator decide — they are the failure mode a
  write-time check could never catch, design §5.)*

- **OQ-7 — `case ↔ person` concealment semantics.** When an
  unauthorised caller queries a ref that has only governed edges, return
  an empty `200`, a `403`, or a `404`? *(Lean: indistinguishable from
  "no such edges" so existence is not leaked — §9.2 / §12.2.)* Must be
  pinned before T-16.

- **OQ-8 — Full snapshot vs reference in consumed `data`.** The bus
  envelope's `data` may be a full snapshot or a reference for large
  records (event-bus.md §11). This service needs only the `EntityRef`s
  and edge fields — confirm it can ignore large `data` bodies and
  subscribe to a leaner projection if one is offered. *(Lean: consume
  only the fields it needs; tolerate either shape.)*
