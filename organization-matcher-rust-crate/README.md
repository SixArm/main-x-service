# organization-matcher

Pairwise **organization-record matching** for Rust, modelled on
[schema.org/Organization](https://schema.org/Organization). Combines
deterministic identifier short-circuits with explainable probabilistic
scoring. Dependency-light, no IO, no `unsafe`.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)

## Usage

```rust
use organization_matcher::{Organization, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());

let a = Organization::new("Acme, Inc.");
let b = Organization::new("ACME Corporation");
let r = engine.match_organizations(&a, &b);
// Both normalise to "acme" → name score ~1.0.
assert!(r.is_match);
```

## Matching at a glance

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.35 | Legal-suffix-aware Jaro-Winkler + Soundex bonus |
| Address | 0.20 | Weighted field-by-field Jaro-Winkler |
| URL / domain | 0.15 | Registered-domain equality, else Jaro-Winkler |
| Jurisdiction | 0.10 | Exact country (1.0 / 0.0) |
| Founding date | 0.10 | Same year 1.0, ±1yr 0.5, else 0.0 |
| Keywords | 0.10 | Jaccard |

**Deterministic short-circuit → 1.0** on a shared value of any globally
unique identifier (LEI, DUNS, ISO 6523, GLN, Wikidata, ROR, ISNI, VAT),
a same-jurisdiction tax id, or a `same_as` URL overlap.
`Naics`/`IsicV4`/`Sic` (classification) and `Custom` are evidence at
most — never pins.

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
