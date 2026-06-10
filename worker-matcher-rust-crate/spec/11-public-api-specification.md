## 11. Public API Specification

Stable re-exports from `lib.rs`: `identifiers` module (42 parsers + 9 passport-format validators); `MatchingError`, `Result`; `Confidence`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, `MatchingEngine`; `Address`, `BloodType`, `Gender`, `PassportBook`, `Worker`, `WorkerBuilder`; `NicknameTable`; `Normalizer`, `ParsedAddressLine`; `Scorer`, `SimilarityAlgorithm`.

**Stability rules.** `Worker` and `Address` carry `#[non_exhaustive]` (FR-53); external consumers construct via `Worker::builder()` or `Address::new()` + `with_*` fluent setters. Adding fields = minor bump; removing/renaming = major; changing default weights = minor (with CHANGELOG "Behaviour Change" entry); changing the meaning of `is_match` for the same `score` = major.

---

