## 18. Confidence classification

- `Confidence::High` for score ≥ 0.95.
- `Confidence::Medium` for score in `[0.70, 0.95)`.
- `Confidence::Low` for score < 0.70.

This is independent of `MatchConfig::threshold` (used by
`is_match`).

