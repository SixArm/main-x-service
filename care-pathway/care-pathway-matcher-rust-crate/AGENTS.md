# AGENTS.md — Working Guide for AI Coding Agents

Entry point for AI coding agents working in the `care-pathway-matcher`
Rust crate.

## Specification Driven Development

**The `spec/` directory is the source of truth.** 
- Before doing anything read all files inside the `spec/` directory using your file-reading tools.
- Treat all files inside `spec/` as strict requirements. Where any file disagrees with the spec, the spec wins: fix the file, or change the spec on purpose and bring the other files into line in the same commit. 
- Do not improvise structure, style, or citation practice.

## Quick orientation

| Question | Answer |
|---|---|
| What does the crate do? | Pairwise care-pathway (clinical pathway) record matching, deterministic + probabilistic. |
| Canonical spec? | [`spec/index.md`](./spec/index.md). |
| Build / test / lint / fmt | `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt` |
| Run the demo | `cargo run` (`src/main.rs`; not SemVer surface). |
| Public types | `src/lib.rs` re-exports from `src/{care_pathway,matcher,scoring,config,normalize,phonetic,error}.rs`. |
| Deterministic schemes (→ 1.0) | DOI, Wikidata, `GuidelineId`, URI, UUID; plus same-provider pathway-code (R-1) and `same_as` URL overlap (R-2). |
| Probabilistic components | name (Jaro-Winkler + Soundex), condition codes (Jaccard), provider-scoped pathway code, care setting, interventions (Jaccard), keywords (Jaccard). |
| Public API shape | `MatchingEngine::new(MatchConfig::default()).match_care_pathways(&a, &b) -> MatchResult`. |

## Golden rules

1. **Spec-first.** Behavioural change ⇒ update `spec/index.md`.
2. **Pure library.** No IO, no logging, no global state in `src/`
   (excluding `src/main.rs`).
3. **No `unsafe`. No `unwrap`/`expect`/`panic!`** in library code.
4. **Deterministic.** No clocks, RNGs, or environment variables.
5. **Explainability.** Every match returns a per-component breakdown.
6. **Diacritic-correct.**

## What not to do

- Do not short-circuit on provider-scoped (`PathwayCode`/`LocalId`) or
  `Custom` schemes — they are not globally unique.
- Do not score a `pathway_code` across differing providers.
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
├── lib.rs          public re-exports
├── main.rs         demo binary (not SemVer surface)
├── care_pathway.rs domain types (CarePathway, ConditionCode, CodeSystem, CareSetting, IdentifierScheme, PathwayIdentifier)
├── matcher.rs      MatchingEngine + per-component fns + deterministic rules
├── scoring.rs      MatchResult, MatchBreakdown, Confidence, weighted_average
├── normalize.rs    fold, pathway_code, fold_set
├── phonetic.rs     Soundex (name component bonus)
├── config.rs       MatchConfig (weights + threshold)
└── error.rs        error type
```
