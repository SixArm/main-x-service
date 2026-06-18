# Testing — plan-matcher

## Unit tests

Embedded in `#[cfg(test)] mod tests` blocks per source file. Run with
`cargo test --lib`.

| Module | What's covered |
|---|---|
| `plan` | `Plan::new` defaults; `IdentifierScheme::is_deterministic` (doctest). |
| `config` | Default weights sum to 1.0; `strict`/`lenient` change only the threshold. |
| `normalize` | `fold`, `plan_code` (alphanumeric-only), `fold_set`. |
| `scoring` | `weighted_average` renormalisation; `Confidence::classify` bands. |
| `phonetic` | Soundex examples + `same()` contract. |
| `matcher` | Identical → high; R-0 deterministic-scheme short-circuit (Jira / URI / UUID / …); owner-scoped plan code not across owners; `same_as` overlap; goal-title Jaccard; owner-org exact; plan-type exact; timeframe Gaussian decay; relationships / tags typed-set & set Jaccard; unrelated → low; rank / find_matches / one-to-many. |

## Integration tests

[`tests/public_api.rs`](../tests/public_api.rs) drives only the
re-exported surface (`use plan_matcher::…`): R-0 for every deterministic
scheme, owner-scoped/`Custom` NOT short-circuiting, R-1 plan code, R-2
`same_as`, goals / owner / type / timeframe corroboration,
relationships + tags supporting signals, renormalisation, threshold
presets, the one-to-many surface, and `MatchResult` JSON serialisation.
Run `cargo test --test public_api`.

## Gate

`cargo test` (all green), `cargo clippy --all-targets --all-features --
-D warnings` (clean — mirrors CI), `cargo fmt --check` (clean). No
`unwrap`/`expect`/`panic` and no `#[allow(clippy::…)]` in library code
(clippy-clean without suppressions).
