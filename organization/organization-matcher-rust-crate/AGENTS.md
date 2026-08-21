# AGENTS.md — Working Guide for AI Coding Agents

Entry point for AI coding agents working in the `organization-matcher`
Rust crate.

> If you only read one file, read [`spec/index.md`](./spec/index.md) —
> the living specification. This guide tells you **how to work**; the
> spec tells you **what to build**.

## Quick orientation

| Question | Answer |
|---|---|
| What does the crate do? | Pairwise organization-record matching (deterministic + probabilistic) per [schema.org/Organization](https://schema.org/Organization). |
| Canonical spec? | [`spec/index.md`](./spec/index.md). |
| Build / test / lint / fmt | `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt` |
| Run the demo | `cargo run` (`src/main.rs`; illustrative, not SemVer surface). |
| Public types | `src/lib.rs` re-exports from `src/{organization,matcher,scoring,config,normalize,phonetic,error}.rs`. |
| Deterministic schemes (short-circuit to 1.0) | LEI, DUNS, ISO 6523, GLN, Wikidata, ROR, ISNI, VAT; plus same-jurisdiction tax-id (R-1) and `same_as` URL overlap (R-2). |
| Probabilistic components | name (legal-suffix-aware JW + Soundex), address (field-by-field), url/domain, jurisdiction, founding date, keywords (Jaccard). |
| Public API shape | `MatchingEngine::new(MatchConfig::default()).match_organizations(&a, &b) -> MatchResult { score, confidence, is_match, breakdown }`. |

## Golden rules

1. **Spec-first.** Behavioural change ⇒ update `spec/index.md` in the
   same change.
2. **Pure library.** No IO, no logging, no global state in `src/`
   (excluding `src/main.rs`, the demo binary).
3. **No `unsafe`. No `unwrap`/`expect`/`panic!`** in library code.
4. **Deterministic.** No clocks, RNGs, or environment variables.
5. **Explainability.** Every probabilistic match returns a per-field
   breakdown.
6. **Diacritic-correct.** `Müller` ≠ `Muller`.

## What not to do

- Do not short-circuit on classification codes (`Naics`/`IsicV4`/`Sic`)
  or `Custom` — they are not globally unique identifiers.
- Do not score a `TaxId` across differing jurisdictions (spec §15–§16).
- Do not change default weights/threshold without updating
  `spec/index.md §7` and `CHANGELOG.md`.
- Do not strip diacritics in normalisation.

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
├── organization.rs domain types (Organization, OrgIdentifier, IdentifierScheme, PostalAddress)
├── matcher.rs      MatchingEngine + per-component fns + deterministic rules
├── scoring.rs      MatchResult, MatchBreakdown, Confidence, weighted_average
├── normalize.rs    fold, legal_name, domain, fold_set
├── phonetic.rs     Soundex (name component bonus)
├── config.rs       MatchConfig (weights + threshold)
└── error.rs        error type
```
