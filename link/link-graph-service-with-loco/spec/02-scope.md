## 2. Scope

### 2.1 In scope

- **Bus consumption.** Subscribe to every entity topic
  (`mxi.<entity>.events`,
  [event-bus.md §7](../../../agents/share/event-bus.md#7-topics-partitioning-config))
  and process the `created`, `deleted`, `merged`, `linked`, and
  `unlinked` event kinds. Idempotent on the envelope `event_id`
  ([event-bus.md §6](../../../agents/share/event-bus.md#6-delivery-semantics)).
- **Edge read-model.** Maintain the bidirectional `edges` table,
  indexed on both `from_ref` and `to_ref`, populated from `linked`
  events and pruned by `unlinked` events.
- **Symmetric canonicalisation.** `same_identity` (symmetric) is
  stored as **one** row with the lexicographically smaller ref as
  `from_ref`, regardless of which side asserted it.
- **Presence oracle.** Maintain `entity_presence`, fed by every
  entity's `created` / `deleted` events, as the source for the
  integrity lifecycle.
- **Integrity lifecycle.** Compute and surface edge `status` —
  `unverified | verified | dangling` — from `entity_presence`
  ([design §5](../../../agents/share/cross-service-linking.md#5-integrity--optimistic--async-verify)).
- **Lazy verify-on-read (interim).** Until the durable bus lands,
  resolve unknown endpoint presence with a one-shot `GET /{id}` to the
  source service and cache the verdict
  ([design §5.1](../../../agents/share/cross-service-linking.md#51-interim-before-the-durable-bus)).
- **Merge repointing.** Consume `merged{pid, merged_from}` and rewrite
  every edge referencing `merged_from` to `pid`
  ([design §5.3](../../../agents/share/cross-service-linking.md#53-merge-repointing-why-one-aggregator-helps)).
- **Read API.** `GET /api/v1/neighbors/{ref}`, `GET /api/v1/edges`,
  `GET /api/v1/single-view/{ref}`, `GET /api/v1/health/freshness` —
  every graph response carrying an `as_of` watermark.
- **Reconciliation worker.** Periodically diff the read-model against
  each service's authoritative `entity_links` and emit a divergence
  metric
  ([design §8](../../../agents/share/cross-service-linking.md#8-reconciliation-the-cost-of-two-sources-of-truth)).
- **Governance for `case ↔ person`.** Access control, audit, and
  privacy masking on the `subject_of` / `about` edge, matching the case
  service's posture
  ([design §10](../../../agents/share/cross-service-linking.md#10-governance--case--person)).
- **Observability.** Tracing + OpenTelemetry OTLP; Prometheus metrics
  including consumer lag and reconciliation divergence.
- PostgreSQL persistence via SeaORM, with migrations.

### 2.2 Out of scope (MVP)

- **Link writes.** Edge creation / withdrawal stays in the owning
  entity service (design §4.1); this service is read-only to the world.
- **Matching.** No matcher embeds here; cross-service edges are never a
  match signal (design §7). A future cross-service `same_identity`
  matcher is a *producer* of edges (roadmap §15), not part of this
  service.
- **Arbitrary-depth traversal.** v1 caps `neighbors` depth (OQ in §16);
  unbounded recursive CTE is deferred.
- **Suggestion-queue UI.** `provenance = matcher_suggested` review
  (design §5.2) is a roadmap item; v1 stores and exposes `provenance` /
  `confidence` but ships no review workflow.
- **gRPC API** (stub only).
- **FHIR mapping** (no FHIR resource models a cross-service link).
- **New edge kinds** beyond the v1 closed registry (design §9). Adding
  a kind is a future registry-row change, not MVP work.
