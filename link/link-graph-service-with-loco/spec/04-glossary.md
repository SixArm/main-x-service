## 4. Glossary

| Term | Meaning |
|---|---|
| **`EntityRef`** | The one shared contract: an opaque URN `<entity_type>:<id>` naming a record in any service (e.g. `person:0c4f…`). `entity_type` is globally unique across the family; a static `entity_type → service` lookup resolves the owning service. `id` is the record's public UUID (`pid`). See [design §3](../../../agents/share/cross-service-linking.md#3-the-entityref--the-one-shared-contract). |
| **Edge** | A typed, directed-or-symmetric link between two `EntityRef`s, with kind, provenance, confidence, optional validity dates, and a lifecycle `status`. |
| **Edge kind** | A member of the closed v1 registry — `same_identity`, `works_at`, `member_of`, `employed_by`, `subject_of` / `about` ([design §9](../../../agents/share/cross-service-linking.md#9-v1-edge-kind-registry)). Each fixes endpoint types, direction, inverse, temporality, and sensitivity. |
| **Read-model aggregator** | This service — the read side of the hybrid topology; consumes the bus and serves the queryable graph. |
| **`entity_links`** | The per-service **write-side** table (in each entity service, not here) holding that service's outbound edges. The authoritative source against which §8 reconciliation diffs. |
| **`edges`** | This service's derived, bidirectional read-model table (one row per edge, both directions reachable by index). |
| **`entity_presence`** | The existence oracle table: `ref → alive`, fed by `created` / `deleted` events; drives the integrity lifecycle. |
| **`linked` / `unlinked`** | The two new event kinds on the [shared envelope](../../../agents/share/event-bus.md#4-event-envelope-canonical-versioned); carry the edge detail in `data`. See [design §4.2](../../../agents/share/cross-service-linking.md#42-the-linked--unlinked-events). |
| **`status`** | An edge's integrity lifecycle value: `unverified` (an endpoint not yet seen), `verified` (both endpoints alive), `dangling` (an endpoint was deleted after the edge formed). |
| **Lazy verify-on-read** | Interim integrity strategy before the durable bus: resolve unknown presence with a one-shot `GET /{id}` to the source service, caching the verdict ([design §5.1](../../../agents/share/cross-service-linking.md#51-interim-before-the-durable-bus)). |
| **Merge repointing** | Centrally rewriting every edge referencing `merged_from` to the surviving `pid` on a `merged` event ([design §5.3](../../../agents/share/cross-service-linking.md#53-merge-repointing-why-one-aggregator-helps)). |
| **`as_of`** | The read-model's freshness watermark, returned on every graph response so eventual consistency is visible (§6 of the design / §7 here). |
| **Single view** | The golden-record walk over `same_identity` + affiliations for one real-world identity (`GET /single-view/{ref}`). |
| **Reconciliation** | The periodic diff of read-model `edges` against each service's authoritative `entity_links`, emitting a divergence metric ([design §8](../../../agents/share/cross-service-linking.md#8-reconciliation-the-cost-of-two-sources-of-truth)). |
| **Divergence** | The count of edges present in one store and not the other; an SLO (steady-state ≈ 0). |
| **Partition rule** | The load-bearing invariant: cross-service links are **never** a matcher signal and live **only** here + `entity_links` + the events, never in within-entity `relationships` ([design §7](../../../agents/share/cross-service-linking.md#7-relationship-to-within-entity-matching-the-partition-rule)). |
| **Durable event bus** | The outbox → Fluvio transport this service consumes ([event-bus.md](../../../agents/share/event-bus.md)). Interim transport is in-memory + lazy verify-on-read. |
