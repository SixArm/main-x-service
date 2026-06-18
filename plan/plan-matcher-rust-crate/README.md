# plan-matcher

Pairwise **plan (project / product / programme / initiative / portfolio
/ epic) record matching** for Rust, for portfolio deduplication.
Combines deterministic identifier short-circuits with explainable
probabilistic scoring. Dependency-light, no IO, no `unsafe`.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)

## Usage

```rust
use plan_matcher::{Plan, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());
let a = Plan::new("Customer Onboarding Revamp");
let b = Plan::new("Customer Onboarding Revamp Initiative");
let r = engine.match_plans(&a, &b);
assert!((0.0..=1.0).contains(&r.score));
```

## Matching at a glance

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.30 | Jaro-Winkler + Soundex bonus |
| Goals | 0.15 | Jaccard over folded goal titles |
| Plan code | 0.15 | Same-owner equality (1.0 / 0.0) |
| Owner org | 0.10 | Case-folded exact (1.0 / 0.0) |
| Plan type | 0.08 | Exact enum (1.0 / 0.0) |
| Timeframe | 0.07 | Date proximity (Gaussian decay) |
| Keywords | 0.05 | Jaccard |
| Relationships | 0.05 | Typed-set Jaccard over `(relation, plan_id)` |
| Tags | 0.05 | Set Jaccard over normalised tags |

**Deterministic short-circuit → 1.0** on a shared globally-unique
identifier (URI, UUID, Jira project key, Asana GID, Trello board id, MS
Project id, GitHub project id, Linear id), a same-owner plan code, or a
`same_as` URL overlap. Owner-scoped codes (`PlanCode`/`LocalId`) and
`Custom` are evidence at most.

## Run the demo

```bash
cargo run
```

## Testing

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
