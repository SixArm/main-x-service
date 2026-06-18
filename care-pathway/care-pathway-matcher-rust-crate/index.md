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

match_care_pathways(
  { name: "Stroke v1", provider_id: "trust-1", pathway_code: "STROKE-01" },
  { name: "Stroke v2", provider_id: "trust-1", pathway_code: "stroke 01" },
) -> score 1.0  (R-1 same-provider pathway code; codes normalise equal,
                 so differing names are irrelevant)

match_care_pathways(
  { name: "Alpha", same_as: ["https://www.nice.org.uk/guidance/ng128"] },
  { name: "Omega", same_as: ["  https://www.nice.org.uk/guidance/ng128  "] },
) -> score 1.0  (R-2 same_as URL overlap; folding trims the whitespace)
```

### Renormalised partial score (numeric)

When a present component scores `0.0` it *does* pull the average down —
unlike an absent (`None`) component, which is dropped from the divisor.
Consider two records with identical names but a deliberately mismatched
care setting and nothing else:

```text
match_care_pathways(
  { name: "Sepsis Six Pathway", care_setting: Inpatient },
  { name: "Sepsis Six Pathway", care_setting: PrimaryCare },
)
```

Only two components are present:

| Component    | Score | Weight |
|--------------|-------|--------|
| name         | 1.00  | 0.30   |
| care_setting | 0.00  | 0.10   |

Renormalised over the present weights (§17):

```text
(1.00·0.30 + 0.00·0.10) / (0.30 + 0.10) = 0.30 / 0.40 = 0.75
```

So the pair lands at `0.75` — `Medium` confidence, below the default
`0.85` threshold. The mismatched setting demonstrably drags the
name-only score down from `~1.0`.

Part of the Main X Index family; embedded by
[care-pathway-service](../care-pathway-service-with-loco).
