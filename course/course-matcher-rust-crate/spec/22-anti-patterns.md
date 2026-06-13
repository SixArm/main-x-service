## 22. Anti-patterns

- **Calling `match_courses` in a hot loop with the same `MatchingEngine`
  reconstructed each iteration.** Build the engine once.
- **Adding stop-words / stemming.** Out of scope (§2.2).
- **Setting weights to negative.** Not validated today — caller
  contract.
- **Using `Course::default()` in tests.** Use `Course::new(name)` so
  the required field is set.

