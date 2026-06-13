# Spec-driven development — Course Entity

The discipline: **the spec is the source of truth.** Code conforms to
the spec; not the other way around. At entity level there are **four**
living specs, with a clear authority split.

## Authority model

| Question | Governing spec |
|---|---|
| How does the trio compose? DTO contract, wire contract, shared invariants, entity goals | [`../spec/`](../spec/index.md) (entity level) |
| Service internals (handlers, repositories, validation, persistence) | [service spec](../course-service-rust-crate/spec/index.md) (§1–§18) |
| Matcher internals (algorithms, weights, normalisation) | [matcher spec](../course-matcher-rust-crate/spec/index.md) (§1–§25) |
| Front-end internals (routes, components, build) | [front-end spec](../course-front-end-with-svelte/spec/index.md) (§1–§18) |

On conflict: **crate spec wins on crate internals; entity spec wins
on the integration contract.** Either way, open a task (entity
[§13](../spec/13-tasks.md) or the crate's queue) to reconcile — never
silently rewrite the loser.

## Three-part PRs

Every behavioural change comes as a single PR with three parts:

1. **Spec edit** — the governing spec per the table above (or open a
   §16 question, then a follow-up PR with the resolution).
2. **Code edit** — the owning subproject's source.
3. **Test edit** — including the **cross-subproject** test when the
   contract moved: bridge test (`course-service/tests/duplicate_detection.rs`)
   for service↔matcher, front-end unit tests for service↔front-end.

Reviewers reject PRs that change behaviour without touching all three.

## Section mapping (entity spec)

| Entity spec section | Corresponds to |
|---|---|
| §5 Domain Model | `course-service/src/matching/adapter.rs`, `course-matcher/src/course.rs`, `course-front-end/src/lib/api/types.ts` |
| §6 Functional Requirements | The trio's composed behaviour; each FR names its owner |
| §7 Non-Functional Requirements | Benches, deployment artefacts, locales, SSO plan |
| §8 Architecture | Dependency direction, loco boot shape, deployment topology |
| §9 API Surface | Service routes + front-end routes + envelope |
| §11 Testing Strategy | The bridge test + front-end client tests as composition pins |
| §13 Tasks | Live entity-level work queue — cross-subproject work only |

## Anti-patterns

- **Changing the adapter routing without a bridge-test edit.** The
  adapter (`src/matching/adapter.rs`) is the pinch point for
  service↔matcher drift; the 14 bridge tests are its lock.
- **Changing the service wire format without fixing
  `src/lib/api/types.ts`** in the same change cycle. The front-end
  mirrors, it does not negotiate.
- **Adding a deterministic identifier scheme in the matcher alone.**
  A false positive at score 1.0 is the worst-case bug; the scheme
  lands matcher-spec + matcher-code + service-bridge-test together.
- **Putting crate-internal tasks in the entity §13.** The entity
  queue is for cross-subproject work; crate work goes to the crate
  queue.
- **Duplicating crate tables into entity docs.** Entity docs give
  the shape and link down; exhaustive tables live in one place — the
  owning crate.
- **Treating course codes as global.** Provider-scoped, everywhere,
  always (entity spec §5.5).
