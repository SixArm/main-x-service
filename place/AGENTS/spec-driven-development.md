# Spec-Driven Development — Place entity

The place entity practises **spec-driven development** at two levels.
Read this before changing anything that crosses a subproject boundary.

## The authority model

| Question is about… | Source of truth |
|---|---|
| A subproject's internals (fields, weights, routes, components) | That subproject's own `spec/` |
| How the trio composes (front-end → service → matcher) | [`../spec/index.md`](../spec/index.md) (entity spec) |
| The service ↔ matcher DTO contract (adapter routing) | Entity [spec §5.3](../spec/05-domain-model.md) |
| Shared invariants (GLN check digit, coordinate bounds, soft-delete-only) | Entity [spec §5.5](../spec/05-domain-model.md) |
| Entity-wide goals (scale, locales, compliance, roadmap) | Entity spec §7, §12, §15 |

When the entity spec and a crate spec disagree about **crate
internals, the crate spec wins**; about the **integration contract,
the entity spec wins**. Either way: open a task (entity
[spec §13](../spec/13-tasks.md) or the crate's §13) — never silently
rewrite the loser.

## Three-part PRs

A behavioural change is one PR: **spec edit + code edit + test edit.**

- Change confined to one subproject → that subproject's spec + code +
  tests. Follow its own SDD guide:
  [service](../place-service-rust-crate/AGENTS/spec-driven-development.md) ·
  [matcher](../place-matcher-rust-crate/AGENTS/spec-driven-development.md) ·
  [front-end](../place-front-end-with-svelte/AGENTS/spec-driven-development.md).
- Change crossing a seam → edit the entity spec **and** each affected
  crate spec in the same PR, plus the seam's tests:
  - service ↔ matcher seam → a bridge test in
    [`tests/duplicate_detection.rs`](../place-service-rust-crate/tests/duplicate_detection.rs)
    (entity FR-19).
  - service ↔ front-end seam → front-end `src/lib/api/types.ts` +
    its unit tests change with the service field (entity FR-20).

## When to update which entity-spec section

| You're changing… | Update entity section… |
|---|---|
| Adapter routing rules (`src/matching/adapter.rs`) | §5.3 (+ bridge test) |
| A shared invariant | §5.5 |
| Which subproject owns a capability | §2.2, §6 |
| Integration requirements | §6.1 (FR-19…) |
| Scale / locale / security targets | §7 |
| Deployment topology, SSO wiring | §8 |
| Endpoint or route inventory (summary level) | §9 (detail lives in the crate docs) |
| Compliance scope | §12 |
| Adding cross-subproject work | §13 (new `E-N` entry) |
| Completing work | tick the box in §13; update §14 |
| Open-question resolution | move from §16 into the relevant section |

## Anti-patterns

- **Duplicating crate detail upward.** The entity spec summarises and
  links; exhaustive field tables, endpoint bodies, and weight tables
  live in the crate docs. If you're copying a table, stop and link.
- **Fixing drift by editing only one side.** Doc drift between
  subprojects (see entity §13 E-1…E-4 for live examples) is fixed by
  establishing ground truth in code, then correcting every doc that
  disagrees, in one PR.
- **Crate-internal tasks in the entity queue.** `E-N` tasks span
  subprojects or fix the contract; everything else goes in the owning
  crate's §13.
