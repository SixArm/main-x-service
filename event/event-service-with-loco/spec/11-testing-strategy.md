## 11. Testing Strategy

Layered: [`AGENTS/testing.md`](../AGENTS/testing.md).

- **Unit tests** — `#[cfg(test)]` modules; matching, scoring,
  validation (including attendance-mode/location coupling and capacity
  invariants), privacy, models, time-interval algebra.
- **Integration tests** — `tests/`; full HTTP request/response
  cycles against real PostgreSQL + Tantivy.
- **Bridge integration tests** — `tests/duplicate_detection.rs`;
  drives service-side records through `adapter::to_matcher_event` and
  asserts on `MatchingEngine::match_events` end-to-end. Covers
  identical clones, name typos, deterministic identifier short-circuits,
  field-routing pinning, and config-preset invariants.
- **Benchmarks** — Criterion for matching, search, validation.
- **CI** — `test.yml`, `quality.yml`, `security.yml`.

