# care-pathway-matcher

Pairwise **care-pathway (clinical pathway) record matching** for Rust.
Combines deterministic identifier short-circuits with explainable
probabilistic scoring. Dependency-light, no IO, no `unsafe`.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)

## Usage

```rust
use care_pathway_matcher::{CarePathway, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());
let a = CarePathway::new("Acute Stroke Care Pathway");
let b = CarePathway::new("Acute Stroke Pathway");
let r = engine.match_care_pathways(&a, &b);
assert!((0.0..=1.0).contains(&r.score));
```

## Matching at a glance

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.30 | Jaro-Winkler + Soundex bonus |
| Condition codes | 0.25 | Jaccard over `system:code` tokens (ICD/SNOMED) |
| Pathway code | 0.15 | Same-provider equality (1.0 / 0.0) |
| Care setting | 0.10 | Exact enum (1.0 / 0.0) |
| Interventions | 0.10 | Jaccard |
| Keywords | 0.10 | Jaccard |

**Deterministic short-circuit → 1.0** on a shared globally-unique
identifier (DOI, Wikidata, guideline-registry id, URI, UUID), a
same-provider pathway code, or a `same_as` URL overlap. Provider-scoped
codes (`PathwayCode`/`LocalId`) and `Custom` are evidence at most.

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
