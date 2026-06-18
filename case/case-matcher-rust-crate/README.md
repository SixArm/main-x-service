# case-matcher

Pairwise **governmental case-management record matching** for Rust.
Combines deterministic identifier short-circuits with explainable
probabilistic scoring. Dependency-light, no IO, no `unsafe`.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)

## Usage

```rust
use case_matcher::{Case, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());
let a = Case::new("Housing benefit appeal — J. Smith");
let b = Case::new("Housing benefit appeal — John Smith");
let r = engine.match_cases(&a, &b);
assert!((0.0..=1.0).contains(&r.score));
```

## Matching at a glance

| Component | Weight | Algorithm |
|---|---:|---|
| Title | 0.30 | Jaro-Winkler + Soundex bonus |
| Subjects | 0.25 | Jaccard over folded subject strings |
| Case number | 0.15 | Same-agency equality (1.0 / 0.0) |
| Case type | 0.10 | Exact enum (1.0 / 0.0) |
| Status | 0.05 | Exact enum (1.0 / 0.0) |
| Keywords | 0.15 | Jaccard |

`priority`, `opened_date`, and `in_language` are carried for
downstream consumers but are **never scored**.

**Deterministic short-circuit → 1.0** on a shared globally-unique
identifier (`Docket`, `ExternalCaseId`, URI, UUID), a same-agency case
number, or a `same_as` URL overlap. Agency-scoped codes
(`AgencyCaseNumber`/`LocalId`) and `Custom` are evidence at most.

## Run the demo

```bash
cargo run
```

## Testing

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
