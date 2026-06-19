# Spec-Driven Development — Agent Guide

This crate practises **spec-driven development**: the specification
([`../spec/index.md`](../spec/index.md)) is the canonical artefact. Code conforms
to the spec; not the other way around.

## What That Means In Practice

- The spec is *living* — it changes whenever observable behaviour
  changes.
- A behavioural change PR has **three** parts: spec edit, code edit,
  test edit. All in one PR.
- When spec and code disagree, the spec is right. Open a task in §23
  of the spec to bring the code in line. Do **not** silently rewrite
  the spec to match broken code.
- When the spec is silent, propose an addition before writing code.
- When the public surface of the matcher changes (re-exports from
  `lib.rs`, `MatchConfig` fields, `MatchResult` / `MatchBreakdown`
  shape, deterministic-identifier scheme list), update the bridge
  test in
  [`../portfolio-service-with-loco/tests/matching.rs`](../../portfolio-service-with-loco/tests/matching.rs)
  in the same PR.

## When To Update Which Section

The portfolio-matcher spec uses the §1–§25 matcher-crate shape (distinct
from the §1–§18 shape used by the service crates).

| You're changing… | Update spec section… |
|---|---|
| Purpose / scope statement | §1 / §2 |
| A new term in the vocabulary | §3 Glossary |
| Citing a paper or external rule | §4 Research basis |
| The pipeline at a glance | §5 Algorithm overview |
| The list of `WorkItem` fields | §6 Domain model |
| Default weights or threshold | §7 Configuration |
| Normalisation behaviour | §8 Normalisation |
| Name-similarity algorithm | §9 Name similarity |
| Goal-title overlap (Jaccard) | §10 Goals |
| Code rule (same-owner gate, shape) | §11 Code |
| Owner-org score | §11a Owner org |
| Portfolio (parent-portfolio) score | §11b Portfolio |
| Kind gate (R-GATE) / timeframe score | §12 Kind gate & timeframe |
| Keywords Jaccard | §13 Keywords |
| Relationships typed-set Jaccard | §13.1 Relationships |
| Tags set Jaccard | §13.2 Tags |
| (reserved) | §14 |
| Identifier short-circuit scheme list | §15 Deterministic identifier short-circuits |
| Owner+code (R-1) / same_as (R-2) short-circuit | §16 Owner+code, same_as, open questions |
| Renormalisation arithmetic | §17 |
| Confidence-band thresholds | §18 |
| Quality goals (precision / recall targets) | §19 |
| How `portfolio-service` consumes the crate | §20 |
| SemVer / MSRV compatibility | §21 |
| Anti-patterns | §22 |
| New scheduled work | §23 (add a task) |
| Completed work | tick the box in §23 + CHANGELOG |
| Test coverage requirements | §24 Testing strategy |
| Open question resolution | move from the relevant section into the section it belongs in |
| Toolchain / release process | §25 Change control |

## Anatomy of a Good Spec Edit

- **Precise language.** Use RFC 2119 keywords (MUST / SHOULD / MAY)
  for normative statements.
- **Examples for normalisation, tables for weights, prose for
  algorithms.** Mix forms only when the data calls for it.
- **No code in the spec beyond minimal type signatures.** The spec is
  what, not how.
- **No screenshots or diagrams that can't be diffed.** Mermaid is
  fine; PNGs are not.
- **One concept per section.** If a section grows past ~300 lines,
  split it.

## Anatomy of a Good Task (§23)

```
**T-NN — Short imperative title.**
- [ ] Concrete step.
- [ ] Another concrete step.
- **Acceptance:** A testable statement.
```

Tasks should be small enough to complete in a single PR. If a task is
bigger, split it (`T-12a`, `T-12b`).

## Section Mapping (spec → code)

| Spec section | Corresponds to |
|---|---|
| §1 Purpose / §2 Scope | repo-level positioning (also `AGENTS.md`) |
| §3 Glossary | `src/work_item.rs` types, `src/scoring.rs` enums |
| §4 Research basis | `AGENTS/matching-algorithm.md` |
| §5 Algorithm overview | `src/matcher.rs` |
| §6 Domain model | `src/work_item.rs` |
| §7 Configuration | `src/config.rs` (`MatchConfig`) |
| §8 Normalisation | `src/normalize.rs`, `AGENTS/normalization.md` |
| §9–§13 per-component scoring | `src/matcher.rs` component fns |
| §12 Kind gate (R-GATE) | `src/matcher.rs` gate (first rule) |
| §15–§16 short-circuits | `src/matcher.rs` deterministic gate |
| §17 Renormalisation | `src/scoring.rs` weighted-sum helper |
| §18 Confidence classification | `src/scoring.rs` `Confidence` |
| §19–§21 quality / consumption / compat | top-level docs, integration with `portfolio-service` |
| §22 Anti-patterns | this file + AGENTS.md |
| §23 Tasks | spec.md only — the live work queue |
| §24 Testing | `AGENTS/testing.md` |
| §25 Change control | this file |

## Closing the Loop

When you finish a task:

1. Tick the box: `[x]`.
2. Add a `CHANGELOG.md` entry under "Unreleased".
3. Verify the task's acceptance criterion is met by an automated
   test or a clearly described manual check.
4. If the task resolved an Open Question, delete the OQ entry and
   re-anchor any references.

## Anti-Patterns

- "I'll write the code now and update the spec later" — later never
  comes.
- "The spec is wrong; let me just fix it to match the code" —
  without first confirming the code's behaviour is the *intended*
  behaviour, you're laundering a bug into a feature.
- **Matching across kinds.** Per §12, R-GATE refuses any `A.kind !=
  B.kind` comparison at `0.0`. A project is never a product.
- **Adjusting a default weight without a §7 spec edit.** Reviewers
  diff the spec against `MatchConfig::default()`.
- **Adding a deterministic-identifier scheme without bridge-test
  coverage** in `portfolio-service`. A false positive at score 1.0 is
  the worst-case bug.
- **Sneaking normalisation behaviour into `matcher.rs`.** Push it
  into `normalize::` and document the rule under §8.
- **Using `unwrap` / `expect` in library code.** Total functions
  only.
- **Scoring a `code` across owners.** Per §11, the code component MUST
  be gated on `owner_org_id` equality (PROJ-01 at one org != PROJ-01 at
  another).
- **Matching on `status`.** Duplicate records routinely sit at
  different statuses; it is informational-only.
- **Adding behaviour gated by a flag that the spec doesn't mention.**
- **"It's only a refactor" used to justify a behavioural shift.**

## Document Hierarchy

```
spec.md            ← what the library is, how it is built, and what
                     is queued (§1–§25; live tasks in §23)
README.md          ← user-facing intro (must stay consistent with spec)
CHANGELOG.md       ← what changed when (history)
AGENTS.md          ← how to work in the repo (entry point)
AGENTS/*.md        ← topic-specific agent guides
CLAUDE.md          ← @AGENTS.md (Claude Code entry)
index.md           ← navigation aid
```

There is intentionally **no** `plan.md` and **no** `tasks.md`. SDD
artefacts that some projects split across multiple files are
consolidated into the numbered sections of `spec.md`:

- "Plan" content (architecture, design, contracts) lives in
  `spec.md` §1–§22.
- "Tasks" content (work breakdown, acceptance criteria, status)
  lives in `spec.md` §23.
- Testing strategy lives in `spec.md` §24.
- Change-control / release lives in `spec.md` §25.

If you find disagreement between any of these, file it as an issue
and fix it in a follow-up PR.
