# plan-matcher — documentation index

Pairwise plan (project / product / programme / initiative / portfolio /
epic) record matching, deterministic + probabilistic, for portfolio
deduplication.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§25). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; public API; layout. |
| [README.md](./README.md) | User-facing intro + usage. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |
| [AGENTS/matching-algorithm.md](./AGENTS/matching-algorithm.md) | Per-component derivations + weights. |
| [AGENTS/normalization.md](./AGENTS/normalization.md) | Fold / plan-code rules. |
| [AGENTS/testing.md](./AGENTS/testing.md) | Test layout. |

## Worked example

```text
match_plans(
  { name: "Onboarding", identifiers: [JiraProjectKey "ONB"] },
  { name: "Customer onboarding revamp", identifiers: [JiraProjectKey "onb"] },
) -> score 1.0  (R-0 deterministic Jira project key match)

match_plans(
  { name: "Customer Onboarding Revamp", goals: ["Reduce time-to-value"] },
  { name: "Onboarding Revamp",          goals: ["Reduce time to value"] },
) -> high score (name + goals corroborate)

match_plans(
  { name: "Q3 Plan v1", owner_org_id: "org-1", plan_code: "PLAN-01" },
  { name: "Q3 Plan v2", owner_org_id: "org-1", plan_code: "plan 01" },
) -> score 1.0  (R-1 same-owner plan code; codes normalise equal,
                 so differing names are irrelevant)

match_plans(
  { name: "Alpha", same_as: ["https://example.org/plans/42"] },
  { name: "Omega", same_as: ["  https://example.org/plans/42  "] },
) -> score 1.0  (R-2 same_as URL overlap; folding trims the whitespace)
```

### Renormalised partial score (numeric)

When a present component scores `0.0` it *does* pull the average down —
unlike an absent (`None`) component, which is dropped from the divisor.
Consider two records with identical names but a deliberately mismatched
plan type and nothing else:

```text
match_plans(
  { name: "Mobile App Relaunch", plan_type: Project },
  { name: "Mobile App Relaunch", plan_type: Programme },
)
```

Only two components are present:

| Component | Score | Weight |
|-----------|-------|--------|
| name      | 1.00  | 0.30   |
| plan_type | 0.00  | 0.08   |

Renormalised over the present weights (§17):

```text
(1.00·0.30 + 0.00·0.08) / (0.30 + 0.08) = 0.30 / 0.38 ≈ 0.789
```

So the pair lands at `~0.79` — `Medium` confidence, below the default
`0.85` threshold. The mismatched type demonstrably drags the name-only
score down from `~1.0`.

Part of the Main X Index family; embedded by
[plan-service](../plan-service-with-loco).
