# Spec-Driven Development — Agent Guide

This crate practises **spec-driven development**: the specification
([`../spec.md`](../spec/index.md)) is the canonical artefact. Code conforms
to the spec; not the other way around.

## What that means in practice

- The spec is *living* — it changes whenever observable behaviour
  changes.
- A behavioural change PR has **three parts**: spec edit + code edit +
  test edit. All in one PR.
- When spec and code disagree, the spec is right. Open a task in §13
  of the spec to bring the code in line. Do **not** silently rewrite
  the spec to match broken code.
- When the spec is silent, propose an addition before writing code.

## When to update which section

| You're changing… | Update spec section… |
|---|---|
| `Worker` field list or types | §5.1 |
| Supporting types (`Location`, `Party`, `Offer`, `Identifier`, …) | §5.2 |
| Domain invariants (time window, capacity, attendance mode) | §5.3 |
| New capability surface (e.g. iCalendar I/O, recurrence) | §6.x |
| Default match weights, strong-identifier list, thresholds | §6.2 |
| Indexed field set or blocking strategy | §6.3 |
| Validation / normalisation rules | §6.5 |
| Privacy / masking / consent behaviour | §6.6 |
| FHIR coverage | §6.8 |
| Non-functional targets (latency, throughput, scale) | §7 |
| Module layout | §8.1 |
| Layering rules | §8.2 |
| Trait surface | §8.3 |
| API tier inventory | §9 |
| Database tables / extensions | §10 |
| Test inventory or coverage requirements | §11 |
| Compliance framework added / dropped | §12 |
| Adding work | §13 (new `T-N` entry) |
| Completing work | tick the box in §13 + CHANGELOG entry |
| Open-question resolution | move from §16 into the relevant section |
| Shifting roadmap priority | §15 |

## Anatomy of a good spec edit

- **Precise language.** Use RFC 2119 keywords (MUST / SHOULD / MAY)
  for normative statements.
- **Examples for normalisation, tables for weights, prose for
  algorithms.** Mix forms only when the data calls for it.
- **No code in the spec beyond minimal type signatures.** The spec is
  *what*, not *how*.
- **No screenshots or diagrams that can't be diffed.** Mermaid is
  fine; PNGs are not.
- **One concept per section.** If a section grows past ~300 lines,
  split it (e.g. `§6.2` → `§6.2a`).

## Anatomy of a good task (§13)

```
**T-NN — Short imperative title.**
- [ ] Concrete step.
- [ ] Another concrete step.
- **Acceptance:** A testable statement.
```

Tasks should be small enough to complete in a single PR. If a task is
bigger, split it (`T-12a`, `T-12b`).

## Closing the loop

When you finish a task:

1. Tick the box: `[x]`.
2. Add a CHANGELOG entry under "Unreleased" (or create one).
3. Verify the task's acceptance criterion is met by an automated test
   or a clearly described manual check.
4. If the task resolved an Open Question (§16), delete the OQ entry
   and re-anchor any references.

## CI enforcement (planned)

A `spec-drift` CI check (tracked as §13 T-7 family) will fail any
pull request that modifies `src/matching/**` or `src/models/worker.rs`
without also updating `spec.md`. Path-pattern exceptions will live in
`.spec-allow`.

Run it locally before pushing (once the script lands):

```bash
bash scripts/spec-drift-check.sh main HEAD
```

If you have a genuinely spec-irrelevant change (an internal refactor
of a private helper), prefer to add a one-line note to the spec — that
is almost always the right answer — over adding a `.spec-allow`
pattern. Every allow pattern erodes the discipline the check exists to
enforce.

## Anti-patterns

- "I'll write the code now and update the spec later" — later never
  comes.
- "The spec is wrong; let me just fix it to match the code" — without
  first confirming the code's behaviour is the *intended* behaviour,
  you are laundering a bug into a feature.
- Adding behaviour gated by a flag that the spec does not mention.
- "It's only a refactor" used to justify a behavioural shift.

## Document hierarchy

```
spec.md            ← what the library is, how it is built, and what is queued
                     (consolidates specification, plan, and tasks — authoritative)
README.md          ← user-facing intro (must stay consistent with spec)
CHANGELOG.md       ← what changed when (history)
AGENTS.md          ← directory of agents/* reference docs
agents/*.md        ← how to work in the repo + per-topic reference
index.md           ← navigation aid + worked examples
CLAUDE.md          ← project overview loaded by Claude Code at session start
```

There is intentionally **no** `plan.md` and **no** `tasks.md`. SDD
artefacts that some projects split across multiple files are
consolidated into the numbered sections of `spec.md`:

- "Plan" content (architecture, design, contracts) lives in `spec.md`
  §8–§12.
- "Tasks" content (work breakdown, acceptance criteria, status) lives
  in `spec.md` §13.
- "Status / roadmap" content lives in `spec.md` §14–§15.
- "Open questions" sit in `spec.md` §16 until resolved.

If you find disagreement between any of these documents, file it as
an issue and fix it in a follow-up PR.
