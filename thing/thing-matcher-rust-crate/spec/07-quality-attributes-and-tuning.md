## 7. Quality attributes and tuning

### 7.1 Performance budget

- A single `match_things` call SHOULD complete in `< 50 µs` on a 2024-vintage Apple Silicon laptop with default config and typical (≤ 5-name, ≤ 10-identifier, ≤ 5-sameAs) records.
- `rank_one_to_many` SHOULD scale linearly in the candidate count — no shared state between candidates.

### 7.2 Determinism budget

The crate produces identical bytes for identical inputs across runs, processes, and machines. There is no thread-local state, no global cache, no order-dependent collection iteration.

### 7.3 Stability

Once `0.4.0` ships, the public types (`Thing`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Identifier`, `Confidence`, `MatchingError`, `SimilarityAlgorithm`, `MatchingEngine`, `Normalizer`, `Scorer`) and their semantics are stable under SemVer. Field additions to `Thing` and `MatchBreakdown` go via `#[non_exhaustive]` so they are not breaking. New `MatchingError` variants are non-breaking for the same reason.

### 7.4 Tuning guidance

Detailed tuning guidance (when to reach for `strict()`, when for `lenient()`, when to raise `identifiers_weight`, when to enable phonetic matching) lives in [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md). Defaults are calibrated for general-purpose catalogue deduplication.

---

