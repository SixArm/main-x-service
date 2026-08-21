## 7. Quality attributes and tuning

### 7.1 Performance budget

- A single `match_things` call SHOULD complete in `< 50 µs` on a 2024-vintage Apple Silicon laptop with default config and typical (≤ 5-name, ≤ 10-identifier, ≤ 5-sameAs) records.
- `rank_one_to_many` SHOULD scale linearly in the candidate count — no shared state between candidates.

### 7.2 Determinism budget

The crate produces identical bytes for identical inputs across runs, processes, and machines. There is no thread-local state, no global cache, no order-dependent collection iteration.

### 7.3 Stability

Since `0.4.0` shipped (the crate is now at `0.6.1`), the public types (`Thing`, `ThingBuilder`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Identifier`, `Confidence`, `MatchingError`, `Result`, `SimilarityAlgorithm`, `MatchingEngine`, `Normalizer`, `Scorer`) and their semantics are stable under SemVer.

Only `Thing` and `MatchingError` carry `#[non_exhaustive]` (confirmed in `src/models.rs` and `src/error.rs`; see the `AGENTS.md` Quick orientation table) — field additions to `Thing` and new `MatchingError` variants are therefore non-breaking. `MatchBreakdown` does **not** carry `#[non_exhaustive]`: adding `relationships_score` / `tags_score` (§3.7, §5.9.1, §5.9.2) will be a struct-literal-breaking change under strict SemVer, mitigated only by this crate's pre-1.0 "minor bumps may break" policy (`agents/release.md`).

### 7.4 Tuning guidance

Detailed tuning guidance (when to reach for `strict()`, when for `lenient()`, when to raise `identifiers_weight`, when to enable phonetic matching) lives in [`agents/matching-algorithm.md`](../agents/matching-algorithm.md). Defaults are calibrated for general-purpose catalogue deduplication.

---

