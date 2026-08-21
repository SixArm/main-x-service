# Matching algorithm — project-portfolio-management-matcher

## Strategies

- **No kind gate** — the four former kinds were unified into one
  recursive plan collection, so any two plans may match regardless of
  their (optional) `kind`, which is never compared.
- **Deterministic** — three short-circuit rules pin the score to 1.0.
- **Probabilistic** — weighted average over the components both records
  carry; absent fields don't penalise.

## Kind — no gate

`kind` (`Portfolio`, `Project`, `Product`, `Program`, `Practice`,
`Process`, `Purpose`, `Pathway`, `Proposal`) is **optional descriptive
metadata** — a display / grouping label. It does **not** gate matching
and is never compared: two plans with different kinds may still be the
same identity. All plans live in one recursive collection (`plans`) and
the matcher compares any two of them. The
`MatchBreakdown.kind_gate_blocked` field is retained but vestigial
(always `false`). The removed gate replaces the `plan_type` weighted
component the ancestor `plan-matcher` carried — `kind` is now neither a
gate nor a weight.

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
| Parent | 0.08 | Any plan with a containment link: same parent `parent_ref` (case-folded) → 1.0 else 0.0. `None` if either unset (e.g. a root plan). |
| Timeframe | 0.07 | Date proximity over `start_date` / `target_date` by Gaussian decay `exp(-(Δdays/σ)²/2)`, σ default 90 days, averaged over comparable date pairs. `None` if no comparable pair. |
| Keywords | 0.05 | Jaccard on `fold_set(keywords)`. |
| Relationships | 0.05 | Typed-set Jaccard over `(relation, plan_id)` pairs. `None` if either side empty. Supporting signal. |
| Tags | 0.05 | Set Jaccard over `fold`-normalised tags. `None` if either side empty. Supporting signal. |

Weights sum to 1.0; the weighted average is renormalised over the
`Some` components only. `kind` is optional descriptive metadata (never
scored, never a gate). `status`, `owner_org_name`, `lead_ref`, and
per-goal `description` / `target_date` / `status` are informational-only
and never scored.

## Confidence band

`High` ≥ 0.95, `Medium` ≥ 0.70, `Low` < 0.70 — separate from
`MatchConfig::threshold` (`is_match`, default 0.85).

## Worked example

```rust
use project_portfolio_management_matcher::{Plan, IdentifierScheme, PlanIdentifier, MatchingEngine};

let engine = MatchingEngine::default_config();
let mut a = Plan::new("Onboarding");
let mut b = Plan::new("Customer onboarding revamp");
a.identifiers.push(PlanIdentifier { scheme: IdentifierScheme::JiraProjectKey, value: "ONB".into() });
b.identifiers.push(PlanIdentifier { scheme: IdentifierScheme::JiraProjectKey, value: "onb".into() });
let r = engine.match_plans(&a, &b);
assert_eq!(r.score, 1.0);                       // R-0 fires
assert!(r.breakdown.deterministic_match);

// `kind` never gates: two plans with different (optional) kinds still
// match on their shared signals.
use project_portfolio_management_matcher::PlanKind;
a.kind = Some(PlanKind::Project);
b.kind = Some(PlanKind::Product);
let r2 = engine.match_plans(&a, &b);
assert_eq!(r2.score, 1.0);                      // R-0 still fires; kind ignored
```
