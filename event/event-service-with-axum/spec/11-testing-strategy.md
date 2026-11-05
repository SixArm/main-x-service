## 11. Testing Strategy

Layered: [`agents/testing.md`](../agents/testing.md).

- **Unit tests** — `#[cfg(test)]` modules; matching, scoring,
  validation (including attendance-mode/location coupling and capacity
  invariants), privacy, models, time-interval algebra.
- **Integration tests** — `tests/api_integration_test.rs`; full HTTP
  request/response cycles against real PostgreSQL + Tantivy. Two more
  files are gated further: `tests/enforcement.rs` (`#[ignore]`d,
  real-router blanket-enforcement + ABAC proof, own test binary) and
  `tests/fluvio_relay.rs` (feature-gated on `fluvio` **and**
  `#[ignore]`d, needs a live broker).
- **Bridge integration tests** — `tests/duplicate_detection.rs`;
  drives service-side records through `adapter::to_matcher_event` and
  asserts on `MatchingEngine::match_events` end-to-end. Covers
  identical clones, name typos, deterministic identifier short-circuits,
  field-routing pinning, and config-preset invariants.
- **Benchmarks** — Criterion for matching, search, validation, and the
  adapter/matcher bridge (`bridge_bench.rs`).
- **CI** — `test.yml`, `quality.yml`, `security.yml`.

