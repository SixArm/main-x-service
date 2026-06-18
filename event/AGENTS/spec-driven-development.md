# Spec-driven development — Event Entity

How the SDD discipline works **across** the trio. For the per-crate
discipline, read the owner's guide:
[service](../event-service-with-loco/AGENTS/spec-driven-development.md) ·
[matcher](../event-matcher-rust-crate/AGENTS/spec-driven-development.md) ·
[front-end](../event-front-end-with-svelte/AGENTS/spec-driven-development.md).

## Authority model

Two layers of "single source of truth", split by subject matter:

| Subject | Source of truth |
|---|---|
| A subproject's internals (algorithms, weights, routes, schema, components) | That subproject's own `spec/` |
| The cross-subproject contract (trio composition, service ↔ matcher DTO via `adapter.rs`, shared invariants, `/api/v1` versioning, entity-wide goals) | [`../spec/index.md`](../spec/index.md) |

Conflict rule: about crate internals, the crate spec wins; about the
integration contract, the entity spec wins. Either way, **the
disagreement becomes a task** (entity spec §13 or the crate's §13) —
never silently rewrite the losing document.

## Three-part PRs

A behavioural change is one PR: **spec edit + code edit + test
edit.** At the entity level this means:

- Change touches one subproject only → its spec, its code, its
  tests. Entity spec untouched unless the contract moved.
- Change touches the integration contract (wire format, adapter
  routing, shared invariant) → entity spec §5/§6 edit **plus** the
  affected crates' edits **plus** a seam-test edit
  (`tests/duplicate_detection.rs` for service ↔ matcher;
  front-end `tests/unit/*` for front-end ↔ service). Still one PR.

## Where work lives

- Live entity task queue: [`../spec/13-tasks.md`](../spec/13-tasks.md) (`ET-n`).
- Entity open questions: [`../spec/16-open-questions.md`](../spec/16-open-questions.md) (`EOQ-n`).
- Per-crate tasks keep their own numbering in their own §13.
- There is intentionally no `plan.md` and no `tasks.md` — plan
  content lives in spec §8–§12, tasks in §13, status in §14–§15.

## Anti-patterns

- **Drift-by-copy.** Don't paste the service's endpoint table or the
  matcher's weight table into entity docs — link down. Duplicated
  tables rot (the front-end README's person-entity leftovers are the
  cautionary example — ET-2).
- **Contract change without a seam test.** If `adapter.rs` or
  `types.ts` changes and no seam test changes, the PR is incomplete.
- **Cross-layer reach.** Front-end importing matcher types, service
  exposing matcher internals on the wire un-adapted, matcher growing
  IO — all forbidden by [`../spec/08-architecture.md`](../spec/08-architecture.md) §8.2.
- **Silent realignment.** Finding spec/code disagreement and "just
  fixing the code" (or the spec) without a task and a reviewable
  spec diff.
- **Aspirational status.** Multi-region, SSO, durable bus are
  roadmap (§15) until tests prove otherwise; never list them in §14.
