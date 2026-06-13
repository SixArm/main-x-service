# Matching algorithm — case-matcher

## Strategies

- **Deterministic** — three short-circuit rules pin the score to 1.0.
- **Probabilistic** — weighted average over the components both records
  carry; absent fields don't penalise.

## Deterministic short-circuits

| Rule | Condition |
|---|---|
| **R-0** | Both records share a value on a *deterministic* identifier scheme. |
| **R-1** | Both share an agency key (`agency_id`, or `agency_name` fallback) AND normalised `case_number`. |
| **R-2** | A `same_as` URL overlaps (case-folded). |

Deterministic schemes (`IdentifierScheme::is_deterministic`): `Docket`,
`ExternalCaseId`, `Uri`, `Uuid`. NOT deterministic:
`AgencyCaseNumber` / `LocalId` (agency-scoped) and `Custom`.

## Probabilistic components

| Component | Default weight | Algorithm |
|---|---|---|
| Title | 0.30 | Best Jaro-Winkler over `title` + `alternate_titles`, + Soundex +0.05 bonus capped at 0.95. |
| Subjects | 0.25 | Jaccard over folded involved-party id strings. A strong signal that two records describe the same matter. Skipped if both empty. |
| Case number | 0.15 | Within the same agency key: 1.0 if normalised numbers equal else 0.0. Across agencies: `None`. |
| Case type | 0.10 | Exact enum match → 1.0 else 0.0. `None` if either unset. |
| Status | 0.05 | Exact enum match → 1.0 else 0.0. `None` if either unset. |
| Keywords | 0.15 | Jaccard on `fold_set(keywords)`. |

Weights sum to 1.0; the weighted average is renormalised over the
`Some` components only. `priority` and `opened_date` are **never
scored** — they are carried for downstream consumers.

## Confidence band

`High` ≥ 0.95, `Medium` ≥ 0.70, `Low` < 0.70 — separate from
`MatchConfig::threshold` (`is_match`, default 0.85).

## Worked example

```rust
use case_matcher::{Case, IdentifierScheme, CaseIdentifier, MatchingEngine};

let engine = MatchingEngine::default_config();
let mut a = Case::new("Smith v. Housing Authority");
let mut b = Case::new("Appeal of benefit denial");
a.identifiers.push(CaseIdentifier { scheme: IdentifierScheme::Docket, value: "CV-2024-001234".into() });
b.identifiers.push(CaseIdentifier { scheme: IdentifierScheme::Docket, value: "cv-2024-001234".into() });
let r = engine.match_cases(&a, &b);
assert_eq!(r.score, 1.0);                       // R-0 fires
assert!(r.breakdown.deterministic_match);
```
