# Spec-Driven Development — Case Entity

The case entity practises **spec-driven development** at two levels.
Read the per-crate discipline first (the matcher's
[AGENTS/spec-driven-development.md](../case-matcher-rust-crate/AGENTS/spec-driven-development.md)
is the fullest local statement); this file adds the entity-level rules.

## Authority model

- Each subproject's `spec/` is the single source of truth **for that
  subproject's internals**.
- The entity-level [`../spec/`](../spec/index.md) is the single source
  of truth **for the cross-subproject contract**: trio composition, the
  DTO contract (API body = matcher `Case`, persisted as JSONB), the
  endpoint inventory the front-end consumes, and shared invariants
  (entity spec §5.5).
- Disagreement about crate internals → the crate spec wins.
  Disagreement about the integration contract → the entity spec wins.
  Either way: open a task (entity spec §13 or crate spec §13) — never
  silently rewrite the loser.

## Three-part PRs

A behavioural change is one PR: **spec edit + code edit + test edit**.
If a change crosses subprojects (e.g. a new `Case` field), the one PR
carries all the touched specs: matcher spec §6, entity spec §5,
front-end `types.ts` + its spec, and CHANGELOGs.

## When to update which entity-spec section

| You're changing… | Update entity spec… |
|---|---|
| `Case` fields or enums | §5.1 (+ matcher spec §6, front-end types) |
| JSONB persistence shape / table columns | §5.3, §10 |
| Shared invariants | §5.5 |
| Endpoint inventory or wire conventions | §6, §9 |
| Match weights / thresholds / rules | §6.2 (mirror of matcher spec §5–§18) |
| Non-functional targets | §7 |
| Trio composition / deployment / SSO | §8 |
| Compliance / privacy scope | §12 |
| Adding / completing work | §13 |
| Status honesty | §14 |
| Priorities | §15 |
| Open-question resolution | move from §16 into the relevant section |

## Anti-patterns

- Forking a service-side `Case` DTO "for convenience" — the whole
  entity design is *one shape end to end*.
- Short-circuiting matches on agency-scoped identifiers
  (`AgencyCaseNumber` / `LocalId` / `case_number` across agencies) —
  they are not globally unique.
- Putting personal detail or substantive case content in any free-text
  field; `subjects` carry only opaque ids (entity spec §5.5, §12). Case
  data is personal data — treat every payload accordingly.
- Updating the matcher type without updating the front-end's
  `src/lib/api/types.ts` in the same cycle.
- "I'll write the code now and update the spec later" — later never
  comes.
