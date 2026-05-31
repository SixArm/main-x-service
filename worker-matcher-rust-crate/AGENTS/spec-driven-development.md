# Spec-Driven Development — Agent Guide

This crate practises **spec-driven development**: the specification ([`../spec.md`](../spec.md)) is the canonical artefact. Code conforms to the spec; not the other way around.

## What That Means In Practice

- The spec is *living* — it changes whenever observable behaviour changes.
- A behavioural change PR has **three** parts: spec edit, code edit, test edit. All in one PR.
- When spec and code disagree, the spec is right. Open a task in §23 of the spec to bring the code in line. Do **not** silently rewrite the spec to match broken code.
- When the spec is silent, propose an addition before writing code.

## When To Update Which Section

| You're changing… | Update spec section… |
|---|---|
| The list of `Worker` fields | §8.1 |
| Default weights or threshold | §13 |
| Component scoring rules | §12 |
| Normalisation behaviour | §14 |
| Error variants | §15 |
| Test coverage requirements | §18 |
| Toolchain / release process | §19 |
| Module layout | §9.1 |
| Open question resolution | move from §22 into the relevant section |
| New scheduled work | §23 (add a task) |
| Completed work | tick the box in §23 + CHANGELOG |

## Anatomy of a Good Spec Edit

- **Precise language.** Use RFC 2119 keywords (MUST / SHOULD / MAY) for normative statements.
- **Examples for normalisation, tables for weights, prose for algorithms.** Mix forms only when the data calls for it.
- **No code in the spec beyond minimal type signatures.** The spec is what, not how.
- **No screenshots or diagrams that can't be diffed.** Mermaid is fine; PNGs are not.
- **One concept per section.** If a section grows past ~300 lines, split it.

## Anatomy of a Good Task (§23)

```
**T-NN — Short imperative title.**
- [ ] Concrete step.
- [ ] Another concrete step.
- **Acceptance:** A testable statement.
```

Tasks should be small enough to complete in a single PR. If a task is bigger, split it (`T-12a`, `T-12b`).

## Closing the Loop

When you finish a task:

1. Tick the box: `[x]`.
2. Add a CHANGELOG entry under "Unreleased".
3. Verify the task's acceptance criterion is met by an automated test or a clearly described manual check.
4. If the task resolved an Open Question (§22), delete the OQ entry and re-anchor any references.

## CI Enforcement

The `spec-drift` CI check (`.github/workflows/spec-drift.yml`, spec.md §23 T-7) fails any pull request that modifies `src/matcher.rs` without also updating `spec.md`. Path-pattern exceptions live in `.spec-allow`.

Run it locally before pushing:

```bash
bash scripts/spec-drift-check.sh main HEAD
```

If you have a genuinely spec-irrelevant change (e.g. an internal refactor of a private helper), prefer to add a one-line note to the spec — that's almost always the right answer — over adding a `.spec-allow` pattern. Every allow pattern erodes the discipline the check exists to enforce.

## Anti-Patterns

- ❌ "I'll write the code now and update the spec later" — later never comes.
- ❌ "The spec is wrong; let me just fix it to match the code" — without first confirming the code's behaviour is the *intended* behaviour, you're laundering a bug into a feature.
- ❌ Adding behaviour gated by a flag that the spec doesn't mention.
- ❌ "It's only a refactor" used to justify a behavioural shift.

## Document Hierarchy

```
spec.md            ← what the library is, how it is built, and what is queued
                     (consolidates specification, plan, and tasks — authoritative)
README.md          ← user-facing intro (must stay consistent with spec)
CHANGELOG.md       ← what changed when (history)
IMPLEMENTATION_SUMMARY.md  ← historical snapshot; do not edit (superseded by spec)
AGENTS.md + AGENTS/*.md   ← how to work in the repo
index.md           ← navigation aid
```

There is intentionally **no** `plan.md` and **no** `tasks.md`. SDD artefacts that some projects split across multiple files are consolidated into the numbered sections of `spec.md`:

- "Plan" content (architecture, design, contracts) lives in `spec.md` §8–§20.
- "Tasks" content (work breakdown, acceptance criteria, status) lives in `spec.md` §23.
- The mapping is documented in `spec.md` §24.2.

If you find disagreement between any of these, file it as an issue and fix it in a follow-up PR.
