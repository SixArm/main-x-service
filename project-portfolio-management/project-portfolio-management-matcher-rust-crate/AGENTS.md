# AGENTS.md — Working Guide for AI Coding Agents

Entry point for AI coding agents working in the `project-portfolio-management-matcher` Rust
crate.

> If you only read one file, read [`spec/index.md`](./spec/index.md) —
> the living specification.

## Quick orientation

| Question | Answer |
|---|---|
| What does the crate do? | Pairwise plan record matching, deterministic + probabilistic, for dedup. The four former kinds (Portfolio / Project / Product / Program / Practice / Process / Purpose / Pathway / Proposal) are unified into one recursive plan tree; `kind` is optional descriptive metadata and does not gate matching. |
| Canonical spec? | [`spec/index.md`](./spec/index.md). |
| Entity domain model? | [`../spec/index.md`](../spec/index.md) §5 (entity-level umbrella). |
| Build / test / lint / fmt | `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt` |
| Run the demo | `cargo run` (`src/main.rs`; not SemVer surface). |
| Public types | `src/lib.rs` re-exports from `src/{plan,matcher,scoring,config,normalize,phonetic,error}.rs`. |
| Match gate | **None.** The former kind gate was removed; any two plans may match. `MatchBreakdown.kind_gate_blocked` is vestigial (always `false`). |
| Deterministic schemes (→ 1.0) | URI, UUID, Jira project key, Asana GID, Trello board id, MS Project id, GitHub project id, Linear id; plus same-owner code (R-1) and `same_as` URL overlap (R-2). |
| Probabilistic components | name (Jaro-Winkler + Soundex), goals (Jaccard over titles), owner-scoped code, owner org, parent (parent plan exact via `parent_ref`), timeframe (Gaussian decay), keywords (Jaccard), relationships (typed-set Jaccard), tags (set Jaccard). |
| Public API shape | `MatchingEngine::new(MatchConfig::default()).match_plans(&a, &b) -> MatchResult`. `Plan::new(name)` (kind defaults `None`). |

## Golden rules

1. **Spec-first.** Behavioural change ⇒ update `spec/index.md`.
2. **No kind gate.** The four kinds were unified into one recursive
   plan tree; `kind` is optional descriptive metadata and never
   gates matching. Any two plans may match.
3. **Pure library.** No IO, no logging, no global state in `src/`
   (excluding `src/main.rs`).
4. **No `unsafe`. No `unwrap`/`expect`/`panic!`** in library code.
5. **Deterministic.** No clocks, RNGs, or environment variables.
6. **Explainability.** Every match returns a per-component breakdown.
7. **Diacritic-correct.**

## What not to do

- Do not reintroduce a kind gate — `kind` is descriptive metadata, and
  two plans with different kinds may still be the same identity.
- Do not short-circuit on owner-scoped (`Code`/`LocalId`) or `Custom`
  schemes — they are not globally unique.
- Do not score a `code` across differing owners.
- Do not match on `status` (it drifts between duplicate records).
- Do not change default weights/threshold without updating
  `spec/index.md §7` and `CHANGELOG.md`.

## Detailed guides

- [agents/matching-algorithm.md](./agents/matching-algorithm.md)
- [agents/normalization.md](./agents/normalization.md)
- [agents/spec-driven-development.md](./agents/spec-driven-development.md)
- [agents/testing.md](./agents/testing.md)

## File layout

```
src/
├── lib.rs         public re-exports
├── main.rs        demo binary (not SemVer surface)
├── plan.rs   domain types (Plan, PlanKind [optional label], Goal, GoalStatus, PlanStatus, IdentifierScheme, PlanIdentifier, PlanRelationship, RelationKind)
├── matcher.rs     MatchingEngine + per-component fns + deterministic rules (no kind gate)
├── scoring.rs     MatchResult, MatchBreakdown, Confidence, weighted_average
├── normalize.rs   fold, code, url, fold_set, iso_date_to_days
├── phonetic.rs    Soundex (name component bonus)
├── config.rs      MatchConfig (weights + threshold)
└── error.rs       error type
```
