# Testing — care-pathway-matcher

## Unit tests

Embedded in `#[cfg(test)] mod tests` blocks per source file. Run with
`cargo test --lib`.

| Module | What's covered |
|---|---|
| `care_pathway` | `CarePathway::new` defaults; `IdentifierScheme::is_deterministic` (doctest). |
| `config` | Default weights sum to 1.0; `strict`/`lenient` change only the threshold. |
| `normalize` | `fold`, `pathway_code` (alphanumeric-only), `fold_set`. |
| `scoring` | `weighted_average` renormalisation; `Confidence::classify` bands. |
| `phonetic` | Soundex examples + `same()` contract. |
| `matcher` | Identical → high; R-0 DOI / guideline-id short-circuit; provider-scoped pathway code not across providers; `same_as` overlap; condition-code Jaccard; care-setting exact; unrelated → low; rank / find_matches / one-to-many. |

## Integration tests

[`tests/public_api.rs`](../tests/public_api.rs) drives only the
re-exported surface (`use care_pathway_matcher::…`): R-0 for every
deterministic scheme, provider-scoped/`Custom` NOT short-circuiting,
R-1 pathway code, R-2 `same_as`, condition/setting corroboration,
renormalisation, threshold presets, the one-to-many surface, and
`MatchResult` JSON serialisation. Run `cargo test --test public_api`.

## Gate

`cargo test` (all green), `cargo clippy --all-targets -- -D warnings`
(clean), `cargo fmt --check` (clean). No `unwrap`/`expect`/`panic` in
library code.
