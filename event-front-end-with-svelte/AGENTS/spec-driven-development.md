# Spec-Driven Development — Agent Guide

This project practises **spec-driven development**: the specification
([`../spec.md`](../spec.md)) is the canonical artefact for this
front-end. Code conforms to the spec; not the other way around.

## What that means in practice

- The spec is *living* — it changes whenever observable behaviour
  changes.
- A behavioural change PR has **three** parts: spec edit, code edit,
  test edit. All in one PR.
- When spec and code disagree, the spec is right. Open a task in §13
  of the spec to bring the code in line. Do **not** silently rewrite
  the spec to match broken code.
- When the spec is silent, propose an addition before writing code.

## Two specs, two contracts

This is a thin presentation layer over a system of record. There are
**two** specs in play:

1. **Service spec** —
   [`../../event-service-rust-crate/spec.md`](../../event-service-rust-crate/spec.md)
   — describes the API contract. If `Event` loses a field server-side,
   fix `src/lib/api/types.ts` here; do not let the front-end drift.
2. **Front-end spec** — [`../spec.md`](../spec.md) — describes
   front-end-specific decisions: routes, components, design system,
   build pipeline, testing layers.

Cross-spec rule: **the service spec wins on wire shape**. Only edit
this project's spec for front-end-only concerns (route layout, form
UX, component composition, build tooling).

## When to update which section

This project's spec uses the §1–§18 SDD shape (matching the service
crates and other front-ends).

| You're changing… | Update spec section… |
|---|---|
| Purpose / scope statement | §1 / §2 |
| Glossary entries | §4 |
| Information architecture / route map | §5 |
| What the UI does (FR-N) | §6 Functional Requirements |
| Performance / accessibility / build targets | §7 Non-Functional Requirements |
| Component / module layout | §8 Architecture |
| API client / repository shape | §9 API Consumption |
| Local-storage / IndexedDB use | §10 Persistence (typically out-of-scope) |
| Test layer counts, harness | §11 Testing Strategy |
| GDPR / accessibility / compliance | §12 Compliance |
| New scheduled work | §13 (add a task) |
| Completed work | tick the box in §13 + CHANGELOG |
| Implementation status snapshot | §14 |
| Versioning roadmap (v0.2 / v0.3 / …) | §15 Roadmap |
| Open questions | §16 |
| External references | §17 |
| Release / change-control discipline | §18 |

## Anatomy of a good spec edit

- **Precise language.** Use RFC 2119 keywords (MUST / SHOULD / MAY)
  for normative statements.
- **Tables for routes, prose for component contracts, code blocks
  only for representative snippets.**
- **No screenshots.** Describe the route, not the pixels.
- **One concept per section.** If a section grows past ~300 lines,
  split it.

## Anatomy of a good task (§13)

```
**T-NN — Short imperative title.**
- [ ] Concrete step.
- [ ] Another concrete step.
- **Acceptance:** A testable statement (svelte-check clean,
  vitest N/N, playwright N/N, or a manual verification).
```

Tasks should be small enough to complete in a single PR. If a task is
bigger, split it (`T-12a`, `T-12b`).

## Closing the loop

When you finish a task:

1. Tick the box: `[x]`.
2. Add a `CHANGELOG.md` entry under "Unreleased".
3. Verify the task's acceptance criterion is met by an automated
   test (vitest / playwright / svelte-check) or a clearly described
   manual check.
4. If the task resolved an Open Question (§16), delete the OQ entry
   and re-anchor any references.

## Drift policy (project-wide)

Per the family-wide decision recorded in
[`../../AGENTS.md`](../../AGENTS.md): each `*-front-end-with-svelte`
project keeps its **own** copy of API types, client, and form
primitives. Drift between sibling front-ends is **accepted**. Do
NOT factor shared code into a `mxi-svelte-core` package without
explicit user approval.

Practical implication: when you spot a bug in this project's
`ApiClient`, do not also "fix" the sibling projects' clients in the
same PR. They are independent. File a follow-up task in each sibling's
own spec.md §13 if the same bug applies there.

## Anti-patterns

- "I'll write the code now and update the spec later" — later never
  comes.
- "The spec is wrong; let me just fix it to match the code" — without
  first confirming the code's behaviour is the *intended* behaviour,
  you're laundering a bug into a feature.
- **Adding a UI route that doesn't appear in §5 (Information
  Architecture) or §6 (FRs).** Update the spec first.
- **Changing the wire-format type to "fix" a server bug.** If
  `Event` doesn't match `event-service`'s `Event`, the fix is in
  the service. File a task in the service's spec.md §13.
- **Using legacy Svelte 4 syntax** (`$:`, `export let`). Svelte 5
  runes only — see AGENTS.md.
- **Adding SSR back without a §5 edit** explaining why. The default
  is SPA-only because the SVAR Grid is browser-only.

## Document hierarchy

```
spec.md            ← what the front-end is, how it is built, and what
                     is queued (§1–§18; live tasks in §13)
README.md          ← user-facing intro (must stay consistent with spec)
CHANGELOG.md       ← what changed when (history)
AGENTS.md          ← how to work in the repo (entry point + ground rules)
AGENTS/*.md        ← topic-specific agent guides
CLAUDE.md          ← @AGENTS.md (Claude Code entry)
index.md           ← navigation + worked flows
```

There is intentionally **no** `plan.md` and **no** `tasks.md`. SDD
artefacts that some projects split across multiple files are
consolidated into the numbered sections of `spec.md`.
