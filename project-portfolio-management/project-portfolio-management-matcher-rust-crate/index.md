# project-portfolio-management-matcher — documentation index

Pairwise work-item (Portfolio / Project / Product / Program) record
matching, kind-gated + deterministic + probabilistic, for
within-collection deduplication.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§25). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; public API; layout. |
| [README.md](./README.md) | User-facing intro + usage. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |
| [AGENTS/matching-algorithm.md](./AGENTS/matching-algorithm.md) | R-GATE + per-component derivations + weights. |
| [AGENTS/normalization.md](./AGENTS/normalization.md) | Fold / code rules. |
| [AGENTS/testing.md](./AGENTS/testing.md) | Test layout. |

The entity-level domain model lives in
[`../spec/index.md`](../spec/index.md) §5.

## Worked example

```text
match_work_items(
  { kind: Project, name: "Onboarding", identifiers: [JiraProjectKey "ONB"] },
  { kind: Project, name: "Customer onboarding revamp", identifiers: [JiraProjectKey "onb"] },
) -> score 1.0  (R-0 deterministic Jira project key match; kinds agree)

match_work_items(
  { kind: Project, name: "Onboarding" },
  { kind: Product, name: "Onboarding" },
) -> score 0.0  (R-GATE: different kind, distinct collections)

match_work_items(
  { kind: Program, name: "Customer Onboarding Revamp", goals: ["Reduce time-to-value"] },
  { kind: Program, name: "Onboarding Revamp",          goals: ["Reduce time to value"] },
) -> high score (name + goals corroborate)

match_work_items(
  { kind: Project, name: "Q3 Plan v1", owner_org_id: "org-1", code: "PROJ-01" },
  { kind: Project, name: "Q3 Plan v2", owner_org_id: "org-1", code: "proj 01" },
) -> score 1.0  (R-1 same-owner code; codes normalise equal,
                 so differing names are irrelevant)

match_work_items(
  { kind: Portfolio, name: "Alpha", same_as: ["https://example.org/portfolios/42"] },
  { kind: Portfolio, name: "Omega", same_as: ["  https://example.org/portfolios/42  "] },
) -> score 1.0  (R-2 same_as URL overlap; folding trims the whitespace)
```

### Renormalised partial score (numeric)

When a present component scores `0.0` it *does* pull the average down —
unlike an absent (`None`) component, which is dropped from the divisor.
Consider two **Project** children with identical names but mismatched
parent portfolios and nothing else:

```text
match_work_items(
  { kind: Project, name: "Mobile App Relaunch", portfolio_ref: "pf-1" },
  { kind: Project, name: "Mobile App Relaunch", portfolio_ref: "pf-2" },
)
```

Only two components are present:

| Component | Score | Weight |
|-----------|-------|--------|
| name      | 1.00  | 0.30   |
| portfolio | 0.00  | 0.08   |

Renormalised over the present weights (§17):

```text
(1.00·0.30 + 0.00·0.08) / (0.30 + 0.08) = 0.30 / 0.38 ≈ 0.789
```

So the pair lands at `~0.79` — `Medium` confidence, below the default
`0.85` threshold. The mismatched parent portfolio demonstrably drags the
name-only score down from `~1.0`.

Part of the Main X Index family; embedded by
[project-portfolio-management-service](../project-portfolio-management-service-with-loco).
