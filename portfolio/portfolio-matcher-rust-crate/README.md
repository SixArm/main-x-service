# portfolio-matcher

Pairwise **work-item (Portfolio / Project / Product / Program) record
matching** for Rust, for within-collection deduplication. Combines a
hard kind gate, deterministic identifier short-circuits, and explainable
probabilistic scoring. Dependency-light, no IO, no `unsafe`.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Entity domain model: [`../spec/index.md`](../spec/index.md) §5

## Usage

```rust
use portfolio_matcher::{WorkItem, WorkItemKind, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());
let a = WorkItem::new(WorkItemKind::Project, "Customer Onboarding Revamp");
let b = WorkItem::new(WorkItemKind::Project, "Customer Onboarding Revamp Initiative");
let r = engine.match_work_items(&a, &b);
assert!((0.0..=1.0).contains(&r.score));
```

## Matching at a glance

The matcher gates on `kind` first: two work items of different kind
(`Portfolio` / `Project` / `Product` / `Program`) **never match** — they
are distinct collections. Matching is within-kind only.

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.30 | Jaro-Winkler + Soundex bonus |
| Goals | 0.15 | Jaccard over folded goal titles |
| Code | 0.15 | Same-owner equality (1.0 / 0.0) |
| Owner org | 0.10 | Case-folded exact (1.0 / 0.0) |
| Portfolio | 0.08 | Same parent `portfolio_ref` — child kinds (1.0 / 0.0) |
| Timeframe | 0.07 | Date proximity (Gaussian decay) |
| Keywords | 0.05 | Jaccard |
| Relationships | 0.05 | Typed-set Jaccard over `(relation, work_item_id)` |
| Tags | 0.05 | Set Jaccard over normalised tags |

**Kind gate (R-GATE) → 0.0** when `A.kind != B.kind`, before any other
rule. **Deterministic short-circuit → 1.0** on a shared globally-unique
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
