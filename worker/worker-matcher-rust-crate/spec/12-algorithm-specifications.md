## 12. Algorithm Specifications

This section is a summary. Verbatim per-subsection detail (deterministic-match branch enumeration, component-scoring table, address / birth-place / death-place / DOD sub-scores, confidence bands, batch scoring) is archived in [`agents/matching-algorithm.md`](../agents/matching-algorithm.md). Behaviour-defining numbers live there in canonical form; update both surfaces in lockstep.

### 12.1 Deterministic Matching

`deterministic_match` returns `true` iff any of the 42 scheme-local identifier branches fires (one per FR-12..FR-91), the passport-book branch fires (FR-52), or the demographic-tuple branch holds (normalised given + family + exact DOB + matching-or-missing gender). Identifiers are strictly scheme-local — see FR-13.

### 12.2 Component Scoring

Per-field rules (full table in [`agents/matching-algorithm.md`](../agents/matching-algorithm.md)): national identifiers + passport books — exact equality of per-scheme canonical form. Names — configured `name_algorithm`; nickname boost to `≥ 0.9`; middle blended `0.95 × given + 0.05 × middle`. DOB / death date — exact `1.0` / same-year DD↔MM transposition `0.5` / else `0.0`. Gender / blood type / multiple birth — exact equality, with gender `Unknown` scored as missing (`None`), since it means no gender was recorded; the name + DOB + gender short-circuit likewise treats `Unknown` as absent. Address — postcode `0.5` + city `0.3` + line 1 `0.2`, weighted-average; best-of across `previous_addresses`. Birth / death place — `0.7 × JW(city) + 0.3 × exact(country)`. Phone — E.164 with legacy fallback. Email — equality of canonical form. Phonetic — Soundex average; asymmetric bonus only. Missing / unparseable input on either side → `None`. Relationships score as typed-set **Jaccard** over `(relation, worker_id)` pairs (`|A ∩ B| / |A ∪ B|`), so a line-manager reference only agrees with a line-manager reference to the **same** worker; tags score as set **Jaccard** over the case-insensitively normalised tag sets (`|A ∩ B| / |A ∪ B|`). Both are `None` (does not participate) when either side is empty (§23 T-33/T-34, v0.7.0).

### 12.3 Probabilistic Scoring

```text
weighted_sum = Σ_field  score_field × weight_field   (over fields with score = Some)
total_weight = Σ_field  weight_field                  (over the same fields)
if phonetic_score is Some(s) and s > 0.9:
    weighted_sum += s × 0.05;  total_weight += 0.05
score = weighted_sum / total_weight   (or 0.0 if total_weight == 0)
is_match = score >= match_threshold
```

Missing fields are dropped from both numerator and denominator. The phonetic bonus is asymmetric (only ever lifts).

### 12.4 Address Sub-Score

Postcode (exact, `0.5`) + city (Jaro-Winkler, `0.3`) + line 1 (structured house-number + street, `0.2`); weighted-average; neutral `0.5` when nothing fires; best-of across `(address ∪ previous_addresses)` cartesian product.

### 12.5 Confidence Bands

`High` for `score >= 0.90`; `Medium` for `0.75 <= score < 0.90`; `Low` otherwise. Independent of `match_threshold`. `Confidence::from_score` is total over `f64` (NaN → `Low`).

### 12.6 Batch Scoring

`match_one_to_many` returns `Vec<MatchResult>` parallel to the input slice; `rank_one_to_many` returns `Vec<(usize, MatchResult)>` sorted by descending score with ascending-index tiebreak. No blocking; engine is `Send + Sync` for consumer-managed parallelism.

### 12.7 Match Evidence (T-38)

Beside `score`, every `MatchResult` carries `evidence: Evidence`, per the family contract [`agents/share/match-evidence.md`](../../../agents/share/match-evidence.md). `score`, `is_match` and `confidence` are unchanged. `score` asks how alike the data both records hold; evidence asks how much that data *proves*, which is what a merge must be gated on.

- **Weight.** `weight_bits = Σ component_bits(s, m, u)` over every non-identifier breakdown component that is `Some`, where `component_bits` gives `log2(m/u)` at `s ≥ 0.95`, `log2((1−m)/(1−u))` at `s ≤ 0.60`, and interpolates linearly between. A missing component adds `0` and is not counted in `fields_compared`. The address score's neutral `0.5` (no sub-field on both sides, §12.4) is absence, not disagreement, and is left out.
- **Identifiers.** A value shared under any of the 42 national-identifier schemes, or a shared passport `(country, number)` pair, adds `IDENTIFIER_BITS = log2(0.99/1e-12) ≈ 39.85` once; all identifiers together count as one field compared. The demographic-tuple branch of §12.1 is **not** an identifier and adds nothing beyond its components.
- **Contradictions.** Each national-identifier scheme under which both records parse to different values (each scheme is single-valued on `Worker`) records `Contradiction { field: "identifier:<scheme>" }` (e.g. `identifier:uk_nhs_number`) and adds `log2(0.01/(1−1e-12)) ≈ −6.64` bits. Passport books never contradict: a passport number changes on every renewal, so disjoint book lists are ordinary.
- **Strength**, checked in order: any contradiction → `Contradicted`; `weight_bits ≥ proof_bits` (§13.3) → `Proven`; `is_match` → `Consistent`; else `NonMatch`.

The per-component `m`/`u` table (informed estimates, not measurements; `src/evidence.rs` carries a line per entry on where `u` comes from):

| Component | `m` | `u` | agree bits | disagree bits |
|---|---|---|---|---|
| given name | 0.95 | 0.02 | 5.57 | −4.29 |
| family name | 0.95 | 0.005 | 7.57 | −4.31 |
| date of birth | 0.97 | 1/14 600 | 13.79 | −5.06 |
| gender | 0.98 | 0.5 | 0.97 | −4.64 |
| blood type | 0.98 | 0.25 | 1.97 | −5.23 |
| multiple birth | 0.98 | 0.5 | 0.97 | −4.64 |
| address | 0.80 | 1e-4 | 12.97 | −2.32 |
| birth place | 0.90 | 0.05 | 4.17 | −3.25 |
| death date | 0.97 | 1/36 500 | 15.11 | −5.06 |
| death place | 0.90 | 0.05 | 4.17 | −3.25 |
| phone | 0.85 | 1e-4 | 13.05 | −2.74 |
| email | 0.85 | 1e-5 | 16.38 | −2.74 |
| relationships | 0.60 | 0.10 | 2.58 | −1.17 |
| tags | 0.60 | 0.20 | 1.58 | −1.00 |

The phonetic-name component is **not** summed: it is derived from the two name components, so counting it would count the names twice (contract §6). It still lifts `score` (§12.3). Family-name `u` was `0.01` in T-38 and is `0.005` since T-39: the sum of squared surname shares, so a common surname with its own share of `0.01` (§12.9) is weaker than the average.

Consequences at defaults (T-39):

| Identical pair holding | Bits | Strength |
|---|---|---|
| given + family + DOB + gender | 27.90 | `Consistent` |
| … + blood type | 29.87 | `Consistent` — just under the bar on purpose: nothing here rules out two workers with a common name born the same day; one more independent field clears it |
| … + address, birth place, phone, email | ≈ 70 | `Proven` |
| address + phone only | 19.54 | not `Proven` (§12.8) |
| family name + address + phone | 27.11 | not `Proven`; ungrouped it would be 33.6 |

### 12.8 Correlated Components (T-39)

Per contract §6, `CORRELATED_GROUPS` (public contract, `src/evidence.rs`) lists components that are not independent given the match status. Within a group the present member with the largest absolute bits (ties to the larger value) counts in full and every other present member at `CORRELATED_DISCOUNT = 0.5`, for agreement and disagreement alike; the result does not depend on component order. The groups are disjoint.

| Group | Why |
|---|---|
| `address`, `phone` | A household's landline is recorded with, and moves with, its address. Two people in one house agree on both for one reason, and a move makes both disagree for one reason. |
| `death_date`, `death_place` | Both come from one death registration. |

`birth_place` + `address` is **not** grouped: they are recorded separately and at different times, and the address comparison is postcode-and-street led, which a birth city does not predict. Name, birth date and gender stay independent of each other.

### 12.9 Common and Rare Values (T-39)

Per contract §7, `MatchingEngine::match_workers_with_frequencies(a, b, &dyn ValueFrequencies)` adjusts the agreement weight of each `FREQUENCY_COMPONENTS` entry when both workers hold the **same** key and the source knows its share: the bits are `frequency_bits(m, share) = log2(m / clamp(share, MIN_SHARE = 1e-9, m))` in place of the table's `u`, so a share at or above `m` gives `0`, never negative. Partial agreement (e.g. "Prichard" vs "Pritchard", a nickname boost) and disagreement are not adjusted. `match_workers` uses `NoFrequencies`, so plain calls see the §12.7 table exactly. `score`, `is_match`, `confidence` and `breakdown` are identical with or without frequencies. `FrequencyTable` (`new`, `from_counts(counts, total)` → `count / total`, `insert(component, value, share)`) is an in-memory source.

The frequency components and their keys are public contract; `frequency_key(component, raw)` returns exactly the key the matcher looks up:

| Component | Key |
|---|---|
| `given_name` | `Normalizer::normalize_name` (lowercase, diacritics and punctuation stripped, whitespace collapsed); `None` if empty. The middle name is not part of the key. |
| `family_name` | as `given_name` |
| `date_of_birth` | ISO 8601 `YYYY-MM-DD`; `raw` must parse as one |

Examples: a family name with share `0.01` gives 6.57 bits against the table's 7.57; one with share `1e-6` gives 19.86. A placeholder birth date such as `1900-01-01` held by 30% of records gives 1.69 bits against the table's 13.79.

### 12.10 Per-Term Listing (T-39)

Per contract §8, `Evidence.components: Vec<ComponentBits>` lists every term of the sum, in order: `"identifier"` (when shared), one `"contradiction:identifier:<scheme>"` per contradiction, then each compared component by its `COMPONENTS` name, with `bits` after any discount, `correlated` (a discounted non-strongest group member) and `frequency_adjusted`. The terms add up to `weight_bits`; a property test checks it.
