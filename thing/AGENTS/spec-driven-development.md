# Spec-Driven Development — Thing Entity Agent Guide

The thing entity practises spec-driven development at **two levels**.
This guide tells you which spec to edit for a given change.

## Authority model

- Each subproject's own `spec/` is the single source of truth **for
  that subproject's internals** —
  [service](../thing-service-rust-crate/spec/index.md),
  [matcher](../thing-matcher-rust-crate/spec/index.md),
  [front-end](../thing-front-end-with-svelte/spec/index.md).
- The entity-level [`../spec/`](../spec/index.md) is the source of
  truth **for the cross-subproject contract**: how the trio composes,
  the service ↔ matcher DTO contract (entity §5.3), the REST surface
  the front-end consumes (entity §9), shared invariants (entity
  §5.4), and entity-wide goals.
- Disagreement about **crate internals** → the crate spec wins.
  Disagreement about the **integration contract** → the entity spec
  wins. Either way, open a task (entity §13 or crate §13) — do not
  silently rewrite the loser.

## Three-part PRs

A behavioural change is one PR: **spec edit + code edit + test edit**.
When the change crosses subprojects, the PR carries edits to every
affected spec — typically the entity spec plus one or more crate
specs — and the bridge tests.

## Which spec do I edit?

| You're changing… | Edit |
|---|---|
| A `Thing` field, validation rule, endpoint internals | Service spec (§5 / §6 / §9) — plus entity §5–§9 if a summary here mentions it |
| A matcher weight, normalisation rule, public type | Matcher spec (§3–§8) |
| A front-end route, component, form | Front-end spec (§5 / §6) |
| The adapter mapping (`to_matcher_thing` routing) | **Entity spec §5.3** + service spec §6.2 + bridge tests |
| The REST endpoints the front-end consumes | **Entity spec §9** + service spec §9 + front-end spec §9 |
| Shared invariants (identifier semantics, soft-delete-only) | **Entity spec §5.4** + every affected crate spec |
| Confidence vocabulary / threshold mapping | **Entity spec** (§4, §16 OQ-2, T-8) |
| Entity-wide goals, NFR targets, compliance scope, roadmap | **Entity spec §1 / §7 / §12 / §15** |
| Cross-subproject work | **Entity spec §13** (new `T-N`) |

## The bridge tests are the contract's enforcement

[`thing-service-rust-crate/tests/duplicate_detection.rs`](../thing-service-rust-crate/tests/duplicate_detection.rs)
pins both sides of the DTO contract. Any edit to entity §5.3, to
[`adapter.rs`](../thing-service-rust-crate/src/matching/adapter.rs),
or to the matcher's scoring MUST update a bridge test in the same PR.

## Per-subproject discipline

Each subproject has its own SDD guide — read it before working there:

- [service AGENTS/spec-driven-development.md](../thing-service-rust-crate/AGENTS/spec-driven-development.md)
- [matcher AGENTS/spec-driven-development.md](../thing-matcher-rust-crate/AGENTS/spec-driven-development.md)
- [front-end AGENTS/spec-driven-development.md](../thing-front-end-with-svelte/AGENTS/spec-driven-development.md)

## Anti-patterns

- Editing the entity spec to describe a crate internal in detail —
  summarise and link down instead; duplicated tables drift.
- Changing the adapter "because the matcher changed" without an
  entity §5.3 edit — that's the contract, not an internal.
- Fixing a broken cross-subproject link by deleting it — repair it
  (see entity §13 T-1) so the document graph stays navigable.
- "Plan" or "tasks" files — there is intentionally no `plan.md` and
  no `tasks.md`; plans live in spec §8–§12, tasks in §13, status in
  §14–§15, open questions in §16.
