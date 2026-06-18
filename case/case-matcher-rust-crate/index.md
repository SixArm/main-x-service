# case-matcher — documentation index

Pairwise governmental case-management record matching, deterministic +
probabilistic.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§25). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; public API; layout. |
| [README.md](./README.md) | User-facing intro + usage. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |
| [AGENTS/matching-algorithm.md](./AGENTS/matching-algorithm.md) | Per-component derivations + weights. |
| [AGENTS/normalization.md](./AGENTS/normalization.md) | Fold / case-number / url rules. |
| [AGENTS/testing.md](./AGENTS/testing.md) | Test layout. |

## Worked example

```text
match_cases(
  { title: "Smith v. Housing Authority", identifiers: [Docket "CV-2024-001234"] },
  { title: "Appeal of benefit denial",   identifiers: [Docket "cv-2024-001234"] },
) -> score 1.0  (R-0 deterministic docket match)

match_cases(
  { title: "Housing benefit appeal — J. Smith",   subjects: ["person:pid-42"] },
  { title: "Housing benefit appeal — John Smith", subjects: ["person:pid-42"] },
) -> high score (title + subjects corroborate)

# Renormalisation over partial components (§17): only title + subjects
# are present, so case_number / type / status / keywords drop out of the
# average entirely (their weights are NOT counted in the divisor).
match_cases(
  { title: "Child protection referral — Doe family", subjects: ["person:99"] },
  { title: "Child protection referral — Doe family", subjects: ["person:99"] },
) -> ~1.0  (divisor = title_weight + subjects_weight only)

# Soundex phonetic title bonus (§9): "Smith" / "Smyth" share Soundex
# S530, so the title component is nudged +0.05 (capped at 0.95) above the
# literal Jaro-Winkler — a weak corroborating signal, never decisive.
match_cases({ title: "Smith" }, { title: "Smyth" }) -> title_score ≤ 0.95
```

The `score` is preset-independent: `MatchConfig::strict()` (threshold
0.95) and `MatchConfig::lenient()` (0.70) change only `is_match`, never
the computed `score`.

Part of the Main X Index family; embedded by
[case-service](../case-service-with-loco).
