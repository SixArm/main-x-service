# Spec-Driven Development — Worker entity

The Worker entity practises the same SDD discipline as every Main X
Index subproject, with one extra layer: an **entity-level spec**
([`../spec/index.md`](../spec/index.md)) above the three per-subproject
specs.

## Authority model

| Question is about… | Source of truth |
|---|---|
| Service crate internals (handlers, schema, in-service matcher) | [service spec](../worker-service-with-loco/spec/index.md) (§1–§18) |
| Matcher crate internals (algorithms, weights, normalisation, API) | [matcher spec](../worker-matcher-rust-crate/spec/index.md) (§1–§25) |
| Front-end internals (routes, components, build) | [front-end spec](../worker-front-end-with-svelte/spec/index.md) (§1–§18) |
| **The integration contract** — trio composition, service↔matcher DTO (adapter routing), REST envelope the front-end relies on, shared invariants, entity-wide goals | [entity spec](../spec/index.md) (§1–§18) |

When the entity spec and a crate spec disagree about **crate
internals**, the crate spec wins. When they disagree about the
**integration contract**, the entity spec wins. Either way: open a
task (entity [§13](../spec/13-tasks.md) or the crate's task section)
to bring the loser in line — never silently rewrite the winner.

## Three-part PRs

A behavioural change is one PR: **spec edit + code edit + test
edit**. At the entity level this often spans subprojects — e.g. a new
adapter routing rule touches:

1. entity spec §5.3 (the contract),
2. `worker-service-with-loco/src/matching/adapter.rs` (the code),
3. `worker-service-with-loco/tests/duplicate_detection.rs` (the
   seam test).

## When to update which entity-spec section

| You're changing… | Update entity spec… |
|---|---|
| What a subproject owns | §2 |
| Service or matcher `Worker` shape *as seen across the seam* | §5.1 / §5.2 |
| Adapter routing rules, dropped fields, scheme-locality | §5.3 (+ bridge test) |
| Wire envelope, endpoint inventory, front-end routes | §9 |
| Who persists what | §10 |
| Seam-test obligations | §11 |
| Compliance posture, data-subject rights mapping | §12 |
| Adding cross-subproject work | §13 (new `T-N`) |
| A capability landing / regressing | §14 |
| Scale / deployment ambitions | §15 |
| Unresolved cross-subproject decisions | §16 |

Crate-internal changes update the crate's own spec per its
`agents/spec-driven-development.md`
([service](../worker-service-with-loco/agents/spec-driven-development.md),
[matcher](../worker-matcher-rust-crate/agents/spec-driven-development.md)).

## Anti-patterns

- Duplicating a crate's field tables / endpoint tables into the
  entity spec "for convenience" — link down instead; copies drift.
- Fixing a contract break by editing only the entity spec (or only
  the crate) — the seam test must move in the same PR.
- Claiming roadmap scale (§15) as delivered status (§14).
- Parking crate-internal tasks in the entity §13 — they belong in the
  owning crate's queue.

## Document hierarchy

```
worker/
├── spec/              ← entity-level living spec (cross-subproject contract)
├── agents/            ← this directory: entity-level orientation docs
├── worker-service-with-loco/      ← own spec/ + agents/ (authoritative for itself)
├── worker-matcher-rust-crate/      ← own spec/ + agents/ (authoritative for itself)
└── worker-front-end-with-svelte/   ← own spec/ + AGENTS.md (authoritative for itself)
```

There is intentionally no `plan.md` and no `tasks.md` at any level:
plan content lives in spec §8–§12, tasks in §13, status / roadmap in
§14–§15, open questions in §16.
