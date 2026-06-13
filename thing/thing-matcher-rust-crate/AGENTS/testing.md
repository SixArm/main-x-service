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

Every public item re-exported from `lib.rs` MUST be exercised by at least one test or doctest. The doctests on `Thing`, `ThingBuilder`, `Identifier`, `MatchingEngine`, `MatchConfig`, `Scorer`, and `Normalizer` are the primary worked examples — keep them honest by running `cargo test --doc` before any release.

When you add a new public item, add a doctest. When you add behaviour to an existing item, add at least one integration test that pins it. See [spec-driven-development.md](./spec-driven-development.md).

## Naming conventions

- `<subject>_<expected>` or `test_<subject>_<expected>` — e.g. `perfect_match_all_fields`, `url_score_trailing_slash_equals_root`, `identifier_equality_is_scheme_scoped`.
- Use plain English in `assert!` messages when the assertion is non-obvious: `assert!(result.is_match, "expected aliases to match");`.
- Keep the test name a complete English clause readable in the `cargo test` output.

## Fixtures

Synthetic data only. Reuse the existing illustrative fixtures rather than inventing new ones (cognitive economy across the suite).

| Fixture kind | Conventional choice |
|---|---|
| Landmark things | Eiffel Tower / La Tour Eiffel, Big Ben, Statue of Liberty. Public knowledge, not personal data. |
| Book things | Pride and Prejudice (ISBN `9780141439518`), Hamlet, The Brothers Karamazov. |
| Wikidata QIDs | `Q243` (Eiffel Tower), `Q170583` (Pride and Prejudice), `Q41940` (Snowdon / Yr Wyddfa), other well-known QIDs. |
| ISBNs | Real ISBNs of public-domain or canonical works. `9780141439518`, `9780486264646`, etc. |
| DOIs | Conventional `10.1038/…` form on a real paper, e.g. `10.1038/nature12373`. |
| Owners | Fictional or institutional names — `"Penguin Random House"`, `"British Library"` — never real individuals. |
| URLs | RFC 2606 reserved `example.org`, `example.com`, `example.net` for invented URLs. Real Wikipedia / Wikidata URLs are fine for `same_as`. |

## Test independence

- Tests MUST NOT share mutable state.
- No `lazy_static`, no `OnceCell`. The library is stateless; the tests should be too.
- Property tests use `proptest` with deterministic seeding. The `tests/property_tests.proptest-regressions` file is checked in so historical shrunk failure seeds are re-tried on every run.

## Coverage targets

- Statement coverage `>= 90%` on `src/`.
- Every public function MUST be exercised by at least one test or doctest.
- Coverage is not measured in CI today; treat the target as aspirational.

## What belongs in unit vs integration

- **Unit tests** validate a *function*: `normalize_url("HTTPS://Example.ORG/")` returns `"https://example.org"`.
- **Integration tests** validate a *flow*: build two things, run `match_things`, assert on the result and breakdown.
- **Property tests** validate a *universal property*: "score is in `[0.0, 1.0]` for any well-formed `Thing` pair".
- Do not duplicate. If integration covers a happy path, do not add a redundant unit test that asserts the same thing.

## Doctest hygiene

- Keep doctests short; one expected behaviour per block.
- Use `# use ...;` hidden lines to make examples runnable without cluttering the rendered docs.
- If a doctest needs `unwrap()`, it is fine — doctests are demonstrations, not production code. Prefer `expect("...")` with a clear message when the unwrap could plausibly fail.
- Doctests on `MatchingEngine`, `MatchConfig`, `Scorer`, `Normalizer`, `Thing`, `ThingBuilder`, and `Identifier` are the primary worked examples downstream users will encounter. Treat them as part of the public API surface — breaking them is a behavioural change.

## Performance tests

- The `criterion` harness at `benches/match_pair.rs` exercises hot paths: single-pair `match_things` (identical / fuzzy / unrelated), `deterministic_match`, `rank_one_to_many` parameterised by `n ∈ {10, 100, 1000}`, and a config-variant sweep.
- Run with `cargo bench`. Use `--quick` for a smoke check during PR review; HTML reports land in `target/criterion/`.
- PRs that touch `MatchingEngine`, `Normalizer`, or `Scorer` SHOULD post before / after timings for at least one representative bench.

## Property tests

- `tests/property_tests.rs` uses `proptest` to pin invariants that example-based tests can miss: normalisation idempotency (`f(f(x)) == f(x)`), score bounds (`s ∈ [0.0, 1.0]`), self-match positivity (`match_places(p, p).score == 1.0` when at least one field scores), symmetry of `match_places` and `deterministic_match`, monotonicity of `Confidence::from_score`, and serde round-trips.
- When a property fails, `proptest` shrinks the input and writes a seed to `tests/property_tests.proptest-regressions`. **Commit that file** — it makes the seed a permanent fixed-input regression test for everyone who runs `cargo test`.
- New invariants belong in `proptest!` blocks if the property is naturally universally quantified (e.g. "for all `p`, …"). One-off corner cases belong in `integration_tests.rs` as ordinary `#[test]` functions.

## Negative tests

- Always include at least one negative case: a clear non-match, a missing-name `Thing`, an empty `Identifier` value (must yield `None` from `Identifier::new`), a URL pair that differs only on the path (so `url_score` is `Some(0.0)`, not `Some(1.0)`), a `same_as` Jaccard with zero overlap.
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

## Adapter Contract Tests

`tests/adapter_contract.rs` pins the **public API surface** that downstream
consumers (`thing-service` via its `to_matcher_thing` adapter) depend on. The
test exists so a rename, removal, or signature change to any public symbol
breaks the matcher's own CI **before** publish — not after publish silently
breaks downstream services.

Run with: `cargo test --test adapter_contract`

### Coverage (10 tests)

The suite touches every symbol called by the service-side adapter:

- `Thing::builder()` + every fluent builder method (demographic, contact,
  identifier, address slots).
- ThingBuilder schema.org/Thing surface, Identifier::new fallibility (opaque property_id string), MatchBreakdown component fields (name, description, url, same_as, additional_types, image, main_entity_of_page, …).
- `MatchingEngine::default_config`, `MatchingEngine::new`,
  `match_things`, `deterministic_match`, `match_one_to_many`.
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
silent. The corresponding service-side test, `thing-service`'s
`tests/duplicate_detection.rs`, will already pass against the new shape
because it lives outside this crate.

### Precedent

A real prior incident: the worker-matcher renamed `se_personnummer` to
`se_workernummer` on crates.io 0.3.0, which broke `person-service`
silently. With the contract test in place, the rename would have failed
the matcher's CI before publish.
