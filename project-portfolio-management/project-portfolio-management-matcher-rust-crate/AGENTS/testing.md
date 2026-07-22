# Testing — project-portfolio-management-matcher

## Unit tests

Embedded in `#[cfg(test)] mod tests` blocks per source file. Run with
`cargo test --lib`.

| Module | What's covered |
|---|---|
| `plan` | `Plan::new(name)` defaults (`kind` defaults to `None`); `IdentifierScheme::is_deterministic` (doctest); `PlanKind` is closed (no `Custom`). |
| `config` | Default weights sum to 1.0; `strict`/`lenient` change only the threshold. |
| `normalize` | `fold`, `code` (alphanumeric-only), `fold_set`. |
| `scoring` | `weighted_average` renormalisation; `Confidence::classify` bands. |
| `phonetic` | Soundex examples + `same()` contract. |
| `matcher` | `kind` does not gate (differing kinds still match; `kind_gate_blocked` always `false`); identical → high; R-0 deterministic-scheme short-circuit (Jira / URI / UUID / …); owner-scoped code not across owners; `same_as` overlap; goal-title Jaccard; owner-org exact; parent-ref exact; timeframe Gaussian decay; relationships / tags typed-set & set Jaccard; unrelated → low; rank / find_matches / one-to-many. |

## Integration tests

[`tests/public_api.rs`](../tests/public_api.rs) drives only the
re-exported surface (`use project_portfolio_management_matcher::…`): that `kind` does not
gate (differing kinds still match), R-0 for every deterministic scheme,
owner-scoped/`Custom` NOT short-circuiting, R-1 code, R-2 `same_as`,
goals / owner / parent / timeframe corroboration, relationships + tags supporting signals,
renormalisation, threshold presets, the one-to-many surface, and
`MatchResult` JSON serialisation. Run `cargo test --test public_api`.

## Gate

`cargo test` (all green), `cargo clippy --all-targets --all-features --
-D warnings` (clean — mirrors CI), `cargo fmt --check` (clean). No
`unwrap`/`expect`/`panic` and no `#[allow(clippy::…)]` in library code
(clippy-clean without suppressions).
