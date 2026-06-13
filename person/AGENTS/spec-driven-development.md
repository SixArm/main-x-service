# Spec-Driven Development — Entity-Level Guide

The person entity practises **spec-driven development at two levels**.
Each subproject's `spec/` is the canonical artefact for that
subproject's internals; the entity-level
[`../spec/index.md`](../spec/index.md) is the canonical artefact for
the cross-subproject contract. Code conforms to spec; not the other
way around.

## Authority model

| Question | Authoritative spec |
|---|---|
| What fields does the service `Person` have? What are the match weights? | [service spec](../person-service-rust-crate/spec/index.md) |
| How does the matcher score? Which identifier schemes exist? | [matcher spec](../person-matcher-rust-crate/spec/index.md) |
| What routes / components does the UI have? | [front-end spec](../person-front-end-with-svelte/spec/index.md) |
| How does the trio compose? What is the adapter contract? What invariants are shared? | [entity spec](../spec/index.md) |

When the entity spec and a crate spec disagree **about crate
internals**, the crate spec wins. When they disagree **about the
integration contract**, the entity spec wins. Either way, file a task
(entity spec §13 or the crate's task section) — never silently
reconcile.

## Three-part PRs, two levels

A behavioural change is one PR: **spec edit + code edit + test edit**.

- Change inside one subproject → that subproject's spec + code +
  tests. Follow its own SDD guide:
  [service](../person-service-rust-crate/AGENTS/spec-driven-development.md),
  [matcher](../person-matcher-rust-crate/AGENTS/spec-driven-development.md),
  [front-end](../person-front-end-with-svelte/AGENTS.md).
- Change touching a **seam** (adapter routing, wire types, shared
  invariants, composition rules) → edit the entity spec **and** the
  affected crate spec(s), plus code, plus a seam test
  ([entity spec §11.2–§11.3](../spec/11-testing-strategy.md)).

## Which entity-spec section to edit

| You're changing… | Update entity spec… |
|---|---|
| Adapter routing rules (`adapter.rs`) | §5.3 (+ service spec §6.2 + bridge test) |
| Wire format / envelope / front-end types | §5.4 (+ front-end types + unit test) |
| A shared invariant (soft delete, scheme-locality, score range) | §5.5 |
| Composition rules (who may call what) | §6.1 |
| Entity-wide scale / security / i18n targets | §7 |
| Deployment topology or SSO plan | §8 |
| Compliance posture | §12 |
| Adding cross-subproject work | §13 (new `E-N` entry) |
| Completing work | tick the box in §13 |
| Roadmap priority | §15 |
| Open-question resolution | move from §16 into the relevant section |

## Task conventions

Entity tasks are `E-N` (split as `E-Na`, `E-Nb`); they may reference
subproject tasks (service `T-N`, matcher `T-N`, front-end `T-N`) but
MUST NOT duplicate their content. House anatomy:

```
**E-NN — Short imperative title.**
- [ ] Concrete step (which subproject).
- **Acceptance:** A testable statement.
```

## Anti-patterns

- Editing only the entity spec for a change that alters crate
  behaviour (or vice versa) — seam changes need both.
- Duplicating a crate's weight tables / endpoint tables into the
  entity docs — link down instead; duplication is how drift starts.
- "It's only the adapter" — adapter changes are contract changes;
  they need §5.3 wording and a bridge test in the same PR.
- Resolving an entity ↔ crate spec conflict by silently rewriting one
  side.
