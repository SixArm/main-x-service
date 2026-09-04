## 22. Anti-patterns

- **Calling `match_courses` in a hot loop with the same `MatchingEngine`
  reconstructed each iteration.** Build the engine once.
- **Adding stop-words / stemming.** Out of scope (§2.2).
- **Setting weights to negative.** Not validated today — caller
  contract. Consequence, made explicit by §23 T-13: `scoring::
  weighted_average`'s `[0.0, 1.0]` return guarantee assumes every
  present component's weight is non-negative; a negative
  `MatchConfig` weight can push the renormalised score outside that
  range (pinned by
  `weighted_average_negative_weight_breaks_the_unit_interval_bound`,
  `src/scoring.rs`). This is the documented behaviour of the caller
  contract above, not a bug — a future decision to validate
  `MatchConfig` would be a deliberate change to this contract.
- **Using `Course::default()` in tests.** Use `Course::new(name)` so
  the required field is set.

