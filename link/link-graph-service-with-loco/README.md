# link-graph-service-with-loco

The **read-model aggregator** for cross-service entity linking in the
Main X Index family — the **read side** of the hybrid topology fixed in
[`cross-service-linking.md`](../../agents/share/cross-service-linking.md).
It consumes every entity service's event stream and maintains one
queryable, bidirectional graph of **typed links between records in
different services**: person *is the same human as* worker, person
*works at* organization, case *is about* person.

> **Status: read API + cross-service identity suggestion implemented.**
> The read-model core, the four read endpoints, the real Fluvio bus
> consumer (BUS-2), reconciliation, offline-PASETO auth, and the LNK-4
> cross-service `same_identity` suggestion job are all live
> ([`spec/`](spec/index.md), §1–§18; `cargo test --lib`: 95 passed,
> 2026-08-04); the remaining work (graph-read masking parity with the
> case service, OTLP wiring, the durable-bus flip, the bus/governance/
> bench test tiers) is tracked in
> [`spec/13-tasks.md`](spec/13-tasks.md).

## What it is

- A **loco.rs** backend service (Rust, Axum, SeaORM, PostgreSQL) on the
  family [Rust + Loco stack](../../agents/share/rust-loco-stack.md).
- **Read-only to the world.** It exposes no write endpoints of its own;
  every change to *its own* state arrives as a consumed bus event
  (`created` / `deleted` / `merged` / `linked` / `unlinked`). It does,
  however, act as an authenticated **client** of peer write/read APIs
  for two periodic background jobs — reconciliation (`GET`) and the
  LNK-4 identity-suggestion job (`POST` to person, below) — neither of
  which adds a route here ([`spec/16-open-questions.md`](spec/16-open-questions.md)
  OQ-9(c)).
- A **§9 consumer** of the
  [durable event bus](../../agents/share/event-bus.md), subscribing to
  every `mxi.<entity>.events` topic plus the new `linked` / `unlinked`
  edge events.
- The home of **merge repointing** (rewrite edges centrally on
  `merged`), the **integrity lifecycle** (`unverified | verified |
  dangling`), and **reconciliation** (diff the read-model against each
  service's authoritative `entity_links`, emit a divergence metric).

## Read API

| Endpoint | Purpose |
| --- | --- |
| `GET /api/neighbors/{ref}` | Edges incident to an `EntityRef`, both directions, depth-capped |
| `GET /api/edges` | Filtered edge list (`from` / `to` / `kind` / `status`) |
| `GET /api/single-view/{ref}` | Golden-record walk: `same_identity` unification + affiliations |
| `GET /api/health/freshness` | Per-entity consumer lag (eventual consistency, made visible) |

Every graph response carries an **`as_of`** watermark — the read-model's
freshness, so a UI can show "graph as of 10:42:05".

## Quick start

Requires PostgreSQL.

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/link_graph_service_development
cargo loco start        # migrations auto-run in development

# Edges incident to a record (both directions, depth-capped)
curl -s 'localhost:5160/api/neighbors/person:0c4f1e2a-0000-4000-8000-000000000000?direction=both&depth=1'

# Filtered edge list
curl -s 'localhost:5160/api/edges?kind=same_identity&status=verified'

# Per-entity consumer freshness (the `as_of` watermark source)
curl -s localhost:5160/api/health/freshness
```

## Key concepts

- **`EntityRef`** — the one shared contract: a `<entity_type>:<id>` URN
  (e.g. `person:0c4f…`), indexed on both edge endpoints. See
  [design §3](../../agents/share/cross-service-linking.md#3-the-entityref--the-one-shared-contract).
- **Edge-kind registry** — a **closed** v1 set: `same_identity`
  (person ↔ worker, symmetric), `works_at` / `member_of` (person → org),
  `employed_by` (worker → org), `subject_of` / `about` (case → person,
  high-governance). See
  [design §9](../../agents/share/cross-service-linking.md#9-v1-edge-kind-registry).
- **Partition rule** — cross-service links are **never** a matcher
  signal and are separate from within-entity `relationships`
  ([design §7](../../agents/share/cross-service-linking.md#7-relationship-to-within-entity-matching-the-partition-rule)).
- **Lazy verify-on-read** — interim integrity before the durable bus: a
  one-shot `GET /{id}` to the source service resolves unknown presence
  ([design §5.1](../../agents/share/cross-service-linking.md#51-interim-before-the-durable-bus)).

## Cross-service identity suggestion (LNK-4)

A periodic background job (`src/suggest/`, no HTTP endpoint of its own)
**suggests** `same_identity` (person ↔ worker) edges by comparison,
complementing the hand-asserted federation backbone:

1. Fetch every person and worker record via their database-backed
   `GET /<plural>?limit=&offset=` list endpoints.
2. Map each to a lean `IdentityProbe { name, birth_date, gender,
   identifiers }` and **block** candidate pairs (shared coded
   identifier, else `Soundex(family)` + birth-year) so scoring stays
   sub-quadratic.
3. **Score** same-block pairs (Jaro-Winkler name + DOB proximity +
   gender, or an identifier short-circuit) — confidence always
   `< 1.0`, never auto-promoted regardless of score.
4. **`POST`** surviving candidates (`>= 0.7`, capped by
   `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN`) to person's own
   `POST /api/persons/{id}/links` as `provenance = "matcher_suggested"`
   — person's `create_link` handler does the actual write, the
   `entity_links` upsert, the `linked` event emission, the audit row,
   and the review-queue insert. This service never writes an edge
   itself.

Suggested edges surface in **person's existing `review_queue`**
(`GET /api/persons/review-queue` /
`POST /api/persons/review-queue/{id}/decision`) — there is no separate
review UI or endpoint here. Confirming promotes the edge to
`provenance = "operator", confidence = 1.0`; rejecting soft-deletes it.

Every completed pass writes one durable row to this service's own
`suggestion_runs` table (fetch/candidate/posted/dropped counts, plus
the caps in force) — see [`spec/10-persistence.md`](spec/10-persistence.md)
§10.7. Configuration
(`LINK_GRAPH_SUGGEST_URL_PERSON` / `_URL_WORKER` / `_TOKEN` / `_SECS` /
`_MAX_CANDIDATES` / `_MAX_EDGES_PER_RUN`) is unset-by-default (the job
does not start without both URL vars) — see
[`agents/share/configuration.md`](../../agents/share/configuration.md)
for the full reference. Design decisions:
[`spec/16-open-questions.md`](spec/16-open-questions.md) OQ-9;
implementation: [`spec/13-tasks.md`](spec/13-tasks.md) T-29..T-33.

## Governance — `case ↔ person`

The `subject_of` / `about` edge is itself sensitive data. It carries the
case service's compliance posture: access control + audit + privacy
masking on every read path that could surface it (incl. `single-view`).
See [`spec/12-compliance.md`](spec/12-compliance.md) and
[design §10](../../agents/share/cross-service-linking.md#10-governance--case--person).

## Documentation

- [`spec/index.md`](spec/index.md) — single source of truth (§1–§18;
  live work queue in §13; open questions in §16).
- [`AGENTS.md`](AGENTS.md) — agent guide / ground rules.
- [`CHANGELOG.md`](CHANGELOG.md) — Keep-a-Changelog history.
- Design: [`cross-service-linking.md`](../../agents/share/cross-service-linking.md)
  + [`event-bus.md`](../../agents/share/event-bus.md).

## Sibling

Unlike the per-entity service crates, this is a **cross-cutting** service
— it has no single sibling matcher or front-end. It sits across all
entity services as the consumer of their combined event streams.
