## 2. Terminology

The following terms are used throughout this spec with the meanings defined here. Other documents in the repository MUST use the same vocabulary.

- **Place** — a single record about a geographic place, as represented by the `Place` struct (§3.1). May describe a landmark, natural feature, chain-store branch, administrative area, or any other geographically-located entity.
- **Deterministic match** — a boolean verdict from `MatchingEngine::deterministic_match` (§5.1). Returns `true` only when an objective, transitive criterion is satisfied. Never produces a score.
- **Probabilistic match** — a `MatchResult` from `MatchingEngine::match_places` (§5.2) carrying a score, an `is_match` boolean, a `Confidence` band, and a `MatchBreakdown` (one `Option<f64>` per scored component).
- **Renormalisation** — divide the weighted sum by the sum of *participating* weights, so missing fields neither contribute to nor penalise the overall score (§5.2.2).
- **Confidence band** — a coarse `High` / `Medium` / `Low` bucket derived from the score (§3.6). Fixed bands; independent of `match_threshold`.
- **Normalisation** — the deterministic, idempotent text transformations in `Normalizer` (§4) applied before comparison.
- **Scheme-local identifier** — a `PlaceId` is identified by both its `scheme` and its `value`. Identifiers from different schemes never match, even with identical value strings (§3.5).
- **Score** — a real number in `[0.0, 1.0]`. **Weight** — a dimensionless multiplier on a component score; weights need not sum to `1.0`.
- **Strict mode** — `MatchConfig::strict_mode` (§5.2.3): when `true`, `is_match` additionally requires `deterministic_match` to return `true`.

---

