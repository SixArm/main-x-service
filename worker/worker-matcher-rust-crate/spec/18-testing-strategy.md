## 18. Testing Strategy

### 18.1 Test Pyramid

Unit tests in `src/*.rs` `#[cfg(test)]`; integration tests in `tests/integration_tests.rs`; doctests in `///` examples; smoke examples in `examples/basic_usage.rs` + `examples/custom_config.rs`. Full discipline in [`AGENTS/testing.md`](../AGENTS/testing.md).

### 18.2 Required Scenarios

Each category MUST have at least one test (unit, integration, or doctest). The verbatim per-scenario list (50+ cases) is in [`AGENTS/testing.md`](../AGENTS/testing.md):

Demographic matching (perfect / typographic / phonetic / diacritic / apostrophe / missing-field / unrelated-low-score); address (abbreviated line 1, house-number extraction, unit prefix, directional, mismatching-house-number penalty); phone (country-code / trunk-prefix variants, E.164 within-country, cross-country disambiguation, legacy fallback); national identifiers (per-scheme of the 42 — deterministic-match on identifier alone, layout-variant equivalence, check-digit rejection, scheme-locality, embedded-date validity for CN RRN / MX CURP / ZA ID, structural-invalid rejection for US SSN); passport books (single-pair, multi-country, same-digits-different-country never match, historical pair, both-empty `None`, both-disjoint `0.0`); demographics-only deterministic match + `Worker::validate` empty / solo-identifier / solo-passport; modes & thresholds (strict rejects nicknames; lenient admits more; strict + deterministic clears); transposition heuristic (DD/MM ↔ MM/DD same-year `0.5`; cross-year `0.0`; deterministic still rejects); nicknames + email + `local_id` (English-table lifts `Mike`/`Michael` etc. to ≥ 0.9; boost never lowers; email exact / mismatch / `None`; Gmail dot-folding opt-in; `local_id` not scored); serialisation round-trip + partial-JSON merge + legacy-payload defaulting.

### 18.3 Coverage Goals

Statement coverage SHOULD be `>= 90%` on `src/`. Every public function MUST have at least one direct test or doctest. `cargo test` MUST complete in `< 5 s` on commodity hardware.

### 18.4 Property Tests

Delivered as task T-6. The harness lives in `tests/property_tests.rs` and uses `proptest` with **1000 cases per property**. Properties: `normalize_name` idempotence + output-shape invariants; `score ∈ [0.0, 1.0]` for arbitrary worker pairs; self-match → `is_match` + `Confidence::High`; `match_workers` + `deterministic_match` symmetry; DOB sub-score order-independence; `Confidence::from_score` monotonicity; `MatchConfig::default()` and arbitrary `Worker` JSON round-trips. `proptest` persists shrunk failure seeds in `tests/property_tests.proptest-regressions`.

### 18.5 Adapter-Contract Tests

`tests/adapter_contract.rs` (13 tests) pins the public-API surface
that downstream service adapters depend on. Every public builder method,
every `MatchingEngine` entry point, every `MatchBreakdown` field, every
`MatchConfig` preset, and every enum variant the downstream calls is
touched. Renaming or removing any of these symbols breaks the matcher's
own CI before publish — making cross-crate breakage deliberate. See
[`AGENTS/testing.md`](../AGENTS/testing.md) for the per-section breakdown.

---

