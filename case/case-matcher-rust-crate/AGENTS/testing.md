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
| `matcher` | Identical → high; R-0 docket / external-case-id short-circuit; agency-scoped case number not across agencies; agency-name fallback; `same_as` overlap; subjects Jaccard; case-type / status exact; unrelated → low; rank / find_matches / one-to-many. |

## Integration tests

[`tests/public_api.rs`](../tests/public_api.rs) drives only the
re-exported surface (`use case_matcher::…`): R-0 for every
deterministic scheme, agency-scoped/`Custom` NOT short-circuiting,
R-1 case number, the across-agency non-cross-match, R-2 `same_as`,
subjects/type corroboration, status discrimination, renormalisation,
threshold presets, the one-to-many surface, and `MatchResult` JSON
serialisation. Run `cargo test --test public_api`.

## Gate

`cargo test` (all green), `cargo clippy --all-targets -- -D warnings`
(clean), `cargo fmt --check` (clean). No `unwrap`/`expect`/`panic` in
library code.
