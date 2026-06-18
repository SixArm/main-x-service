# AGENTS.md — Working Guide for AI Coding Agents

Entry point for AI coding agents working in the `case-matcher` Rust
crate.

> If you only read one file, read [`spec/index.md`](./spec/index.md) —
> the living specification.

## Quick orientation

| Question | Answer |
|---|---|
| What does the crate do? | Pairwise governmental case-management record matching, deterministic + probabilistic. |
| Canonical spec? | [`spec/index.md`](./spec/index.md). |
| Build / test / lint / fmt | `cargo build` · `cargo test` · `cargo clippy --all-targets --all-features -- -D warnings` · `cargo fmt` |
| Run the demo | `cargo run` (`src/main.rs`; not SemVer surface). |
| Public types | `src/lib.rs` re-exports from `src/{case,matcher,scoring,config,normalize,phonetic,error}.rs`. |
| Deterministic schemes (→ 1.0) | `Docket`, `ExternalCaseId`, URI, UUID; plus same-agency case-number (R-1) and `same_as` URL overlap (R-2). |
| Probabilistic components | title (Jaro-Winkler + Soundex), subjects (Jaccard), agency-scoped case number, case type, status, keywords (Jaccard). |
| Never scored | `priority`, `opened_date`, `in_language` (carried as data only). |
| Public API shape | `MatchingEngine::new(MatchConfig::default()).match_cases(&a, &b) -> MatchResult`. |

## Golden rules

1. **Spec-first.** Behavioural change ⇒ update `spec/index.md`.
2. **Pure library.** No IO, no logging, no global state in `src/`
   (excluding `src/main.rs`).
3. **No `unsafe`. No `unwrap`/`expect`/`panic!`** in library code.
4. **Deterministic.** No clocks, RNGs, or environment variables.
5. **Explainability.** Every match returns a per-component breakdown.
6. **Diacritic-correct.**

## What not to do

- Do not short-circuit on agency-scoped (`AgencyCaseNumber`/`LocalId`)
  or `Custom` schemes — they are not globally unique.
- Do not score a `case_number` across differing agencies.
- Do not score `priority`, `opened_date`, or `in_language` — they are
  data only.
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
├── lib.rs          public re-exports
├── main.rs         demo binary (not SemVer surface)
├── case.rs         domain types (Case, CaseType, CaseStatus, Priority, IdentifierScheme, CaseIdentifier)
├── matcher.rs      MatchingEngine + per-component fns + deterministic rules
├── scoring.rs      MatchResult, MatchBreakdown, Confidence, weighted_average
├── normalize.rs    fold, case_number, url, fold_set
├── phonetic.rs     Soundex (title component bonus)
├── config.rs       MatchConfig (weights + threshold)
└── error.rs        error type
```
