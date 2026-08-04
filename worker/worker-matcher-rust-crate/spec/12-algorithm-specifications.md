## 12. Algorithm Specifications

This section is a summary. Verbatim per-subsection detail (deterministic-match branch enumeration, component-scoring table, address / birth-place / death-place / DOD sub-scores, confidence bands, batch scoring) is archived in [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md). Behaviour-defining numbers live there in canonical form; update both surfaces in lockstep.

### 12.1 Deterministic Matching

`deterministic_match` returns `true` iff any of the 42 scheme-local identifier branches fires (one per FR-12..FR-91), the passport-book branch fires (FR-52), or the demographic-tuple branch holds (normalised given + family + exact DOB + matching-or-missing gender). Identifiers are strictly scheme-local — see FR-13.

### 12.2 Component Scoring

Per-field rules (full table in [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md)): national identifiers + passport books — exact equality of per-scheme canonical form. Names — configured `name_algorithm`; nickname boost to `≥ 0.9`; middle blended `0.95 × given + 0.05 × middle`. DOB / death date — exact `1.0` / same-year DD↔MM transposition `0.5` / else `0.0`. Gender / blood type / multiple birth — exact equality. Address — postcode `0.5` + city `0.3` + line 1 `0.2`, weighted-average; best-of across `previous_addresses`. Birth / death place — `0.7 × JW(city) + 0.3 × exact(country)`. Phone — E.164 with legacy fallback. Email — equality of canonical form. Phonetic — Soundex average; asymmetric bonus only. Missing / unparseable input on either side → `None`. **Planned, not yet implemented (§23 T-33/T-34):** relationships would score as typed-set **Jaccard** over `(relation, worker_id)` pairs (`|A ∩ B| / |A ∪ B|`), so a line-manager reference would only agree with a line-manager reference to the **same** worker; tags would score as set **Jaccard** over the case-insensitively normalised tag sets (`|A ∩ B| / |A ∪ B|`). Both would be `None` (does not participate) when either side is empty.

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

