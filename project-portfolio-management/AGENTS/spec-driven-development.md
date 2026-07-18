# Spec-Driven Development — Portfolio Entity

The portfolio entity practises **spec-driven development** at two levels.
Read the per-crate discipline first (the matcher's
[AGENTS/spec-driven-development.md](../project-portfolio-management-matcher-rust-crate/AGENTS/spec-driven-development.md)
is the fullest local statement); this file adds the entity-level rules.

## Authority model

- Each subproject's `spec/` is the single source of truth **for that
  subproject's internals**.
- The entity-level [`../spec/`](../spec/index.md) is the single source of
  truth **for the cross-subproject contract**: trio composition, the four
  matchable collections, the DTO contract (API body = matcher `WorkItem`,
  persisted as JSONB), the endpoint inventory the front-end consumes
  (work-item CRUD + match/merge + sub-resources + derived views + links +
  bulk import/export), and shared invariants (entity spec §5.5).
- Disagreement about crate internals → the crate spec wins. Disagreement
  about the integration contract → the entity spec wins. Either way: open
  a task (entity spec §13 or crate spec §13) — never silently rewrite the
  loser.

## Three-part PRs

A behavioural change is one PR: **spec edit + code edit + test edit**. If
a change crosses subprojects (e.g. a new `WorkItem` field or a new
`RelationKind` variant), the one PR carries all the touched specs: matcher
spec §6, entity spec §5, front-end `types.ts` + its spec, and CHANGELOGs.

## When to update which entity-spec section

| You're changing… | Update entity spec… |
|---|---|
| `WorkItem` fields or enums (WorkItemKind, WorkItemStatus, RelationKind, IdentifierScheme) | §5.1 (+ matcher spec §6, front-end types) |
| Sub-resource shape (Goal, Task, Issue) | §5.2 (+ migration, front-end types) |
| Derived views (Timeline, Burndown) | §5.4 |
| JSONB persistence shape / table columns (the four work-item tables) | §5.3, §10 |
| Shared invariants | §5.5 |
| Endpoint inventory or wire conventions | §6, §9 |
| Match weights / thresholds / rules / the kind gate | §6.2 (mirror of matcher spec §5–§18) |
| Cross-service integration (auth / person / worker / org / links) | §8 |
| Non-functional targets | §7 |
| Compliance scope | §12 |
| Adding / completing work | §13 |
| Status honesty | §14 |
| Priorities | §15 |
| Open-question resolution | move from §16 into the relevant section |

## Anti-patterns

- Forking a service-side `WorkItem` DTO "for convenience" — the whole
  entity design is *one shape end to end* (matcher type = DTO = JSONB, no
  adapter).
- Collapsing the four kinds into one collection, or relaxing R-GATE into a
  weighted component — Portfolio / Project / Product / Program are
  distinct record types in distinct tables, and cross-kind pairs never
  match.
- Short-circuiting matches on owner-scoped codes (`Code` / `LocalId`) —
  they are not globally unique.
- Matching on sub-resources beyond goal *titles* — tasks / issues are
  operational, not identity.
- Re-introducing posts / comments / members as core — they are out of
  scope (roadmap only).
- Embedding another service's data in a `WorkItem` — store only the opaque
  `*_ref` ids; resolution belongs to the front-end / link aggregator.
- Updating the matcher type without updating the front-end's
  `src/lib/api/types.ts` in the same cycle.
- "I'll write the code now and update the spec later" — later never comes.
