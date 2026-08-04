# Testing — case-matcher

## Unit tests

Embedded in `#[cfg(test)] mod tests` blocks per source file. Run with
`cargo test --lib`.

| Module | What's covered |
|---|---|
| `case` | `Case::new` defaults; `IdentifierScheme::is_deterministic` (doctest). |
| `config` | Default weights sum to 1.0; `strict`/`lenient` change only the threshold. |
| `normalize` | `fold`, `case_number` (alphanumeric-only), `url`, `fold_set`. |
| `scoring` | `weighted_average` renormalisation; `Confidence::classify` bands. |
| `phonetic` | Soundex examples + `same()` contract. |
| `matcher` | Identical → high; R-0 docket / external-case-id short-circuit; agency-scoped case number not across agencies; agency-name fallback; `same_as` overlap; subjects + keywords Jaccard; Soundex title bonus (lift + clamp + symmetry); `alternate_titles` contribution; case-type / status exact; unrelated → low; rank / find_matches / one-to-many. |

## Integration tests

[`tests/public_api.rs`](../tests/public_api.rs) drives only the
re-exported surface (`use case_matcher::…`): R-0 for every
deterministic scheme, agency-scoped/`Custom` NOT short-circuiting,
R-1 case number, the across-agency non-cross-match, R-2 `same_as`,
subjects/type corroboration, status discrimination, renormalisation,
threshold presets, the one-to-many surface, the documented enum serde
wire shape (unit variants vs `Custom`), and `MatchResult` JSON
serialisation. Run `cargo test --test public_api`.

## Property-based tests

[`tests/proptests.rs`](../tests/proptests.rs) (`proptest`, dev-only)
drives random inputs to pin: the engine never panics; `MatchResult`
scores (and every breakdown sub-score) are finite and in `[0.0, 1.0]`;
`match_cases` is symmetric in argument order; and the pure helpers
(`normalize::*`, `phonetic::soundex`/`same`, `Confidence::classify`)
never panic on arbitrary UTF-8/floats. Run `cargo test --test
proptests`.

## Fuzzing (SEC-I2)

A standalone [`fuzz/`](../fuzz/) `cargo-fuzz` crate — not a workspace
member, so it never affects the gate below — carries two
coverage-guided libFuzzer targets: `match_cases` (JSON-deserialized
`[case_a, case_b]` → `MatchingEngine::match_cases`, asserting a finite
`[0,1]` score in both argument orders) and `normalize` (the pure
`normalize` functions, never-panic over arbitrary UTF-8). Requires a
nightly toolchain; see [`fuzz/README.md`](../fuzz/README.md) for
`cargo +nightly fuzz run <target>`.

## Gate

`cargo test` (all green), `cargo clippy --all-targets --all-features
-- -D warnings` (clean), `cargo fmt --check` (clean). No
`unwrap`/`expect`/`panic` in library code.
