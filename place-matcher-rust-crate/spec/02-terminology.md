## 2. Terminology

The following terms are used throughout this spec with the meanings defined here. Other documents in the repository MUST use the same vocabulary.

- **Place** — a single record about a geographic place, as represented by the `Place` struct (§3.1). May describe a landmark, natural feature, chain-store branch, administrative area, or any other geographically-located entity.
- **Match** — the verdict that two `Place` records refer to the same geographic place. Verdicts come in two flavours, deterministic and probabilistic, with sharply different guarantees.
- **Deterministic match** — a boolean verdict from `MatchingEngine::deterministic_match` (§5.1). Returns `true` only when an objective, transitive criterion is satisfied. Never produces a score.
- **Probabilistic match** — a `MatchResult` from `MatchingEngine::match_places` (§5.2) carrying a score, an `is_match` boolean, a `Confidence` band, and a `MatchBreakdown`.
- **Per-field breakdown** — the `MatchBreakdown` struct (§3.7) containing one `Option<f64>` per scored component. `None` means "not scored on at least one side"; `Some(s)` carries a value in `[0.0, 1.0]`.
- **Renormalisation** — the rule in §5.2.2 by which the weighted sum is divided by the sum of *participating* weights, so missing fields neither contribute to nor penalise the overall score.
- **Confidence band** — a coarse `High` / `Medium` / `Low` bucket derived from the probabilistic score (§3.6). Bands are fixed; they do **not** follow `match_threshold`.
- **Normalisation** — the deterministic, idempotent text transformations in `Normalizer` (§4) applied before comparison.
- **Scheme-local identifier** — a `PlaceId` is identified by both its `scheme` (e.g. `Wikidata`) and its `value`. Identifiers from different schemes never match each other, even if the value string is identical (§3.5).
- **Score** — a real number in `[0.0, 1.0]`, where `1.0` means "identical" and `0.0` means "no observable similarity".
- **Weight** — a dimensionless multiplier on a component score. Weights need not sum to `1.0`; the renormaliser handles that.
- **Strict mode** — the `MatchConfig::strict_mode` flag (§5.2.3): when `true`, the probabilistic `is_match` boolean additionally requires `deterministic_match` to return `true`.

---

