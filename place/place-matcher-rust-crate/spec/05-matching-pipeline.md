## 5. Matching pipeline

The crate exposes two strategies under one engine.

### 5.1 Deterministic match

```rust
fn deterministic_match(&self, p1: &Place, p2: &Place) -> bool;
```

Returns `true` iff **any one** of the following holds:

1. The two `place_ids` lists share at least one element under `PlaceId` equality (i.e. an identical `(scheme, value)` pair).
2. Both places carry a `name` that normalises (via `Normalizer::normalize_name`) to the same string **AND** both carry an `address.postcode` that normalises (via `Normalizer::normalize_postcode`) to the same string.

Otherwise returns `false`. Boolean result; no score.

### 5.2 Probabilistic match

```rust
fn match_places(&self, p1: &Place, p2: &Place) -> MatchResult;
```

Produces `MatchResult { score, is_match, confidence, breakdown }`: `breakdown` is the per-field `Option<f64>` contributions (§6); `score` is the weight-renormalised sum across components that scored (§5.2.2); `is_match` is `score >= MatchConfig::match_threshold` under default mode, additionally requiring `deterministic_match` under strict mode (§5.2.3); `confidence` is the fixed-band derivative of `score` (§3.6).

#### 5.2.1 Per-field scoring

Each field is scored as described in §6. A `None` from any component means "did not participate"; the field is skipped by the renormaliser.

#### 5.2.2 Weighted-sum reducer

```text
weighted_sum  = sum of (score_i * weight_i)  over components that scored
total_weight  = sum of (weight_i)            over components that scored
if name_phonetic_score is Some and > 0.9:
    weighted_sum += name_phonetic_score * 0.05
    total_weight += 0.05
score = if total_weight > 0 then weighted_sum / total_weight else 0.0
```

The renormalisation rule MUST hold: missing fields neither contribute to nor penalise the overall score. A worked numeric trace lives in §11.1.

#### 5.2.3 Strict mode

When `MatchConfig::strict_mode = true`, the engine computes the same `score` and `confidence`, but tightens `is_match` to `score >= match_threshold && deterministic_match`. A fuzzy near-match that clears the threshold but has no shared place ID and no normalised-name + postcode agreement is rejected as `is_match = false`.

### 5.3 Batch entry points

```rust
fn match_one_to_many(&self, query: &Place, candidates: &[Place]) -> Vec<MatchResult>;
fn rank_one_to_many(&self, query: &Place, candidates: &[Place]) -> Vec<(usize, MatchResult)>;
```

- `match_one_to_many` returns results parallel to the input slice.
- `rank_one_to_many` returns the same results sorted by descending `score`, with ties broken by ascending original index. Ordering MUST be deterministic across calls with the same inputs.

### 5.4 Behaviour when a field is missing

If a field is `None` on either side, the corresponding `MatchBreakdown` entry is `None` and the field is skipped by the weighted-sum reducer. This is the renormalisation rule: missing data is treated as "no information", not as "disagreement".

The single exception is `name_phonetic_score`: when `MatchConfig::use_phonetic_matching = false`, it is always `None` regardless of input data.

### 5.5 Match evidence (Fellegi–Sunter)

Every `MatchResult` also carries `evidence: Evidence`, computed by `src/evidence.rs` under the family contract in [`agents/share/match-evidence.md`](../../../agents/share/match-evidence.md). `score`, `is_match` and `confidence` are unchanged by it; evidence answers a different question ("how much does the data *prove*?") and is the gate a service uses for auto-merge.

- **Components.** Each `Some` entry of the `MatchBreakdown` adds `component_bits(s, m, u)` (agreement at `s >= 0.95`, disagreement at `s <= 0.60`, linear between) and counts once in `fields_compared`. A `None` entry adds exactly `0`. The neutral `0.5` address score for two addresses with no sub-field populated on both sides (§6.4) is absence, not disagreement, and is skipped.
- **Identifiers.** Every `PlaceIdScheme` (including `Other(_)`) is treated as globally unique within itself, the assumption the §5.1 shared-`(scheme, value)` rule already makes. `place_ids_score` is weighed as the identifier: a shared non-empty `(scheme, value)` adds `IDENTIFIER_BITS = log2(0.99/1e-12) ≈ 39.85` once, and is never also counted through the component table. Identifiers count once in `fields_compared` when at least one scheme is held by both sides.
- **No identifier bits for name + postcode.** The second §5.1 rule (normalised name + postcode) is not a unique identifier; its fields are weighed as the `name` and `address` components. `local_id` (§6.7a) is unique only within one source and is likewise a plain component.
- **Contradictions.** For each scheme under which both sides hold at least one non-empty (trimmed) value and the value sets are disjoint, a `Contradiction { field: "identifier:<scheme>" }` is recorded (`google`, `osm_node`, `osm_way`, `osm_relation`, `geonames`, `wikidata`, `foursquare`, `here`, `mapbox`, `other:<name lowercased>`) and `log2(0.01/(1-1e-12)) ≈ -6.64` bits are added. Different schemes never contradict each other (an OSM node and an OSM way for one feature is normal).
- **Strength.** `Contradicted` if any contradiction; else `Proven` if `weight_bits >= MatchConfig::proof_bits` (default `30`); else `Consistent` if `is_match`; else `NonMatch`.

The per-component `m`/`u` table (informed estimates, not measurements):

| Component | `m` | `u` | Agree bits | Disagree bits | Where `u` comes from |
|---|---|---|---|---|---|
| `name_score` | 0.95 | 0.005 | +7.57 | −4.31 | place names repeat (chains, "Station Road Café") |
| `name_phonetic_score` | 0.90 | 0.30 | +1.58 | −2.81 | Soundex collides often and is correlated with `name` |
| `coordinates_score` | 0.90 | 1e-4 | +13.14 | −3.32 | full agreement is within ~11 m at the 50 m scale |
| `address_score` | 0.85 | 0.001 | +9.73 | −2.74 | shared mainly within one building / centre |
| `category_score` | 0.90 | 0.10 | +3.17 | −3.17 | a few categories cover most places |
| `country_code_score` | 0.98 | 0.10 | +3.29 | −5.49 | registries skew to a few countries |
| `place_ids_score` | — | — | +39.85 | −6.64 per scheme | identifier rule above |
| `phone_score` | 0.85 | 0.001 | +9.73 | −2.74 | chain switchboards |
| `email_score` | 0.80 | 0.002 | +8.64 | −2.32 | shared `info@` addresses |
| `local_id_score` | 0.95 | 0.001 | +9.89 | −4.32 | unique within one source only (opt-in) |

Worked figures with the default config: two identical records holding only a name and coordinates score `1.0` and `is_match`, but reach only ≈ 20.7 bits — `Consistent`, not `Proven`. A richly identical record (name, coordinates, address, category, country code, phone, email; no identifier) reaches ≈ 55.3 bits — `Proven` without an identifier. A shared `PlaceId` alone clears the bar.

---

