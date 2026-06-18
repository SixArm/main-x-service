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
`Some` components only. `priority`, `opened_date`, and `in_language`
are **never scored** — they are carried for downstream consumers.

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

### Soundex phonetic title bonus (§9)

When two primary titles are spelled differently but sound alike, the
literal Jaro-Winkler score is lifted by `+0.05` (capped at `0.95`) if
the two share a Soundex code:

```rust
use case_matcher::{Case, MatchingEngine};

let engine = MatchingEngine::default_config();
// "Smith" and "Smyth" both encode to Soundex S530.
let a = Case::new("Smith");
let b = Case::new("Smyth");
let r = engine.match_cases(&a, &b);
// The phonetic bonus nudges the title score upward, but the cap keeps
// the phonetic-only path strictly below a "certain" (≥0.95) title match.
let title = r.breakdown.title_score.expect("title always present");
assert!(title <= 0.95);
```

### Renormalisation + threshold presets (§17, §7)

Absent components drop out of the average entirely (they neither add to
the numerator nor the denominator); the presets shift only the
`is_match` threshold, never the `score`:

```rust
use case_matcher::{Case, MatchConfig, MatchingEngine};

// Only title + subjects are present; case_number / type / status /
// keywords are all None and renormalise away.
let mut a = Case::new("Disability benefit appeal");
let mut b = Case::new("Disability benefit review appeal");
a.subjects = vec!["person:7".into()];
b.subjects = vec!["person:7".into()];

let default = MatchingEngine::new(MatchConfig::default()).match_cases(&a, &b);
let strict = MatchingEngine::new(MatchConfig::strict()).match_cases(&a, &b);
let lenient = MatchingEngine::new(MatchConfig::lenient()).match_cases(&a, &b);

// Same score under every preset — only the threshold (and therefore
// is_match) differs.
assert!((default.score - strict.score).abs() < 1e-9);
assert!((default.score - lenient.score).abs() < 1e-9);
assert!(default.breakdown.case_number_score.is_none());   // renormalised away
```
