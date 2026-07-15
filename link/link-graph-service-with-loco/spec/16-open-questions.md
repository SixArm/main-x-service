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

- **OQ-9 — Cross-service `same_identity` matcher + review queue (LNK-4).**
  The federation backbone (`same_identity`, person ↔ worker) is hand-authored
  today (LNK-2/3). LNK-4 **suggests** these links by comparing records,
  emitting `matcher_suggested` edges (confidence `< 1.0`) that an operator
  confirms — promoting to `operator` / `1.0` — per
  [cross-service-linking.md §5.2](../../../agents/share/cross-service-linking.md).
  The partition rule (§7 there) permits a **cross-service** matcher to
  *produce* such edges but never lets any *within-entity* matcher *consume*
  them. This is the spec round that must land before coding (LNK-4 §13 chain
  T-29–T-33). Resolved design decisions (with the genuinely-open parts
  flagged):

  - **Comparable projection.** `Person` and `Worker` are different types but
    share an identity core — name (family + given), `birth_date`, `gender`,
    and national identifiers (NHS / SSN / …). The job maps both sides to a
    lean `IdentityProbe { name, birth_date, gender, identifiers }` and scores
    with the matcher crates' shared primitives (Jaro-Winkler name, DOB
    proximity, gender, deterministic identifier short-circuit). *Lean: a thin
    cross-type comparator reusing the primitives; never feed cross-service
    edges into either within-entity matcher.*
  - **Candidate blocking.** All-pairs person × worker is O(n·m). Block on a
    cheap key — a shared national identifier (exact) or `Soundex(family)` +
    birth-year — to bound comparisons; exact-identifier candidates
    short-circuit to high confidence. *Lean: block; identifier-exact is the
    strongest block.*
  - **Where it runs.** A periodic job **hosted in the aggregator** — it
    already ingests both entities' events, holds the presence oracle, and has
    the reconcile-worker pattern — reading person / worker records via their
    read endpoints. *Lean: aggregator hosts the compute.*
  - **Who writes the edge.** The **originating service owns the write**
    (topology §4). The job writes `matcher_suggested` `same_identity` edges to
    **person's** `entity_links` via the existing
    `POST /api/persons/{id}/links` (which already accepts a `provenance`
    override + `confidence`), so a suggestion flows through the normal
    `linked`-event path into the graph as an `unverified` edge. The aggregator
    stays **read-only to the world** — it never gains a link-write endpoint;
    calling person's write API is person's write, not the aggregator's.
    *Lean: POST to person with `provenance=matcher_suggested`,
    `confidence=score`.*
  - **Review + promotion.** Suggested edges surface in a review surface — the
    same pattern as within-service duplicate review (§5.2). Confirm re-asserts
    the edge as `operator` / `1.0` (idempotent upsert keeps the `edge_id`);
    reject soft-deletes it (`unlinked`). *Lean: reuse the per-service
    review-queue pattern.*

  **Still open (pin before T-29):** (a) the exact block key + the
  auto-suggest vs discard threshold; (b) whether the review surface lives
  per-service or as a new aggregator read endpoint; (c) confirming the
  aggregator-calls-person's-write posture against "read-only to the world"
  (*Lean: acceptable — the invariant forbids a write endpoint **here**, not
  calling a peer*); (d) rate/scale controls for large registries.
