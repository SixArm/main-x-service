# Testing — Thing Entity orientation

Three test suites plus one cross-subproject contract pin. Per-crate
detail lives in the per-crate guides — this page is the map.

## Per-subproject

| Subproject | Commands | Guide |
|---|---|---|
| Service | `cargo test --lib` (~100 unit) · `cargo test --tests` (integration) · `cargo bench` (Criterion) | [service AGENTS/testing.md](../thing-service-rust-crate/AGENTS/testing.md) |
| Matcher | `cargo test` (unit + integration + property + doctests) · `cargo clippy --all-targets -- -D warnings` · `cargo doc --no-deps` — all three must be clean | [matcher AGENTS/testing.md](../thing-matcher-rust-crate/AGENTS/testing.md) |
| Front-end | `pnpm test` — 8 Vitest unit tests + 6 Playwright e2e smoke tests | [front-end AGENTS/testing.md](../thing-front-end-with-svelte/AGENTS/testing.md) |

## The entity-level pin: bridge tests

```bash
cd thing-service-rust-crate
cargo test --test duplicate_detection
```

[`tests/duplicate_detection.rs`](../thing-service-rust-crate/tests/duplicate_detection.rs)
— 15 black-box tests driving service-shaped records through
`adapter::to_matcher_thing` into `MatchingEngine::match_things`. They
pin **both** the adapter's field routing and the matcher's scoring:

- identical-clone score ≥ 0.95; name-typo fuzzy match; ordering
  invariants
- DOI / ISBN / UUID deterministic short-circuits; different-ISBN
  reject; SKU non-deterministic distinction; `Custom` passthrough;
  shared `same_as` contribution
- negative cases, sparse records, config presets

**Rule:** any change to entity spec §5.3, to the adapter, or to
matcher scoring updates a bridge test in the same PR.

## When you change cross-subproject behaviour

| Change | Minimum test edits |
|---|---|
| Adapter routing rule | Bridge test + adapter `#[cfg(test)]` |
| Matcher weight / normalisation | Matcher unit/property test; re-run bridge tests |
| REST endpoint the UI consumes | Service integration test + front-end Vitest (repository) + Playwright route |
| Confidence vocabulary mapping (T-8) | Bridge test pinning the mapping |

## Conventions

- Canonical, recognisable test things (books with real ISBNs, papers
  with real DOIs); never sibling-entity-flavoured data.
- No real personal data anywhere; the matcher mandates synthetic
  fixtures (RFC 2606 domains, reserved phone ranges).
- Entity-level acceptance for the front-end requires a **live**
  service walkthrough, still pending — see entity spec
  [§13 T-5](../spec/13-tasks.md).
