# AGENTS.md — Working Guide for AI Coding Agents

Entry point for AI coding agents working in the `plan-matcher` Rust
crate.

> If you only read one file, read [`spec/index.md`](./spec/index.md) —
> the living specification.

## Quick orientation

| Question | Answer |
|---|---|
| What does the crate do? | Pairwise plan (project / product / programme / initiative / portfolio / epic) record matching, deterministic + probabilistic, for portfolio dedup. |
| Canonical spec? | [`spec/index.md`](./spec/index.md). |
| Build / test / lint / fmt | `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt` |
| Run the demo | `cargo run` (`src/main.rs`; not SemVer surface). |
| Public types | `src/lib.rs` re-exports from `src/{plan,matcher,scoring,config,normalize,phonetic,error}.rs`. |
| Deterministic schemes (→ 1.0) | URI, UUID, Jira project key, Asana GID, Trello board id, MS Project id, GitHub project id, Linear id; plus same-owner plan-code (R-1) and `same_as` URL overlap (R-2). |
| Probabilistic components | name (Jaro-Winkler + Soundex), goals (Jaccard over titles), owner-scoped plan code, owner org, plan type, timeframe (Gaussian decay), keywords (Jaccard), relationships (typed-set Jaccard), tags (set Jaccard). |
| Public API shape | `MatchingEngine::new(MatchConfig::default()).match_plans(&a, &b) -> MatchResult`. |

## Golden rules

1. **Spec-first.** Behavioural change ⇒ update `spec/index.md`.
2. **Pure library.** No IO, no logging, no global state in `src/`
   (excluding `src/main.rs`).
3. **No `unsafe`. No `unwrap`/`expect`/`panic!`** in library code.
4. **Deterministic.** No clocks, RNGs, or environment variables.
5. **Explainability.** Every match returns a per-component breakdown.
6. **Diacritic-correct.**

## What not to do

- Do not short-circuit on owner-scoped (`PlanCode`/`LocalId`) or
  `Custom` schemes — they are not globally unique.
- Do not score a `plan_code` across differing owners.
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
├── lib.rs       public re-exports
├── main.rs      demo binary (not SemVer surface)
├── plan.rs      domain types (Plan, Goal, GoalStatus, PlanType, PlanStatus, IdentifierScheme, PlanIdentifier, PlanRelationship, RelationKind)
├── matcher.rs   MatchingEngine + per-component fns + deterministic rules
├── scoring.rs   MatchResult, MatchBreakdown, Confidence, weighted_average
├── normalize.rs fold, plan_code, fold_set
├── phonetic.rs  Soundex (name component bonus)
├── config.rs    MatchConfig (weights + threshold)
└── error.rs     error type
```
