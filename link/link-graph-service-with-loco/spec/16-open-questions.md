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

  **Pinned 2026-08-04 (closes the T-29 gate).** Each decision below is
  grounded in a live precedent already in the repo, not invented fresh:

  - **(a) Block key + auto-suggest/discard threshold.** Block key: an
    **exact match on a shared coded national identifier** (an
    `Identifier` system+value — NHS/SSN/other — present on both the
    person and the worker side) when one exists, mirroring the matcher
    family's existing deterministic identifier short-circuit
    (`person/person-service-with-loco/AGENTS/matching.md` Rule 0/Rule 1:
    tax-ID / identifier exact match ⇒ short-circuit). Records sharing no
    identifier are blocked instead on **`Soundex(family)` + birth-year**
    (the same phonetic primitive the matcher's name component already
    uses). Only same-block pairs are ever scored — see (d) for why this
    is the load-bearing sub-quadratic bound. Threshold: reuse the
    family's existing **0.7** review-worthy line rather than invent a
    new number — it is both
    `BatchDeduplicationRequest::threshold`'s default
    (`person/person-service-with-loco/src/models/review_queue.rs`) and
    `IMPORT_REVIEW_THRESHOLD`
    (`person/person-service-with-loco/src/bulk/mod.rs`, EX-1/TUT-5). A
    score `< 0.7` is discarded — never stored, never surfaced, so a run
    over a large registry doesn't silently grow the review queue with
    noise. A score `>= 0.7` is written as a `matcher_suggested`
    suggestion at that confidence. Unlike within-entity dedup there is
    **no auto-merge tier**: even an exact-identifier block hit still
    lands in the queue for an operator to confirm rather than
    auto-promoting, because a cross-service identity assertion is a
    higher-stakes claim than a within-entity duplicate — the family's
    `auto_merge_threshold` (0.95) precedent is deliberately *not*
    carried over here (see T-33).
  - **(b) Review-surface home.** Per-service — **person's existing
    `review_queue` table and endpoints**, not a new aggregator
    endpoint. This is structurally sound, not just convenient:
    `record_id_a`/`record_id_b`
    (`person/person-service-with-loco/migrations/2026071900000001_create_review_queue/up.sql`)
    carry **no foreign-key constraint**, so a cross-service row
    (`record_id_a` = person pid, `record_id_b` = worker pid) stores
    cleanly, and the `provenance` column BLK-2 added
    (`m20260802_000001_review_queue_provenance`, 2026-08-02) already
    carries the exact `matcher_suggested` token this job needs — zero
    schema change. `detection_method` (free-text) is set to
    `cross_service_same_identity` so a row is self-describing and
    filterable. The one real gap — the table has no worker-side summary
    for a side-by-side comparison — is resolved by having the reviewing
    client resolve the worker half with its own direct
    `GET /api/workers/{id}` call, the same drift-accepted
    per-front-end pattern the family already uses instead of a shared
    package (`feedback_front_end_drift` memory). T-32 extends
    `review_decision`'s `confirmed` branch to also call
    `entity_links::upsert` with `provenance="operator",
    confidence=1.0` (idempotent on the existing upsert key, so it
    reasserts the same `edge_id`) and its `rejected` branch to
    soft-delete the edge (emitting `unlinked`) — "promotion" *is* the
    existing person link-write path (design §4.1), triggered from the
    existing decision endpoint, not a new one.

    **Landed 2026-08-04, one refinement over the paragraph above:**
    the promotion path calls person's own `upsert_and_emit` wrapper
    (the same one `create_link` itself uses), not a bare
    `entity_links::upsert`, so a confirmed promotion also emits the
    normal `linked` event under the active transport (durably, under
    `outbox`) — a bare `upsert` call would have written the row but
    silently skipped that event. Rejection likewise goes through
    `soft_delete_and_emit`, keyed by a new `entity_links::find_active_by_key`
    natural-key lookup since a review-queue row carries no `edge_id`.
    Also landed: the write side of the *suggestion* (not just the
    promotion) — `create_link` itself now queues the review-queue row
    for a `matcher_suggested` `same_identity` edge, via a
    **non-reordering** `review_queue::upsert_cross_service` rather than
    the existing order-normalizing `upsert` (see this crate's own
    `spec/13-tasks.md` T-32 entry for why that distinction is
    load-bearing). Full detail in `person-service-with-loco`'s
    `CHANGELOG.md` and its own `spec/13-tasks.md` T-32 entry, since that
    is where all of this actually lives. The aggregator's own
    `GET /api/edges?...&status=unverified` stays available as a
    **read-only discovery convenience** (it will already project
    `matcher_suggested` edges once T-31 posts them) but is never where a
    decision is made.
  - **(c) Aggregator-calls-person's-write posture: confirmed
    acceptable.** "Read-only to the world" (this crate's `AGENTS.md`)
    forbids the aggregator from ever exposing **its own** write
    endpoint to external callers; it says nothing about the aggregator
    acting as an authenticated **client** of a peer's write API — which
    is exactly what the reconcile worker already does today in reverse
    (`GET` against a peer, `src/reconcile.rs`
    `HttpAuthoritativeSource::fetch_all`). LNK-4's job is the same
    shape with the verb flipped to `POST`. Auth follows the reconcile
    worker's SEC-B7 template exactly (`source_auth_ok`/
    `is_loopback_url`: a loopback URL may be token-less, any remote
    host requires a bearer token) but under **its own** env vars rather
    than reusing the reconcile ones: `LINK_GRAPH_SUGGEST_URL_PERSON`
    (the target, mirroring `LINK_GRAPH_RECONCILE_URL_<ENTITY>`) and
    `LINK_GRAPH_SUGGEST_TOKEN` (the bearer). A dedicated token rather
    than `LINK_GRAPH_RECONCILE_TOKEN`, because the two credentials have
    different blast radii — reconcile only *reads* a peer's outbound
    edges, suggest can *write* a `same_identity` edge into person's
    graph — and `agents/share/security.md` invariant 10
    ("least-authority artifacts") argues for the narrower-scoped
    credential being independently revocable.
  - **(d) Rate/scale controls.** A dedicated interval,
    `LINK_GRAPH_SUGGEST_SECS` (default 3600 — an hour, coarser than
    reconcile's 300s default because this job does real O(pairs)
    scoring work rather than a cheap diff), following
    `LINK_GRAPH_RECONCILE_SECS`'s skip-first-tick pattern
    (`src/reconcile.rs::run_periodic`). Two caps, mirroring
    `BatchDeduplicationRequest`'s existing `max_candidates` (default
    50) and the bulk subsystem's per-run row caps (SEC-B2,
    `agents/share/bulk-import-export.md` §12):
    `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` (default 50) bounds how many
    same-block comparisons run per anchor record, and
    `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN` (default 200) caps how many
    suggestions a single run may POST, so one pathological block cannot
    flood person's review queue or audit log in one pass. The scale
    claim this rests on: **blocking (a) is what keeps the job
    sub-quadratic** — without it, comparing every person against every
    worker is O(n·m), exactly the DoS shape
    `agents/share/security.md` §3 invariant 3 ("bound every input…
    unbounded fan-out into O(n·m) scoring is a DoS") warns against;
    with blocking, cost is O(n + m + Σ|block|²) over blocks that stay
    small in practice (a shared identifier, or a `Soundex(family)` +
    birth-year cohort) — the same reasoning that already justifies the
    family's Tantivy-index-blocked duplicate-check on
    org/care-pathway/case/portfolio (`agents/share/overview.md`
    capability-matrix footnote 1).

  **OQ-9 fully resolved 2026-08-04 (T-33 landed, closing LNK-4).** Both
  (d) caps are implemented exactly as pinned:
  `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` bounds `generate_candidates`'s
  per-anchor same-block comparisons (a thin default-`50` wrapper over
  `generate_candidates_bounded`, which takes the cap explicitly), and
  `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN` bounds how many of
  `run_suggestion_pass`'s scored candidates actually get `POSTed` (the
  highest-confidence survivors, ties broken deterministically on the
  `(person, worker)` id pair). (a)'s "no auto-merge tier for
  cross-service identity" is now live-verified, not just asserted: a
  real person + a synthetic worker sharing a coded identifier score at
  `IDENTIFIER_MATCH_CEILING` (`0.99`, above the family's own
  within-entity `auto_merge_threshold` of `0.95`) and the resulting
  review-queue row still lands and stays `pending`
  (`tests/live_suggest_never_promoted.rs`). "The suggestion job audits
  every POST it makes" turned out to already be true of T-31/T-32's
  existing infrastructure — person's `create_link` unconditionally
  audits every link creation regardless of provenance — so T-33 added a
  regression test proving that rather than a second, redundant audit
  mechanism; "audits every run's counts" is new (`suggestion_runs`
  table, one durable row per completed pass). See `spec/13-tasks.md`
  T-33 for the full landing account.
