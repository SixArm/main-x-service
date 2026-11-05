## 11. Testing Strategy

Layered: [`agents/testing.md`](../agents/testing.md).

- **Unit tests** — `#[cfg(test)]` modules; matching, phonetic,
  scoring, validation, privacy, models. ~99 tests.
- **Integration tests** — `tests/api_integration_test.rs`; full HTTP
  request/response cycles against real PostgreSQL + Tantivy. 7+ tests.
- **Bridge integration tests** — `tests/duplicate_detection.rs`;
  drives service-side records through `adapter::to_matcher_worker` and
  asserts on `MatchingEngine::match_workers` end-to-end. Covers
  identical clones, name typos, deterministic identifier short-circuits,
  field-routing pinning, and config-preset invariants. 14 tests
  (including the shared-ODS-code negative pin, spec §6.2).
- **Benchmarks** — Criterion: matching, search, validation.
- **CI** — `test.yml`, `quality.yml`, `security.yml`.

