# Testing — Agent Guide

Complements [`../spec.md`](../spec.md) §18.

## Test Pyramid

| Layer | Location | Purpose |
|---|---|---|
| Unit tests | `#[cfg(test)] mod tests` inside each `src/*.rs` | Verify a single function or struct. |
| Integration tests | `tests/integration_tests.rs` | Exercise the public API as a downstream user would. |
| Doctests | `///` examples in `lib.rs` and elsewhere | Keep usage examples honest. |
| Examples | `examples/basic_usage.rs`, `examples/custom_config.rs` | Smoke-test ergonomics; not run by `cargo test`. |

Run everything: `cargo test`. Run a single test: `cargo test test_fuzzy_name_match`. Show stdout: `cargo test -- --nocapture`.

## Required Scenarios

The list in [`../spec.md`](../spec.md) §18.2 is the spec — each scenario MUST have at least one test. When you add a feature that affects matching, add a scenario to that list.

## Naming Conventions

- `test_<thing>_<expected>` — e.g. `test_fuzzy_name_match`, `test_nhs_number_mismatch`.
- Use plain English in `assert!` messages when the assertion is non-obvious: `assert!(result.is_match, "expected fuzzy name to match");`.

## Fixtures

- Synthetic data only. Reuse the existing illustrative fixtures rather than inventing new ones; they are deliberately uncommon to avoid accidentally resembling real records.
- Do **not** use real-looking NHS numbers in tests beyond the existing illustrative `1234567890`-style synthetics. If you need a deterministic NHS-number-valid value, check it against `NHSNumber::from_str`.
- Postcodes: reuse the existing illustrative alphanumeric postcode fixtures (e.g. `CF10 1AA`). Do not use a postcode tied to a real address.
- Phone numbers: reuse the existing illustrative ranges (e.g. `07700 900xxx`, which is a drama-reserved range that is not allocated to a real subscriber). For non-UK examples, prefer fictitious area codes (`+33 1 23 45 67 89`, `(415) 555-1234`).
- Email addresses: use `example.org`, `example.com`, or `example.net` (RFC 2606 reserved); for Gmail-specific dot-folding cases the `@gmail.com` domain is acceptable with synthetic localparts (`jsmith`, `j.smith`).
- US SSNs: never reuse a known historical real SSN. Use the illustrative `123-45-6789` only; the structural-rejection tests carry their own deliberately-invalid values (`000-12-3456`, `666-12-3456`, `900-12-3456`, etc.).
- France NIRs: only use values that round-trip through `parse_fr_nir` (Mod-97 check key satisfied). The integration suite carries `NIR_A_FR` / `NIR_B_FR` constants — reuse them.
- Nicknames: when a test needs a custom equivalence class, use `with_class` with names that don't appear in the built-in English dictionary (e.g. `Reginald`/`Reggie`) to keep the test independent of dictionary additions.

## Test Independence

- Tests must not share mutable state.
- No `lazy_static`, no `OnceCell`. The library is stateless; the tests should be too.
- Property-based tests (planned, see spec §18.4) will use `proptest`. They MUST seed deterministically — pass `--proptest-seed` or use the default seeding.

## Coverage Targets

- Statement coverage `>= 90%` on `src/`.
- Every public function MUST be exercised by at least one test or doctest.
- Coverage is not measured in CI today; treat the target as aspirational until tooling is added (see §23 spec tasks for a possible follow-up).

## What Belongs in Unit vs Integration

- **Unit tests** validate a *function*: `normalize_phone("+44 7700 900123")` → `"7700900123"`.
- **Integration tests** validate a *flow*: build two persons, run `match_persons`, assert on the result + breakdown.
- Do not duplicate. If integration covers it, don't add a redundant unit test.

## Doctest Hygiene

- Keep doctests short; one expected behaviour per block.
- Use `# use ...;` hidden lines to make examples runnable without cluttering the rendered docs.
- If a doctest needs `unwrap()`, it's fine — doctests are demonstrations, not production code.

## Performance Tests

- Performance regressions are a real risk for downstream services. The `criterion` harness lives at `benches/match_pair.rs` (spec task T-5 ✅) and covers single-pair `match_persons` (identical / fuzzy / unrelated), `deterministic_match`, `rank_one_to_many` parameterised by `n ∈ {10, 100, 1000}`, and a config-variant sweep (default / strict / English nickname table).
- Run with `cargo bench`. Use `--quick` for a smoke check during PR review; HTML reports are written to `target/criterion/`.
- Single-pair budget per spec §17 is `< 50 µs`. Current single-machine numbers are well below (~4 µs fuzzy, ~160 ns deterministic ID hit). PRs that touch `MatchingEngine`, normalisation, or scorer code SHOULD post before/after timings for at least the `match_pair / fuzzy_near_match` and `rank_one_to_many / 100` benches.

## Property Tests

- `tests/property_tests.rs` uses `proptest` at **1000 cases per property** (spec §18.4, task T-6 ✅). The properties pin invariants that example-based tests can miss: normalisation idempotency, score bounds, self-match positivity, symmetry, monotonicity of `Confidence::from_score`, and serde round-trips.
- When a property fails, `proptest` shrinks the input and writes a seed to `tests/property_tests.proptest-regressions`. **Commit that file** — it makes the seed a permanent fixed-input regression test for everyone who runs `cargo test`. If you decide the property was wrong and the input is legitimate, fix or remove the property (and delete the relevant seed line) rather than silencing the failure.
- New invariants belong in `proptest!` blocks if the property is naturally universally quantified (e.g. "for all X, …"). One-off corner cases belong in `integration_tests.rs` as ordinary `#[test]` functions — proptest is not free, and a known failing input is cheaper to express as a literal.

## Negative Tests

- Always include at least one negative case: a clear non-match, a missing-field person, an unparseable NHS number.
- Negative tests guard against the "accidentally matches everything" failure mode that probabilistic systems are prone to.
