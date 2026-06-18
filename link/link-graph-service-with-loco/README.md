# link-graph-service-with-loco

The **read-model aggregator** for cross-service entity linking in the
Main X Index family — the **read side** of the hybrid topology fixed in
[`cross-service-linking.md`](../../agents/share/cross-service-linking.md).
It consumes every entity service's event stream and maintains one
queryable, bidirectional graph of **typed links between records in
different services**: person *is the same human as* worker, person
*works at* organization, case *is about* person.

> **Status: spec-only; no code yet.** This crate exists as a
> specification ([`spec/`](spec/index.md), §1–§18). The build-out is
> enumerated as unchecked tasks in [`spec/13-tasks.md`](spec/13-tasks.md).

## What it is

- A **loco.rs** backend service (Rust, Axum, SeaORM, PostgreSQL) on the
  family [Rust + Loco stack](../../agents/share/rust-loco-stack.md).
- **Read-only to the world.** It exposes no write endpoints; every
  change to its state arrives as a consumed bus event
  (`created` / `deleted` / `merged` / `linked` / `unlinked`).
- A **§9 consumer** of the
  [durable event bus](../../agents/share/event-bus.md), subscribing to
  every `mxi.<entity>.events` topic plus the new `linked` / `unlinked`
  edge events.
- The home of **merge repointing** (rewrite edges centrally on
  `merged`), the **integrity lifecycle** (`unverified | verified |
  dangling`), and **reconciliation** (diff the read-model against each
  service's authoritative `entity_links`, emit a divergence metric).

## Read API (planned)

| Endpoint | Purpose |
| --- | --- |
| `GET /api/v1/neighbors/{ref}` | Edges incident to an `EntityRef`, both directions, depth-capped |
| `GET /api/v1/edges` | Filtered edge list (`from` / `to` / `kind` / `status`) |
| `GET /api/v1/single-view/{ref}` | Golden-record walk: `same_identity` unification + affiliations |
| `GET /api/v1/health/freshness` | Per-entity consumer lag (eventual consistency, made visible) |

Every graph response carries an **`as_of`** watermark — the read-model's
freshness, so a UI can show "graph as of 10:42:05".

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
