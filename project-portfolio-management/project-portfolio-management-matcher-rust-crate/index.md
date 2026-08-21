# project-portfolio-management-matcher — documentation index

Pairwise plan record matching, deterministic + probabilistic, for
deduplication across one recursive plan collection. The four former
kinds (Portfolio / Project / Product / Program / Practice / Process /
Purpose / Pathway / Proposal) are unified into one entity; `kind` is
optional descriptive metadata that does not gate matching.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§25). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; public API; layout. |
| [README.md](./README.md) | User-facing intro + usage. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |
| [agents/matching-algorithm.md](./agents/matching-algorithm.md) | No kind gate + per-component derivations + weights. |
| [agents/normalization.md](./agents/normalization.md) | Fold / code rules. |
| [agents/testing.md](./agents/testing.md) | Test layout. |

The entity-level domain model lives in
[`../spec/index.md`](../spec/index.md) §5.

## Worked example

```text
match_plans(
  { name: "Onboarding", identifiers: [JiraProjectKey "ONB"] },
  { name: "Customer onboarding revamp", identifiers: [JiraProjectKey "onb"] },
) -> score 1.0  (R-0 deterministic Jira project key match)

match_plans(
  { kind: Project, name: "Onboarding", identifiers: [JiraProjectKey "ONB"] },
  { kind: Product, name: "Onboarding", identifiers: [JiraProjectKey "onb"] },
) -> score 1.0  (kind is ignored; R-0 still fires despite differing kinds)

match_plans(
  { name: "Customer Onboarding Revamp", goals: ["Reduce time-to-value"] },
  { name: "Onboarding Revamp",          goals: ["Reduce time to value"] },
) -> high score (name + goals corroborate)

match_plans(
  { name: "Q3 Plan v1", owner_org_id: "org-1", code: "PROJ-01" },
  { name: "Q3 Plan v2", owner_org_id: "org-1", code: "proj 01" },
) -> score 1.0  (R-1 same-owner code; codes normalise equal,
                 so differing names are irrelevant)

match_plans(
  { name: "Alpha", same_as: ["https://example.org/plans/42"] },
  { name: "Omega", same_as: ["  https://example.org/plans/42  "] },
) -> score 1.0  (R-2 same_as URL overlap; folding trims the whitespace)
```

### Renormalised partial score (numeric)

When a present component scores `0.0` it *does* pull the average down —
unlike an absent (`None`) component, which is dropped from the divisor.
Consider two plans with identical names but mismatched
parent plans and nothing else:

```text
match_plans(
  { name: "Mobile App Relaunch", parent_ref: "pf-1" },
  { name: "Mobile App Relaunch", parent_ref: "pf-2" },
)
```

Only two components are present:

| Component | Score | Weight |
|-----------|-------|--------|
| name      | 1.00  | 0.30   |
| parent    | 0.00  | 0.08   |

Renormalised over the present weights (§17):

```text
(1.00·0.30 + 0.00·0.08) / (0.30 + 0.08) = 0.30 / 0.38 ≈ 0.789
```

So the pair lands at `~0.79` — `Medium` confidence, below the default
`0.85` threshold. The mismatched parent plan demonstrably drags the
name-only score down from `~1.0`.

Part of the Main X Index family; embedded by
[project-portfolio-management-service](../project-portfolio-management-service-with-loco).
