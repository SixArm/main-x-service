# Matching algorithm reference — Thing Service

## Pipeline

```
Input: Thing A, Thing B, MatchWeights
  │
  ├─ Deterministic identifier match? ──yes──> Return 1.0 (Certain)
  │   (DOI, ISBN, ISSN, GTIN, MPN, SerialNumber, UUID)
  │
  ├─ Name Score ─────── Jaro-Winkler (case-insensitive)
  ├─ Identifier Score ─ Exact (property_id, value) match
  ├─ Description Score ─ Jaro-Winkler on description (case-insensitive)
  ├─ URL Score ──────── Host + path equality (scheme/case normalized)
  ├─ SameAs Score ───── Best URL pair across same_as lists
  ├─ Phonetic Check ─── Soundex on name
  │
  ├─ Weighted Average (only available components)
  ├─ Phonetic Bonus (+5% if name Soundex matches and score < 0.95)
  │
  └─ Return MatchResult { score, confidence, breakdown }
```

## Component algorithms

### Name matching (`matching::name`)

- **Algorithm**: Jaro-Winkler similarity.
- **Range**: 0.0–1.0.
- **Case**: case-insensitive.
- **Empty handling**: both empty → 1.0; one empty → 0.0.
- **Prefix bonus**: Jaro-Winkler gives a small bonus for shared
  leading characters.

### Identifier matching (`matching::identifier`)

- **Algorithm**: best-pair exact match on `(property_id, value)`
  across both identifier lists.
- **Output**: 1.0 if any pair matches, 0.0 otherwise.
- **Deterministic short-circuit**: `has_deterministic_match` returns
  `true` if any pair of *deterministic* identifiers (DOI, ISBN, ISSN,
  GTIN, MPN, SerialNumber, UUID) matches, pinning the final score at
  1.0 regardless of other component scores.

### Description matching (`matching::description`)

- **Algorithm**: Jaro-Winkler (case-insensitive).
- **Range**: 0.0–1.0.

### URL matching (`matching::url`)

- **Algorithm**: scheme- and case-normalized host + path comparison.
- **Output**: 1.0 if normalized URLs are identical; 0.75 if hosts
  match but paths differ; 0.0 otherwise.
- **List variant**: `url_list_similarity(a, b)` returns the best pair
  score across two URL lists — used for `same_as`.

### Phonetic matching (`matching::phonetic`)

- **Algorithm**: 4-character Soundex code on the name.
- **Usage**: applied as a bonus to the final score, not as a
  standalone component.
- **Bonus**: +0.05 if Soundex codes match and the base score is below
  0.95.

## Scoring (`matching::scoring`)

### Default weights

| Component | Weight |
|-----------|-------:|
| Name | 0.40 |
| Identifier | 0.30 |
| Description | 0.10 |
| URL | 0.10 |
| Same-as | 0.10 |

Weights sum to 1.0. Only components for which both Things have data
contribute to the weighted average.

### Confidence levels

| Level | Score range | Meaning |
|-------|-------------|---------|
| Certain | ≥ 0.95 | Definite match |
| Probable | ≥ 0.80 | Likely match |
| Possible | ≥ 0.60 | Potential match |
| Unlikely | < 0.60 | Not a match |

### MatchResult

```rust
pub struct MatchResult {
    pub score: f64,
    pub confidence: MatchConfidence,
    pub breakdown: MatchBreakdown,
}

pub struct MatchBreakdown {
    pub name_score: f64,
    pub identifier_score: f64,
    pub description_score: f64,
    pub url_score: f64,
    pub same_as_score: f64,
    pub phonetic_match: bool,
    pub deterministic_match: bool,
}
```

## Usage example

```rust
use thing_service::matching::scoring::{compute_match, MatchWeights, MatchConfidence};
use thing_service::models::identifier::ThingIdentifier;
use thing_service::models::thing::Thing;

let mut a = Thing::new("Pride and Prejudice");
a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];

let mut b = Thing::new("Stolz und Vorurteil");
b.identifiers = vec![ThingIdentifier::isbn("9780141439518")];

let result = compute_match(&a, &b, &MatchWeights::default());
println!("Score: {}", result.score);              // 1.00 (deterministic)
println!("Confidence: {:?}", result.confidence);  // Certain
println!("Deterministic: {}", result.breakdown.deterministic_match);
```
