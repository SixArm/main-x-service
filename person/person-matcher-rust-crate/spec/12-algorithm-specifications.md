## 12. Algorithm Specifications

Full per-component score tables and pseudocode are in [AGENTS/matching-algorithm.md](../AGENTS/matching-algorithm.md) under "Detailed Algorithm Specifications". Summary:

- **§12.1 Deterministic** — fires on same-scheme identifier agreement (any of 42 schemes), passport-book agreement, or full demographic-tuple agreement (normalised given + family + DOB + compatible gender).
- **§12.2 Component scoring** — per-field scores in `[0.0, 1.0]` or `None`. Identifiers: exact canonical equality. Names: `name_algorithm` (JW / Lev / Exact / Combined) with nickname boost to `≥ 0.9` and `0.95 × given + 0.05 × middle` blend. DOB: exact (`1.0`) or same-year day/month transposition (`0.5`). Gender / blood type / multiple birth: exact equality. Birth/death place: shared `score_named_place` (`0.7 × city + 0.3 × country`). Death date: reuses DOB transposition. Address: §12.4. Phone: E.164 preferred + legacy fallback. Email: canonical equality. Phonetic: Soundex. **Planned, not yet implemented** (§23.2 T-33 / T-34): Relationships would score by typed-set **Jaccard** over the `(relation, person_id)` pairs — `|A ∩ B| / |A ∪ B|`, so a parent reference only agrees with a parent reference to the **same** person; `None` (does not participate) when either side has no relationships. Tags would score by plain set **Jaccard** over the case-insensitively normalised tag sets — `|A ∩ B| / |A ∪ B|`; `None` (does not participate) when either side has an empty tag set.
- **§12.3 Probabilistic** — `score = Σ(score × weight) / Σ(weight)` over participating fields. Phonetic bonus is asymmetric (`+ s × 0.05` when `s > 0.9`); only lifts.
- **§12.4 Address sub-score** — weighted average over postcode (0.5), city (0.3), line 1 (0.2). Line 1 is a `(house_number, street)` blend: `0.6 × street_sim + 0.4 × house_score` when both have a house number, street similarity alone otherwise. Empty-address fallback `0.5`. Best-of across `(current ∪ previous_addresses)` on both sides (FR-48).
- **§12.4a/b/c Place / date-of-death sub-scores** — see AGENTS for city / country blend rules.
- **§12.5 Confidence bands** — `score ≥ 0.90 → High`, `≥ 0.75 → Medium`, else `Low`; independent of `match_threshold`.
- **§12.6 Batch** — `match_one_to_many` (parallel-to-slice); `rank_one_to_many` (sorted descending; deterministic index tie-break). Engine is `Send + Sync`; consumers layer parallelism.

All behaviour-defining numbers are pinned by AGENTS/matching-algorithm.md and the test suite.

