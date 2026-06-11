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

## Doctests

The rustdoc `# Examples` on `Organization::new`,
`IdentifierScheme::is_deterministic`, `MatchConfig::{strict,lenient}`,
`Confidence::classify`, and `MatchingEngine::match_organizations` run as
doctests (`cargo test --doc`).

## Gate

`cargo test` (all green), `cargo clippy --all-targets -- -D warnings`
(clean), `cargo fmt --check` (clean). No `unwrap`/`expect`/`panic` in
library code.
