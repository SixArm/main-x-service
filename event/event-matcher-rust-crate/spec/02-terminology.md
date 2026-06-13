## 2. Terminology

The following terms are used throughout this spec with the meanings defined here. Other documents in the repository MUST use the same vocabulary.

- **Event** — a single record about an event, as represented by the `Event` struct (§3.1), modelled on schema.org/Event. May describe a festival, conference, concert, sports fixture, screening, hackathon, meetup, or any other time-bounded happening.
- **Deterministic match** — a boolean verdict from `MatchingEngine::deterministic_match` (§5.1). Returns `true` only when an objective, transitive criterion is satisfied. Never produces a score.
- **Probabilistic match** — a `MatchResult` from `MatchingEngine::match_events` (§5.2) carrying a score, an `is_match` boolean, a `Confidence` band, and a `MatchBreakdown` (one `Option<f64>` per scored component).
- **Renormalisation** — divide the weighted sum by the sum of *participating* weights, so missing fields neither contribute to nor penalise the overall score (§5.2.2).
- **Confidence band** — a coarse `High` / `Medium` / `Low` bucket derived from the score (§3.9). Fixed bands; independent of `match_threshold`.
- **Normalisation** — the deterministic, idempotent text and date-time transformations in `Normalizer` (§4) applied before comparison.
- **Scheme-local identifier** — an `EventId` is identified by both its `scheme` and its `value`. Identifiers from different schemes never match, even with identical value strings (§3.8).
- **Score** — a real number in `[0.0, 1.0]`. **Weight** — a dimensionless multiplier on a component score; weights need not sum to `1.0`.
- **Strict mode** — `MatchConfig::strict_mode` (§5.2.3): when `true`, `is_match` additionally requires `deterministic_match` to return `true`.

---
