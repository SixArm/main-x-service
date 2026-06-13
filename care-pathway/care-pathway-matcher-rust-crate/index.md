# care-pathway-matcher — documentation index

Pairwise care-pathway (clinical pathway) record matching, deterministic
+ probabilistic.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§25). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; public API; layout. |
| [README.md](./README.md) | User-facing intro + usage. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |
| [AGENTS/matching-algorithm.md](./AGENTS/matching-algorithm.md) | Per-component derivations + weights. |
| [AGENTS/normalization.md](./AGENTS/normalization.md) | Fold / pathway-code rules. |
| [AGENTS/testing.md](./AGENTS/testing.md) | Test layout. |

## Worked example

```text
match_care_pathways(
  { name: "Stroke", identifiers: [GuidelineId "NICE-NG128"] },
  { name: "Cerebrovascular accident pathway", identifiers: [GuidelineId "nice-ng128"] },
) -> score 1.0  (R-0 deterministic guideline-id match)

match_care_pathways(
  { name: "Acute Stroke Care Pathway", condition_codes: [Icd10 "I63"] },
  { name: "Acute Stroke Pathway",      condition_codes: [Icd10 "I63"] },
) -> high score (name + condition corroborate)
```

Part of the Main X Index family; embedded by
[care-pathway-service](../care-pathway-service-rust-crate).
