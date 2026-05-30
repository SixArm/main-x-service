# Matching algorithm reference — Main Thing Service

## Pipeline

```
Input: Thing A, Thing B, MatchWeights
  │
  ├─ GLN Match? ──yes──> Return 1.0 (Certain, deterministic)
  │
  ├─ Name Score ────── Jaro-Winkler (case-insensitive)
  ├─ Geo Score ─────── Haversine distance decay
  ├─ Address Score ──── Weighted field comparison
  ├─ ThingType Score ── Exact enum match (1.0 or 0.0)
  ├─ Identifier Score ─ Exact type+value match
  ├─ Phonetic Check ─── Soundex code comparison
  │
  ├─ Weighted Average (only available components)
  ├─ Phonetic Bonus (+5% if match and score < 0.95)
  │
  └─ Return MatchResult { score, confidence, breakdown }
```

## Component algorithms

### Name matching (`matching::name`)

- **Algorithm**: Jaro-Winkler similarity
- **Range**: 0.00 to 1.00
- **Case**: Case-insensitive (lowercased before comparison)
- **Empty handling**: Both empty = 1.00, one empty = 0.00
- **Prefix bonus**: Jaro-Winkler gives bonus for shared prefixes

### Address matching (`matching::address`)

- **Algorithm**: Weighted field-by-field Jaro-Winkler
- **Weights**: postal_code 30%, locality 25%, street 25%, region 10%, country 10%
- **Adaptive**: Only fields present in both addresses contribute
- **Case**: Case-insensitive

### Geo matching (`matching::geo`)

- **Algorithm**: Haversine distance with sigmoid decay
- **Formula**: `1.0 / (1.0 + distance_km / reference_km)`
- **Default reference**: 1.0 km (score = 0.5 at 1km apart)
- **Configurable**: `geo_similarity_with_reference(a, b, ref_km)`
- **Helper**: `within_radius(a, b, radius_m)` for radius search

### Identifier matching (`matching::identifier`)

- **Algorithm**: Exact type + value match across identifier lists
- **GLN detection**: `has_gln_match()` checks specifically for GLN matches
- **Deterministic**: GLN match short-circuits entire scoring to 1.0

### Phonetic matching (`matching::phonetic`)

- **Algorithm**: Soundex (4-character code)
- **Usage**: Applied as a bonus to the final score, not as a standalone component
- **Bonus**: +5% if Soundex codes match and score < 0.95

## Scoring (`matching::scoring`)

### Default weights

| Component  | Weight |
| ---------- | ------ |
| Name       | 0.35   |
| Geo        | 0.25   |
| Address    | 0.20   |
| Thing Type | 0.10   |
| Identifier | 0.10   |

Weights sum to 1.0. Only components where both things have data contribute.

### Confidence levels

| Level    | Score Range | Meaning         |
| -------- | ----------- | --------------- |
| Certain  | ≥ 0.95      | Definite match  |
| Probable | ≥ 0.80      | Likely match    |
| Possible | ≥ 0.60      | Potential match |
| Unlikely | < 0.60      | Not a match     |

### MatchResult

```rust
pub struct MatchResult {
    pub score: f64,           // 0.0 to 1.0
    pub confidence: MatchConfidence,
    pub breakdown: MatchBreakdown,
}

pub struct MatchBreakdown {
    pub name_score: f64,
    pub geo_score: f64,
    pub address_score: f64,
    pub thing_type_score: f64,
    pub identifier_score: f64,
    pub phonetic_match: bool,
    pub deterministic_match: bool,
}
```

## Usage example

```rust
use main_thing_service::matching::scoring::{compute_match, MatchWeights, MatchConfidence};
use main_thing_service::models::thing::Thing;

let a = Thing::new("Central Park");
let b = Thing::new("Centrl Park");
let result = compute_match(&a, &b, &MatchWeights::default());

println!("Score: {}", result.score);              // ~0.96
println!("Confidence: {:?}", result.confidence);  // Certain
println!("Name: {}", result.breakdown.name_score);
```
