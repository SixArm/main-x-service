# Testing — agent guide

See [`../spec.md`](../spec/index.md) for the authoritative behavioural contract. This guide is the practitioner's view of how the crate is tested.

## Test pyramid

| Layer | Location | Purpose |
|---|---|---|
| Unit tests | `#[cfg(test)] mod tests` inside each `src/*.rs` | Verify a single function or struct in isolation. |
| Integration tests | `tests/integration_tests.rs` | Exercise the public API as a downstream user would. |
| Property tests | `tests/property_tests.rs` | Quickcheck-style invariants via `proptest`. |
| Doctests | `///` examples on every public item | Keep usage examples honest. Enforced by `#![deny(missing_docs)]` plus `cargo test --doc`. |
| Examples | `examples/basic_usage.rs`, `examples/custom_config.rs`, plus the demo at `src/main.rs` | Smoke-test ergonomics; not run by `cargo test`. |
| Benches | `benches/match_pair.rs` | Criterion benches for hot paths. |

Run everything: `cargo test`. Run a single test: `cargo test test_name`. Show stdout: `cargo test -- --nocapture`. Smoke the demo: `cargo run` and `cargo run --example basic_usage`.

## Required coverage

Every public item re-exported from `lib.rs` MUST be exercised by at least one test or doctest. The doctests on `Place`, `MatchingEngine`, `MatchConfig`, `Scorer`, and `Normalizer` are the primary worked examples — keep them honest by running `cargo test --doc` before any release.

When you add a new public item, add a doctest. When you add behaviour to an existing item, add at least one integration test that pins it. See [spec-driven-development.md](./spec-driven-development.md).

## Naming conventions

- `<thing>_<expected>` or `test_<thing>_<expected>` — e.g. `perfect_match_all_fields`, `coordinates_score_far_apart_decays_to_zero`, `place_id_equality_is_scheme_scoped`.
- Use plain English in `assert!` messages when the assertion is non-obvious: `assert!(result.is_match, "expected aliases to match");`.
- Keep the test name a complete English clause readable in the `cargo test` output.

## Fixtures

Synthetic data only. Reuse the existing illustrative fixtures rather than inventing new ones (cognitive economy across the suite).

| Fixture kind | Conventional choice |
|---|---|
| Landmark coordinates | Eiffel Tower (48.8582, 2.2945), Big Ben (51.5007, -0.1247), Statue of Liberty, Wembley Stadium (51.5558, -0.2797). Public knowledge, not personal data. |
| UK postcodes | `CF10 1AA`, `SW1A 2AA`, `EC4M 7BB`, etc. |
| UK phones | Drama-reserved `07700 900xxx` ranges. |
| US phones | Fictitious `(415) 555-…` and similar `555-01xx`. |
| Emails | RFC 2606 reserved `example.org`, `example.com`, `example.net`. Gmail-folding cases MAY use `@gmail.com` with synthetic localparts (`jsmith`, `j.smith`). |
| Wikidata QIDs | `Q243` (Eiffel Tower), `Q41940` (Snowdon / Yr Wyddfa), other well-known QIDs. |
| Google Place IDs | The crate uses public IDs in spec examples; for new tests use invented `ChIJ_xyz`-style strings when the value need not be meaningful. |

## Test independence

- Tests MUST NOT share mutable state.
- No `lazy_static`, no `OnceCell`. The library is stateless; the tests should be too.
- Property tests use `proptest` with deterministic seeding. The `tests/property_tests.proptest-regressions` file is checked in so historical shrunk failure seeds are re-tried on every run.

## Coverage targets

- Statement coverage `>= 90%` on `src/`.
- Every public function MUST be exercised by at least one test or doctest.
- Coverage is not measured in CI today; treat the target as aspirational.

## What belongs in unit vs integration

- **Unit tests** validate a *function*: `normalize_phone("+44 7700 900123")` returns `"7700900123"`.
- **Integration tests** validate a *flow*: build two places, run `match_places`, assert on the result and breakdown.
- **Property tests** validate a *universal property*: "score is in `[0.0, 1.0]` for any well-formed `Place` pair".
- Do not duplicate. If integration covers a happy path, do not add a redundant unit test that asserts the same thing.

## Doctest hygiene

- Keep doctests short; one expected behaviour per block.
- Use `# use ...;` hidden lines to make examples runnable without cluttering the rendered docs.
- If a doctest needs `unwrap()`, it is fine — doctests are demonstrations, not production code. Prefer `expect("...")` with a clear message when the unwrap could plausibly fail.
- Doctests on `MatchingEngine`, `MatchConfig`, `Scorer`, `Normalizer`, `Place`, and `PlaceBuilder` are the primary worked examples downstream users will encounter. Treat them as part of the public API surface — breaking them is a behavioural change.

## Performance tests

- The `criterion` harness at `benches/match_pair.rs` exercises hot paths: single-pair `match_places` (identical / fuzzy / unrelated), `deterministic_match`, `rank_one_to_many` parameterised by `n ∈ {10, 100, 1000}`, and a config-variant sweep.
- Run with `cargo bench`. Use `--quick` for a smoke check during PR review; HTML reports land in `target/criterion/`.
- PRs that touch `MatchingEngine`, `Normalizer`, or `Scorer` SHOULD post before / after timings for at least one representative bench.

## Property tests

- `tests/property_tests.rs` uses `proptest` to pin invariants that example-based tests can miss: normalisation idempotency (`f(f(x)) == f(x)`), score bounds (`s ∈ [0.0, 1.0]`), self-match positivity (`match_places(p, p).score == 1.0` when at least one field scores), symmetry of `match_places` and `deterministic_match`, monotonicity of `Confidence::from_score`, and serde round-trips.
- When a property fails, `proptest` shrinks the input and writes a seed to `tests/property_tests.proptest-regressions`. **Commit that file** — it makes the seed a permanent fixed-input regression test for everyone who runs `cargo test`.
- New invariants belong in `proptest!` blocks if the property is naturally universally quantified (e.g. "for all `p`, …"). One-off corner cases belong in `integration_tests.rs` as ordinary `#[test]` functions.

## Negative tests

- Always include at least one negative case: a clear non-match, a missing-field place, an out-of-range coordinate (lat > 90, lon > 180, NaN), an empty `PlaceId` value (must yield `None` from `PlaceId::new`), an email without `@`.
- Negative tests guard against the "accidentally matches everything" failure mode that probabilistic systems are prone to.

## Pinning a `spec.md` section

When you write a test that pins a specific spec rule:

1. Name the test after the rule, not the implementation detail — e.g. `confidence_band_boundary_at_0_90_is_high`, not `test_from_score_branch_3`.
2. If the rule has a section identifier, reference it in a single line of comment above the test: `// Pins spec.md §3.6 — Confidence boundaries.`
3. If the spec is silent, propose an addition (see [spec-driven-development.md](./spec-driven-development.md)) before writing the test.

## Debugging a failing property test

1. `cargo test` will print the shrunk input that triggered the failure.
2. `proptest` also writes the seed to `tests/property_tests.proptest-regressions`. That seed is replayed on every subsequent run.
3. Reduce the shrunk input further by hand if needed and add it as a fixed-input `#[test]` in `integration_tests.rs` so the regression is named in the test output.
4. Fix the code (or the spec, if the property was wrong).
5. Keep the regression file committed.

## Fuzzing (SEC-I2)

`fuzz/` is a `cargo-fuzz` crate (standalone — **not** a workspace member,
so it never affects the crate's normal stable build/test/clippy) with
three coverage-guided libFuzzer targets: `match_places` (deserialize a
JSON `[place_a, place_b]` tuple → `MatchingEngine::match_places`; asserts
a finite score in `[0,1]` in both argument orders), `normalizer` (the pure
`Normalizer` helpers over arbitrary UTF-8; never-panic), and `scorer` (the
pure `Scorer` similarities; finite in `[0,1]`). It complements the
`proptest` properties above with libFuzzer's coverage-guided search over
the same never-panic / bounded-score invariants. Run on nightly:
`cargo +nightly fuzz run <target>` from `fuzz/`; see `fuzz/README.md` for
the per-target detail.

## Adapter Contract Tests

`tests/adapter_contract.rs` pins the **public API surface** that downstream
consumers (`place-service` via its `to_matcher_place` adapter) depend on. The
test exists so a rename, removal, or signature change to any public symbol
breaks the matcher's own CI **before** publish — not after publish silently
breaks downstream services.

Run with: `cargo test --test adapter_contract`

### Coverage (12 tests)

The suite touches every symbol called by the service-side adapter:

- `Place::builder()` + every fluent builder method (name / alternate_names,
  coordinates, category, place_ids, address, phone / email, and the
  geographic fields).
- PlaceBuilder full surface, PlaceId / PlaceIdScheme variants (Google, OSM*, Wikidata, Foursquare, …), PlaceCategory variant set (35 unit variants + `Other` = 36 total), Address builder (county/postcode), MatchBreakdown component fields.
- `MatchingEngine::default_config`, `MatchingEngine::new`,
  `match_places`, `deterministic_match`, `match_one_to_many`.
- `MatchResult {{ score, is_match, confidence, breakdown }}` field shape.
- `MatchBreakdown` per-component `Option<f64>` fields used by the adapter
  for explainability.
- `MatchConfig::strict / ::default / ::lenient` forming a monotonic
  threshold ladder (strict ≥ default ≥ lenient).
- `Confidence::{{High, Medium, Low}}` variants and `from_score` bucketing.
- `MatchResult` round-trip through `serde_json` (services persist results).
- Builder is `Sized` and returnable by value.

### When to update this test

Update `tests/adapter_contract.rs` **in the same PR** as any public-API
change. The purpose is to make every breaking change deliberate — never
silent. The corresponding service-side test, `place-service`'s
`tests/duplicate_detection.rs`, will already pass against the new shape
because it lives outside this crate.

### Precedent

A real prior incident from a sibling crate (illustrative, cross-crate
history — `place-matcher` itself exposes no national-identifier fields):
the worker-matcher renamed `se_personnummer` to `se_workernummer` on
crates.io 0.3.0, which broke `person-service` silently. With the contract
test in place, the rename would have failed the matcher's CI before publish.
