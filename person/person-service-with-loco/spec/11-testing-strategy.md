## 11. Testing Strategy

Layered: [`AGENTS/testing.md`](../AGENTS/testing.md).

- **Unit tests** — embedded `#[cfg(test)]` modules; matching, phonetic,
  scoring, validation, privacy, models. ~100 tests.
- **Integration tests** — `tests/`; full HTTP request/response cycles
  against real PostgreSQL + Tantivy.
- **Bridge integration tests** — `tests/duplicate_detection.rs`;
  drives service-side records through `adapter::to_matcher_person` and
  asserts on `MatchingEngine::match_persons` end-to-end. Covers
  identical clones, name typos, deterministic identifier short-circuits,
  field-routing pinning, and config-preset invariants. 14 tests.
- **Benchmarks** — Criterion suites for matching, search, validation.
- **CI** — `test.yml`, `quality.yml` (`fmt --check` + `clippy`),
  `security.yml`.

