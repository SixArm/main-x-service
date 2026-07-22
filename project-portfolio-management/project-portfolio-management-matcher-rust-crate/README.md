# project-portfolio-management-matcher

Pairwise **plan record matching** for Rust, for deduplication across one
recursive plan collection. A *plan* may contain other plans (a
recursive tree); the four former kinds (Portfolio / Project / Product /
Program) are unified into one entity, with `kind` surviving only as
optional descriptive metadata that does not gate matching. Combines
deterministic identifier short-circuits and explainable probabilistic
scoring. Dependency-light, no IO, no `unsafe`.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Entity domain model: [`../spec/index.md`](../spec/index.md) §5

## Usage

```rust
use project_portfolio_management_matcher::{Plan, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());
let a = Plan::new("Customer Onboarding Revamp");
let b = Plan::new("Customer Onboarding Revamp Initiative");
let r = engine.match_plans(&a, &b);
assert!((0.0..=1.0).contains(&r.score));
```

## Matching at a glance

There is **no kind gate**: all plans live in one recursive collection,
so any two plans may match regardless of their (optional) `kind`
(`Portfolio` / `Project` / `Product` / `Program` / `Practice` /
`Process` / `Purpose` / `Pathway` / `Proposal`), which is descriptive
metadata only and is never compared.

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.30 | Jaro-Winkler + Soundex bonus |
| Goals | 0.15 | Jaccard over folded goal titles |
| Code | 0.15 | Same-owner equality (1.0 / 0.0) |
| Owner org | 0.10 | Case-folded exact (1.0 / 0.0) |
| Parent | 0.08 | Same parent `parent_ref` (1.0 / 0.0) |
| Timeframe | 0.07 | Date proximity (Gaussian decay) |
| Keywords | 0.05 | Jaccard |
| Relationships | 0.05 | Typed-set Jaccard over `(relation, plan_id)` |
| Tags | 0.05 | Set Jaccard over normalised tags |

**Deterministic short-circuit → 1.0** on a shared globally-unique
identifier (URI, UUID, Jira project key, Asana GID, Trello board id, MS
Project id, GitHub project id, Linear id), a same-owner code, or a
`same_as` URL overlap. Owner-scoped codes (`Code`/`LocalId`) and
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
