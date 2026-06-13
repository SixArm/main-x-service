# Spec-Driven Development — Organization Entity

The organization entity practises **spec-driven development at two
levels**. Read this before changing anything that crosses a
subproject boundary.

## The authority model

| Document | Source of truth for |
|---|---|
| [`../spec/`](../spec/index.md) (this entity's spec) | The **cross-subproject contract**: trio composition, the DTO contract (API body = matcher `Organization` = JSONB payload), wire conventions, shared invariants, entity-wide goals |
| [service `spec/index.md`](../organization-service-rust-crate/spec/index.md) | Service internals: endpoints' implementation, table layout, loco wiring |
| [matcher `spec/index.md`](../organization-matcher-rust-crate/spec/index.md) | Matcher internals: algorithms, weights, normalisation, public API |
| [front-end `spec/index.md`](../organization-front-end-with-svelte/spec/index.md) | Front-end internals: routes, form behaviour, client |

Conflict resolution:

- About **crate internals** → the crate spec wins.
- About the **integration contract** → the entity spec wins.
- Either way: open a task (entity [spec §13](../spec/13-tasks.md) or
  the crate's queue) to bring the loser in line. Never silently
  rewrite either spec.

## Three-part PRs

A behavioural change is one PR: **spec edit + code edit + test edit.**
If the change crosses a boundary, the PR carries *both* spec edits
(entity + crate). Examples:

| Change | Specs to edit |
|---|---|
| New matcher weight default | matcher §7 only (then entity §6 FR-5 table — it restates defaults) |
| New REST endpoint | service §6/§9 + entity §9 (and §6 if a new capability) |
| DTO field added to `Organization` | matcher §6 + entity §5 + front-end types (its §13 if deferred) — one change cycle |
| Soft-delete semantics | entity §5.5 (shared invariant) + service spec |
| Front-end form behaviour | front-end spec only |

## When to update which entity-spec section

| You're changing… | Update entity spec… |
|---|---|
| DTO fields / identifier schemes | §5 |
| Shared invariants | §5.5 |
| Capability set or its owner | §6 |
| Scale / security / i18n targets | §7 |
| Trio composition, dependency direction | §8 |
| Endpoint paths, shapes, status codes, wire naming | §9 |
| Tables / JSONB contract | §10 |
| Seam-test obligations | §11 |
| Compliance scope | §12 |
| Adding cross-subproject work | §13 (new `T-N`) |
| Completing work | tick §13 + update §14 |
| Roadmap priority | §15 |
| Open-question resolution | move §16 → the relevant section |

## Anatomy of a good task (§13)

```
**T-NN — Short imperative title.**
- [ ] Concrete step.
- **Acceptance:** A testable statement.
```

Small enough for one PR; split otherwise (`T-7a`, `T-7b`).
Entity tasks reference, not duplicate, the subproject queues
(service §13, matcher §23, front-end §13).

## Anti-patterns

- "I'll write the code now and update the spec later" — later never
  comes.
- Fixing a contract mismatch by editing only one side's spec.
- Forking a service-side `Organization` DTO "temporarily" — the
  one-type contract is the entity's core design decision.
- Describing the wire format from schema.org memory instead of the
  actual serde output (see entity §16 OQ-1 — this already happened).
- Changing matcher weights/threshold without updating matcher spec §7
  **and** the restated table in entity §6.

## Per-crate discipline guides

- Matcher: [`AGENTS/spec-driven-development.md`](../organization-matcher-rust-crate/AGENTS/spec-driven-development.md)
  (the most detailed in this entity).
- Service / front-end: "Golden rules" in their
  [`AGENTS.md`](../organization-service-rust-crate/AGENTS.md) /
  [`AGENTS.md`](../organization-front-end-with-svelte/AGENTS.md);
  full per-crate guides are queued (entity §13 T-1).
- House exemplar: the person service's
  [`AGENTS/spec-driven-development.md`](../../person/person-service-rust-crate/AGENTS/spec-driven-development.md).
