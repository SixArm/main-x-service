## 11. Testing Strategy

Layered: [`agents/testing.md`](../agents/testing.md).

- **Unit tests** — `#[cfg(test)]` modules; models (32), matching
  (50), validation (25), privacy (8), search (6), streaming (2),
  metrics (1), api (1). 125 tests (`cargo test --lib`).
- **Integration tests** — `tests/integration_*.rs`; end-to-end
  workflows + edge cases (unicode names, geo poles, date line, GLN
  deterministic override, address normalisation edge cases, GDPR
  field preservation, geo-radius candidate filtering, matcher-bridge
  worked example). 72 tests across the `integration_*.rs` files.
- **Bridge integration tests** — `tests/duplicate_detection.rs`;
  drives service-side records through `adapter::to_matcher_place` and
  asserts on `MatchingEngine::match_places` end-to-end. Covers
  identical clones, name typos, deterministic identifier short-circuits,
  field-routing pinning, and config-preset invariants. 14 tests.
  (Integration total in `tests/`: 86.)
- **Benchmarks** — Criterion: 16 — matching, search, validation,
  privacy.
- **CI** — `quality.yml` (fmt + clippy `-D warnings` + `cargo test`).

