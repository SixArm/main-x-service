# AGENTS.md — Working Guide for AI Coding Agents

Entry point for AI coding agents working in the `project-portfolio-management-matcher` Rust
crate.

> If you only read one file, read [`spec/index.md`](./spec/index.md) —
> the living specification.

## Quick orientation

| Question | Answer |
|---|---|
| What does the crate do? | Pairwise work-item (Portfolio / Project / Product / Program) record matching, deterministic + probabilistic, for within-collection dedup. |
| Canonical spec? | [`spec/index.md`](./spec/index.md). |
| Entity domain model? | [`../spec/index.md`](../spec/index.md) §5 (entity-level umbrella). |
| Build / test / lint / fmt | `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt` |
| Run the demo | `cargo run` (`src/main.rs`; not SemVer surface). |
| Public types | `src/lib.rs` re-exports from `src/{work_item,matcher,scoring,config,normalize,phonetic,error}.rs`. |
| Match gate (R-GATE) | `A.kind != B.kind` → `0.0` no-match, **before** every other rule. Matching is within-kind only. |
| Deterministic schemes (→ 1.0) | URI, UUID, Jira project key, Asana GID, Trello board id, MS Project id, GitHub project id, Linear id; plus same-owner code (R-1) and `same_as` URL overlap (R-2). |
| Probabilistic components | name (Jaro-Winkler + Soundex), goals (Jaccard over titles), owner-scoped code, owner org, portfolio (parent-portfolio exact), timeframe (Gaussian decay), keywords (Jaccard), relationships (typed-set Jaccard), tags (set Jaccard). |
| Public API shape | `MatchingEngine::new(MatchConfig::default()).match_work_items(&a, &b) -> MatchResult`. |

## Golden rules

1. **Spec-first.** Behavioural change ⇒ update `spec/index.md`.
2. **Kind-gated.** Two work items of different `kind` never match — the
   R-GATE is absolute (§5 / §12).
3. **Pure library.** No IO, no logging, no global state in `src/`
   (excluding `src/main.rs`).
4. **No `unsafe`. No `unwrap`/`expect`/`panic!`** in library code.
5. **Deterministic.** No clocks, RNGs, or environment variables.
6. **Explainability.** Every match returns a per-component breakdown.
7. **Diacritic-correct.**

## What not to do

- Do not match across kinds — R-GATE refuses a Project vs. Product
  comparison at `0.0`.
- Do not short-circuit on owner-scoped (`Code`/`LocalId`) or `Custom`
  schemes — they are not globally unique.
- Do not score a `code` across differing owners.
- Do not match on `status` (it drifts between duplicate records).
- Do not change default weights/threshold without updating
  `spec/index.md §7` and `CHANGELOG.md`.

## Detailed guides

- [AGENTS/matching-algorithm.md](./AGENTS/matching-algorithm.md)
- [AGENTS/normalization.md](./AGENTS/normalization.md)
- [AGENTS/spec-driven-development.md](./AGENTS/spec-driven-development.md)
- [AGENTS/testing.md](./AGENTS/testing.md)

## File layout

```
src/
├── lib.rs         public re-exports
├── main.rs        demo binary (not SemVer surface)
├── work_item.rs   domain types (WorkItem, WorkItemKind, Goal, GoalStatus, WorkItemStatus, IdentifierScheme, WorkItemIdentifier, WorkItemRelationship, RelationKind)
├── matcher.rs     MatchingEngine + R-GATE + per-component fns + deterministic rules
├── scoring.rs     MatchResult, MatchBreakdown, Confidence, weighted_average
├── normalize.rs   fold, code, fold_set
├── phonetic.rs    Soundex (name component bonus)
├── config.rs      MatchConfig (weights + threshold)
└── error.rs       error type
```
