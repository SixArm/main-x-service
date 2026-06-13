## 11. Testing Strategy

Layered: [`AGENTS/testing.md`](../AGENTS/testing.md).

- **Unit tests** — `#[cfg(test)]` modules; models (32), matching
  (45), validation (19), privacy (8). 104 tests.
- **Integration tests** — `tests/integration_*.rs`; end-to-end
  workflows + edge cases (unicode names, geo poles, date line, GLN
  deterministic override, address normalisation edge cases, GDPR
  field preservation). 67 tests.
- **Bridge integration tests** — `tests/duplicate_detection.rs`;
  drives service-side records through `adapter::to_matcher_place` and
  asserts on `MatchingEngine::match_places` end-to-end. Covers
  identical clones, name typos, deterministic identifier short-circuits,
  field-routing pinning, and config-preset invariants. 14 tests.
- **Benchmarks** — Criterion: 16 — matching, search, validation,
  privacy.
- **CI** — `test.yml`, `quality.yml`, `security.yml`.

