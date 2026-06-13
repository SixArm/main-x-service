## 18. Testing Strategy

### 18.1 Test Pyramid

Unit tests (`src/*.rs` `#[cfg(test)]` modules), integration tests (`tests/integration_tests.rs`), doctests (`///` examples in `lib.rs` and elsewhere), and runnable examples (`examples/basic_usage.rs`, `examples/custom_config.rs`).

### 18.2 Required Scenarios

Scenario classes (each ≥ one test): matching baselines (perfect / unrelated / missing-field); name variants (typographic / phonetic / diacritic / apostrophe); address handling (abbreviation, house-number, unit prefix, directional, mismatched-house penalty); phone normalisation (`+CC` / `00CC` / `07…`, E.164 within-country, cross-country disambiguation, legacy fallback); deterministic identifier-only match (one test per scheme: 42 + passport-book); scheme locality (UK United Kingdom National Health Service Number ≠ UK NI H&C ≠ UK CHI; AU IHI ≠ IE IHI; NL ID ≠ NL BSN; PL NIP ≠ PL PESEL; UK NINO never cross-matches); per-parser validation (canonical / case-whitespace / check-digit / length / character / variant); configuration modes (strict rejects nickname-only; lenient admits more partial; demographics-alone deterministic); DOB transposition (`0.5` swap; deterministic rejects; cross-year doesn't fire); `Person::validate` (rejects empty; accepts solo identifier / non-empty `passport_books`); serde round-trips for `Person` and `MatchResult` (all 42 identifiers, `#[serde(default)]` legacy defaulting); nicknames lift to ≥ 0.9 one-way (default empty table); email canonical equality (opt-in Gmail folding, `local_id` not scored).

### 18.3 Coverage Goals

Statement coverage SHOULD be `≥ 90%` on `src/`. Every public function MUST have at least one direct test or doctest. `cargo test` MUST complete in `< 5 s` on commodity hardware.

### 18.4 Property Tests

Delivered as T-6. `tests/property_tests.rs` uses `proptest` with **1000 cases per property**. Properties: `normalize_name` idempotence + shape (no uppercase / no leading or trailing whitespace); `score ∈ [0.0, 1.0]`; self-match `is_match == true` and `Confidence::High`; `match_persons` / `deterministic_match` symmetry; `MatchConfig::default()` and `Person` JSON round-trip; DOB sub-score order-independence; `Confidence::from_score` monotonicity. Shrunk failure seeds in `tests/property_tests.proptest-regressions`.

---

### 18.5 Adapter-Contract Tests

`tests/adapter_contract.rs` (13 tests) pins the public-API surface
that downstream service adapters depend on. Every public builder method,
every `MatchingEngine` entry point, every `MatchBreakdown` field, every
`MatchConfig` preset, and every enum variant the downstream calls is
touched. Renaming or removing any of these symbols breaks the matcher's
own CI before publish — making cross-crate breakage deliberate. See
[`AGENTS/testing.md`](../AGENTS/testing.md) for the per-section breakdown.

---

