## 11. Testing Strategy

Layered: [`AGENTS/testing.md`](../AGENTS/testing.md).

- **Unit tests** — `#[cfg(test)]` modules; models, matching,
  validation, privacy. ~100 tests.
- **Integration tests** — `tests/integration_*.rs`; end-to-end
  workflows.
- **Bridge integration tests** — `tests/duplicate_detection.rs`;
  drives service-side records through `adapter::to_matcher_thing` and
  asserts on `MatchingEngine::match_things` end-to-end. Covers
  identical clones, name typos, deterministic identifier short-circuits,
  field-routing pinning, and config-preset invariants. 15 tests.
- **Benchmarks** — Criterion: matching, search, validation, privacy.
- **CI** — `test.yml`, `quality.yml`, `security.yml`.

