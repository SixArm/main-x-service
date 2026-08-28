# Matching algorithm — care-pathway-matcher

## Strategies

- **Deterministic** — three short-circuit rules pin the score to 1.0.
- **Probabilistic** — weighted average over the components both records
  carry; absent fields don't penalise.

## Deterministic short-circuits

| Rule | Condition |
|---|---|
| **R-0** | Both records share a value on a *deterministic* identifier scheme. |
| **R-1** | Both share `provider_id` AND normalised `pathway_code`. |
| **R-2** | A `same_as` URL overlaps (case-folded). |

Deterministic schemes (`IdentifierScheme::is_deterministic`): `Doi`,
`Wikidata`, `GuidelineId`, `Uri`, `Uuid`. NOT deterministic:
`PathwayCode` / `LocalId` (provider-scoped) and `Custom`.

## Probabilistic components

| Component | Default weight | Algorithm |
|---|---|---|
| Name | 0.30 | Best Jaro-Winkler over `name` + `alternate_names`, + Soundex +0.05 bonus capped at 0.95. |
| Condition codes | 0.25 | Jaccard over `"system:code"` tokens (ICD-10 / ICD-11 / SNOMED / custom). The defining attribute. Skipped if both empty. |
| Pathway code | 0.15 | Within same `provider_id`: 1.0 if normalised codes equal else 0.0. Across providers: `None`. |
| Care setting | 0.10 | Exact enum match → 1.0 else 0.0. `None` if either unset. |
| Interventions | 0.10 | Jaccard on `fold_set(interventions)`. |
| Keywords | 0.10 | Jaccard on `fold_set(keywords)`. |
| Relationships | 0.05 | Typed-set Jaccard over `(relation, pathway_id)` pairs — `\|A ∩ B\| / \|A ∪ B\|` — see spec §13.1. `None` when either side's `relationships` list is empty. Supporting signal, never resolved against a registry. |
| Tags | 0.05 | Set Jaccard over case-insensitively normalised tag sets — `\|A ∩ B\| / \|A ∪ B\|` — see spec §13.2. `None` when either side's `tags` list is empty. Normalisation happens at scoring time, not on construction. |

The core six weights sum to 1.0; relationships/tags are two further
**supporting** signals that layer on top at 0.05 each. The weighted
average is renormalised over the `Some` components only, so their
presence never changes the score of records that never populate them.

## Confidence band

`High` ≥ 0.95, `Medium` ≥ 0.70, `Low` < 0.70 — separate from
`MatchConfig::threshold` (`is_match`, default 0.85).

## Worked example

```rust
use care_pathway_matcher::{CarePathway, IdentifierScheme, PathwayIdentifier, MatchingEngine};

let engine = MatchingEngine::default_config();
let mut a = CarePathway::new("Stroke");
let mut b = CarePathway::new("Cerebrovascular accident pathway");
a.identifiers.push(PathwayIdentifier { scheme: IdentifierScheme::GuidelineId, value: "NICE-NG128".into() });
b.identifiers.push(PathwayIdentifier { scheme: IdentifierScheme::GuidelineId, value: "nice-ng128".into() });
let r = engine.match_care_pathways(&a, &b);
assert_eq!(r.score, 1.0);                       // R-0 fires
assert!(r.breakdown.deterministic_match);
```
