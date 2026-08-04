## 1. Purpose and Vision

### 1.1 Purpose

The Link Graph Service is the **read-model aggregator** — the read
side — of the Main X Index family's cross-service entity linking. It
consumes every entity service's event stream and maintains one
queryable, bidirectional graph of **typed links between records that
live in different services**: "this person *is the same human as* that
worker", "this person *works at* that organization", "this case is
*about* that person".

It is the read half of the **hybrid topology** fixed in
[`cross-service-linking.md`](../../../agents/share/cross-service-linking.md):
each entity service owns its link **writes** locally (no service calls
another on the write path), and this standalone service consumes the
[durable event bus](../../../agents/share/event-bus.md) to build the
graph that can be traversed in one place.

The service is **read-only to the world.** It exposes no write
endpoints. Every change to its state arrives as a consumed bus event
(`created`, `deleted`, `merged`, `linked`, `unlinked`). The graph is a
**derived read-model**, not a system of record — each entity's
PostgreSQL remains authoritative for its own records and outbound
edges.

### 1.2 Vision

A single graph surface that:

- Answers **neighbours** in both directions for any
  [`EntityRef`](../../../agents/share/cross-service-linking.md#3-the-entityref--the-one-shared-contract)
  (`<entity_type>:<id>` URN) — index lookups on both edge endpoints,
  with a depth-capped recursive walk for multi-hop.
- Produces a **single view** of one real-world identity — the
  golden-record walk over `same_identity` plus affiliations
  (`person → worker → org` employer derivation).
- Makes **eventual consistency visible**: every graph response carries
  an `as_of` watermark, and `GET /health/freshness` reports per-entity
  consumer lag, so a UI can explain why a just-created link is not yet
  present.
- Treats referential integrity as a **lifecycle**
  (`unverified | verified | dangling`) driven by an existence oracle,
  not a write-time foreign key that cannot exist across services.
- **Repoints edges centrally** on a record merge — one handler
  consumes `merged{pid, merged_from}` and rewrites every affected edge,
  the payoff of aggregating the graph in one place.
- Runs a **reconciliation** worker that diffs the read-model against
  each service's authoritative `entity_links` and emits a divergence
  metric — the explicit, measured cost of two sources of truth.

### 1.3 Non-goals

- **Not** a write surface for links. Edge creation / withdrawal happens
  in the owning entity service (`POST/DELETE /<plural>/{pid}/links`,
  [design §4.1](../../../agents/share/cross-service-linking.md#41-write-side--entity_links-per-participating-service)).
  This service only consumes the resulting `linked` / `unlinked`
  events.
- **Not a within-entity matcher, and never a within-entity matcher
  signal.** Cross-service links are **never** fed to any within-entity
  matcher (the partition rule,
  [design §7](../../../agents/share/cross-service-linking.md#7-relationship-to-within-entity-matching-the-partition-rule));
  they are separate from each domain model's within-entity
  `relationships` field. This does **not** mean the service hosts no
  comparison logic at all: the LNK-4 cross-service `same_identity`
  suggestion job (`src/suggest/`) *does* compare person and worker
  records and score candidate pairs — but it produces suggestions
  (`provenance = matcher_suggested`, always `confidence < 1.0`, never
  auto-promoted) via person's own write API, never consumes an edge
  into a within-entity matcher, and is architecturally distinct from
  `person_matcher` / `worker_matcher`. See §5.5 and
  [`spec/16-open-questions.md`](16-open-questions.md) OQ-9.
- **Not** the system of record. Postgres-per-service stays
  authoritative; this is a derived, rebuildable projection of the event
  log.
- **Not** an authentication provider. JWT verification is consumed from
  the [authentication-service](../../../authentication/authentication-service-with-loco/);
  proofing is out of scope.
- **Not** a general graph database. The edge-kind registry is a
  **closed** v1 set
  ([design §9](../../../agents/share/cross-service-linking.md#9-v1-edge-kind-registry));
  arbitrary user-defined edges are out of scope.
