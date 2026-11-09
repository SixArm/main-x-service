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

- **Components.** Each `Some` entry of the `MatchBreakdown` adds `component_bits(s, m, u)` (agreement at `s >= 0.95`, disagreement at `s <= 0.60`, linear between) and counts once in `fields_compared`. A `None` entry adds exactly `0`. `name_phonetic_score` is the exception: derived from the names, it is never summed (§5.5.1). The neutral `0.5` address score for two addresses with no sub-field populated on both sides (§6.4) is absence, not disagreement, and is skipped.
- **Identifiers.** Every `PlaceIdScheme` (including `Other(_)`) is treated as globally unique within itself, the assumption the §5.1 shared-`(scheme, value)` rule already makes. `place_ids_score` is weighed as the identifier: a shared non-empty `(scheme, value)` adds `IDENTIFIER_BITS = log2(0.99/1e-12) ≈ 39.85` once, and is never also counted through the component table. Identifiers count once in `fields_compared` when at least one scheme is held by both sides.
- **No identifier bits for name + postcode.** The second §5.1 rule (normalised name + postcode) is not a unique identifier; its fields are weighed as the `name` and `address` components. `local_id` (§6.7a) is unique only within one source and is likewise a plain component.
- **Contradictions.** For each scheme under which both sides hold at least one non-empty (trimmed) value and the value sets are disjoint, a `Contradiction { field: "identifier:<scheme>" }` is recorded (`google`, `osm_node`, `osm_way`, `osm_relation`, `geonames`, `wikidata`, `foursquare`, `here`, `mapbox`, `other:<name lowercased>`) and `log2(0.01/(1-1e-12)) ≈ -6.64` bits are added. Different schemes never contradict each other (an OSM node and an OSM way for one feature is normal).
- **Strength.** `Contradicted` if any contradiction; else `Proven` if `weight_bits >= MatchConfig::proof_bits` (default `30`); else `Consistent` if `is_match`; else `NonMatch`.

The per-component `m`/`u` table (informed estimates, not measurements):

| Component | `m` | `u` | Agree bits | Disagree bits | Where `u` comes from |
|---|---|---|---|---|---|
| `name_score` | 0.95 | 0.005 | +7.57 | −4.31 | place names repeat (chains, "Station Road Café") |
| `name_phonetic_score` | — | — | 0 | 0 | not summed: derived from the names (§5.5.1) |
| `coordinates_score` | 0.90 | 1e-4 | +13.14 | −3.32 | full agreement is within ~11 m at the 50 m scale |
| `address_score` | 0.85 | 0.001 | +9.73 | −2.74 | shared mainly within one building / centre |
| `category_score` | 0.90 | 0.10 | +3.17 | −3.17 | a few categories cover most places |
| `country_code_score` | 0.98 | 0.10 | +3.29 | −5.49 | registries skew to a few countries |
| `place_ids_score` | — | — | +39.85 | −6.64 per scheme | identifier rule above |
| `phone_score` | 0.85 | 0.001 | +9.73 | −2.74 | chain switchboards |
| `email_score` | 0.80 | 0.002 | +8.64 | −2.32 | shared `info@` addresses |
| `local_id_score` | 0.95 | 0.001 | +9.89 | −4.32 | unique within one source only (opt-in) |

Worked figures with the default config and no frequencies: two identical records holding only a name and coordinates score `1.0` and `is_match`, but reach only ≈ 20.7 bits — `Consistent`, not `Proven`. A richly identical record (name, coordinates, address, category, country code, phone, email; no identifier) reaches ≈ 44.4 bits after the §5.5.1 discount (≈ 55.3 summed independently) — `Proven` without an identifier. Two different businesses in one building (same coordinates, address and country, different names) reach ≈ 15.3 bits (≈ 21.9 undiscounted) — never `Proven`. A shared `PlaceId` alone clears the bar.

#### 5.5.1 Correlated components (`match-evidence.md` §6)

Components that are not independent given the pair's truth are declared in `evidence::CORRELATED_GROUPS` (public contract):

| Group | Why |
|---|---|
| `coordinates`, `address` | A geocoded address is the same fact twice, and every tenant of one building shares both. |
| `address`, `country_code` | The address determines the country. |
| `phone`, `email` | Contact details are recorded together, and every branch of a chain (or tenant of a serviced office) shares a switchboard and an `info@` together. |

`name_phonetic_score` is in no group and adds **no term at all**: the Soundex code is derived from the names, so summing it would count the name twice (`match-evidence.md` §6). It still feeds `score` as the §5.2.2 bonus, and `fields_compared` does not count it.

Within a group, the member with the largest absolute bits counts in full (ties to the larger value, so the result does not depend on component order) and every other present member counts at `CORRELATED_DISCOUNT = 0.5`. Disagreements are discounted the same way. **Groups that share a member are merged** before discounting, so `coordinates`, `address` and `country_code` form one group of three: each component is discounted at most once and appears once in `components`. `category` and `local_id` stand alone.

#### 5.5.2 Common and rare values (`match-evidence.md` §7)

`MatchingEngine::match_places_with_frequencies(a, b, &dyn ValueFrequencies)` takes the share of records holding a value; `match_places` calls it with `NoFrequencies`, so plain calls get exactly the §5.5 table. `score`, `is_match`, `confidence` and the breakdown are identical in both; only `evidence` differs.

- `FREQUENCY_COMPONENTS = ["name"]` (public contract). Chain and generic names ("Starbucks", "Main Street Surgery", "St Mary's Church") are held by many places; a rare name is strong.
- **Key.** `evidence::frequency_key("name", raw)` returns `Normalizer::normalize_name(raw)` (NFKD, combining marks and ASCII punctuation dropped, lowercased, whitespace collapsed), or `None` when that is empty; any other component returns `None`. A service counts each of a place's names (primary and alternates) under this key.
- **When it applies.** Only when `name_score >= AGREE_AT` **and** both places hold at least one identical key. A near-exact typo, a partial agreement and a disagreement are never adjusted. When several names are shared, the **largest** known share is used (the conservative choice); when none is known, the table's `u` applies.
- **Bits.** `frequency_bits(m, share) = log2(m / clamp(share, MIN_SHARE = 1e-9, m))`, replacing the agreement bits before any group discount. A share at or above `m` gives `0`, never negative. With `m = 0.95`: share `0.01` gives 6.57 bits (fewer than the table's 7.57), `1e-6` gives 19.86.
- `FrequencyTable` (`new`, `insert(component, key, share)`, `from_counts(count, total) -> Option<f64>`) is a ready `ValueFrequencies`.

Two branches of one chain ~25 m apart, both "Starbucks", same category and country: ≈ 17.5 bits without frequencies, ≈ 15.5 with a name share of `0.02` — `Consistent`, not `Proven`. Note the table's name `u` (`0.005`) already allows for chains, so a common-name share moves only a few bits; two branches that also share a central phone and email and sit within ~11 m can still clear 30 bits (§10 OQ-O).

#### 5.5.3 Explainability (`match-evidence.md` §8)

`Evidence::components: Vec<ComponentBits>` (`#[serde(default)]`) lists every term — `component`, `bits` (after discount or adjustment), `correlated`, `frequency_adjusted` — in a fixed order: the table components in §5.5 order, then `identifier`, then one `contradiction:identifier:<scheme>` per contradiction. `weight_bits` is computed as the sum of the listed terms, so they add up exactly (pinned by a property test).
