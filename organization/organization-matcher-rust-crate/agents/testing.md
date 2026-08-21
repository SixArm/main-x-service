# Testing — organization-matcher

## Unit tests

Embedded in `#[cfg(test)] mod tests` blocks per source file. Run with
`cargo test --lib`.

| Module | What's covered |
|---|---|
| `organization` | `Organization::new` defaults; `IdentifierScheme::is_deterministic` for every variant (via doctest). |
| `config` | Default weights sum to 1.0; default values; `strict`/`lenient` change only the threshold. |
| `normalize` | `fold`, `legal_name` (suffix stripping, never-empty), `domain` extraction, `fold_set`. |
| `scoring` | `weighted_average` renormalisation; `Confidence::classify` bands. |
| `phonetic` | Soundex examples + `same()` contract. |
| `matcher` | Identical/legal-variant → high; R-0 LEI short-circuit; classification code does NOT short-circuit; tax-id only within jurisdiction; `same_as` overlap; unrelated → low; url/domain, jurisdiction, founding-date, address, keywords components; rank / find_matches / one-to-many. |

## Integration tests

[`tests/public_api.rs`](../tests/public_api.rs) drives only the
re-exported surface (`use organization_matcher::…`): R-0 for every
deterministic scheme, classification/`Custom`/scoped schemes NOT
short-circuiting, R-1 tax-id, R-2 `same_as`, legal-suffix high
confidence, renormalisation, threshold presets, the one-to-many surface,
and `MatchResult` JSON serialisation. Run `cargo test --test public_api`.

## Property-based tests (SEC-M6)

[`tests/property_tests.rs`](../tests/property_tests.rs) uses `proptest`
(dev-dependency) to check invariants over arbitrary UTF-8 input rather
than fixed examples: the engine (`match_organizations`,
`match_one_to_many`/`rank`/`find_matches`) and the pure helpers
(`normalize::{fold,legal_name,domain,fold_set}`, `phonetic::{soundex,same}`,
`IdentifierScheme::is_deterministic`) never panic; `score` stays in
`[0.0, 1.0]` and is never `NaN`; matching is symmetric (score, `is_match`,
`confidence`, the deterministic flag); an identical clone of a
well-formed organization self-matches; `soundex` returns `Some` iff an
ASCII-alpha anchor is present; `Confidence::classify` is monotonic. Run
`cargo test --test property_tests`.

## Fuzzing (SEC-I2)

[`fuzz/`](../fuzz/) is a standalone `cargo-fuzz` crate (not a workspace
member, so it never affects the normal stable build/test/clippy) with
two coverage-guided libFuzzer targets: `match_organizations` (JSON pair
→ `MatchingEngine::match_organizations`, finite score in `[0,1]`, both
argument orders) and `normalize` (the pure `normalize` free functions,
never-panic). Nightly-only: `cargo +nightly fuzz run <target>` from
`fuzz/` — see [`fuzz/README.md`](../fuzz/README.md).

## Doctests

The rustdoc `# Examples` on `Organization::new`,
`IdentifierScheme::is_deterministic`, `MatchConfig::{strict,lenient}`,
`Confidence::classify`, and `MatchingEngine::match_organizations` run as
doctests (`cargo test --doc`).

## Gate

`cargo test` (all green), `cargo clippy --all-targets -- -D warnings`
(clean), `cargo fmt --check` (clean). No `unwrap`/`expect`/`panic` in
library code.
