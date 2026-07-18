# Matching algorithm — project-portfolio-management-matcher

## Strategies

- **Gate (R-GATE)** — a kind mismatch (`A.kind != B.kind`) short-circuits
  to `0.0` *before* any other rule. Matching is within-kind only.
- **Deterministic** — three short-circuit rules pin the score to 1.0
  (all evaluated only after R-GATE passes).
- **Probabilistic** — weighted average over the components both records
  carry; absent fields don't penalise.

## R-GATE — the kind gate

| Rule | Condition | Result |
|---|---|---|
| **R-GATE** | `A.kind != B.kind` | `0.0` (no match) — runs first, before R-0/R-1/R-2 and every component. |

The four kinds (`Portfolio`, `Project`, `Product`, `Program`) map to
four distinct service collections/tables. A project is never a product;
the service never asks the matcher to compare across collections, and if
it did, R-GATE refuses it. When the kinds agree the gate is transparent
and matching proceeds. R-GATE replaces the `plan_type` weighted
component the ancestor `plan-matcher` carried (kind is a gate, not a
weight).

## Deterministic short-circuits

| Rule | Condition |
|---|---|
| **R-0** | Both records share a value on a *deterministic* identifier scheme. |
| **R-1** | Both share `owner_org_id` AND normalised `code`. |
| **R-2** | A `same_as` URL overlaps (case-folded). |

Deterministic schemes (`IdentifierScheme::is_deterministic`): `Uri`,
`Uuid`, `JiraProjectKey`, `AsanaGid`, `TrelloBoardId`, `MsProjectId`,
`GitHubProjectId`, `LinearId`. NOT deterministic: `Code` / `LocalId`
(owner-scoped) and `Custom`.

## Probabilistic components

| Component | Default weight | Algorithm |
|---|---|---|
| Name | 0.30 | Best Jaro-Winkler over `name` + `alternate_names`, + Soundex +0.05 bonus capped at 0.95. |
| Goals | 0.15 | Jaccard over `fold_set` of goal **titles**. Skipped if both empty. |
| Code | 0.15 | Within same `owner_org_id`: 1.0 if normalised codes equal else 0.0. Across owners: `None`. |
| Owner org | 0.10 | `owner_org_id` case-folded exact → 1.0 else 0.0. `None` if either unset. |
| Portfolio | 0.08 | Child kinds only: same parent `portfolio_ref` (case-folded) → 1.0 else 0.0. `None` if either unset (always so for the Portfolio kind). |
| Timeframe | 0.07 | Date proximity over `start_date` / `target_date` by Gaussian decay `exp(-(Δdays/σ)²/2)`, σ default 90 days, averaged over comparable date pairs. `None` if no comparable pair. |
| Keywords | 0.05 | Jaccard on `fold_set(keywords)`. |
| Relationships | 0.05 | Typed-set Jaccard over `(relation, work_item_id)` pairs. `None` if either side empty. Supporting signal. |
| Tags | 0.05 | Set Jaccard over `fold`-normalised tags. `None` if either side empty. Supporting signal. |

Weights sum to 1.0; the weighted average is renormalised over the
`Some` components only. `kind` is the gate (not scored). `status`,
`owner_org_name`, `lead_ref`, and per-goal `description` /
`target_date` / `status` are informational-only and never scored.

## Confidence band

`High` ≥ 0.95, `Medium` ≥ 0.70, `Low` < 0.70 — separate from
`MatchConfig::threshold` (`is_match`, default 0.85). A kind mismatch
(R-GATE) is `Low` at score `0.0`.

## Worked example

```rust
use project_portfolio_management_matcher::{WorkItem, WorkItemKind, IdentifierScheme, WorkItemIdentifier, MatchingEngine};

let engine = MatchingEngine::default_config();
let mut a = WorkItem::new(WorkItemKind::Project, "Onboarding");
let mut b = WorkItem::new(WorkItemKind::Project, "Customer onboarding revamp");
a.identifiers.push(WorkItemIdentifier { scheme: IdentifierScheme::JiraProjectKey, value: "ONB".into() });
b.identifiers.push(WorkItemIdentifier { scheme: IdentifierScheme::JiraProjectKey, value: "onb".into() });
let r = engine.match_work_items(&a, &b);
assert_eq!(r.score, 1.0);                       // R-0 fires (kinds agree)
assert!(r.breakdown.deterministic_match);

// Cross-kind never matches:
let p = WorkItem::new(WorkItemKind::Product, "Onboarding");
let r2 = engine.match_work_items(&a, &p);
assert_eq!(r2.score, 0.0);                      // R-GATE fires
```
