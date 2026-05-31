# Architecture — agent guide

The authoritative description of the crate's data model, scoring pipeline, and safety guarantees lives in [`../spec.md`](../spec.md) §3 and §5. This guide is the practitioner's view of the module layout and the layering rules.

## Module layout

```text
src/
├── lib.rs        ← re-exports only; top-level crate docs
├── main.rs       ← demo binary (NOT part of the library API)
├── models.rs     ← Thing, ThingBuilder, Identifier
├── normalizer.rs ← Normalizer; name / text / URL / phonetic transforms
├── scorer.rs     ← Scorer (Jaro-Winkler, Levenshtein, Combined, Exact, Jaccard) + SimilarityAlgorithm
├── matcher.rs    ← MatchingEngine, MatchConfig, MatchResult, MatchBreakdown, Confidence
└── error.rs      ← MatchingError + Result alias
```

## Layering

```text
lib.rs               (re-exports only)
   │
   └── matcher       (orchestration — depends on the others)
         │
         ├── models       (data types, no logic)
         ├── normalizer   (text transforms)
         ├── scorer       (similarity primitives)
         └── error        (error enum + Result alias)
```

### Rules

- `models` MUST NOT depend on any other module in this crate.
- `normalizer` and `scorer` MUST NOT depend on `matcher` (no upward references).
- `matcher` is the only orchestration layer. Component scoring helpers live as methods on `MatchingEngine` so they can read `self.config`. Pure helpers that don't need config live as free functions in the same file.
- `lib.rs` is **only** re-exports — no behaviour.
- `main.rs` is a demo binary; it is **not** part of the library API surface and is excluded from the `#![deny(missing_docs)]` audit obligation that applies to the library.

### Why these rules?

- Keeping `models` dependency-free means `Thing` can be reused in upstream apps that don't pull in `strsim` or `unicode-normalization` at compile time.
- Keeping `matcher` as the only orchestrator means there is one place to read to understand "what happens when you call `match_things`" — see `spec.md` §5.

## Public surface

The only items considered API are re-exported from `lib.rs`:

```rust
pub use error::{MatchingError, Result};
pub use matcher::{Confidence, MatchBreakdown, MatchConfig, MatchResult, MatchingEngine};
pub use models::{Identifier, Thing, ThingBuilder};
pub use normalizer::Normalizer;
pub use scorer::{Scorer, SimilarityAlgorithm};
```

Anything not re-exported is private to the crate and may be refactored freely.

## The `Thing` data model

`Thing` carries 13 fields, every one optional or defaulting to empty. The canonical field list and which fields are scored (vs data-only) lives in `spec.md` §3.1.

`#[non_exhaustive]` on `Thing` and `MatchingError` formalises that adding fields / variants is a non-breaking change (`spec.md` §7.3). Downstream code MUST construct via `Thing::builder()` rather than struct-literal syntax. `Identifier` provides `Identifier::new(property_id, value) -> Option<Self>` (rejecting empty components) — there is no public struct-literal path because both fields must be validated.

## Match pipeline

`MatchingEngine` exposes four public methods:

- `match_things(&t1, &t2) -> MatchResult` — single-pair probabilistic match. (`spec.md` §5.2)
- `deterministic_match(&t1, &t2) -> bool` — single-pair deterministic match. (`spec.md` §5.1)
- `match_one_to_many(&query, candidates) -> Vec<MatchResult>` — score query against a slice, preserve order. (`spec.md` §5.3)
- `rank_one_to_many(&query, candidates) -> Vec<(usize, MatchResult)>` — same, sorted by descending score with ascending-index tiebreak. (`spec.md` §5.3)

### Probabilistic path

1. Compute per-field component scores (`Option<f64>` each — `spec.md` §6).
2. Sum `score × weight` over fields that scored; sum the participating weights.
3. If the phonetic-name gate fires (`name_phonetic_score > 0.9`), add `score × 0.05` to the weighted sum and `0.05` to the total weight.
4. Divide weighted sum by total weight (or `0.0` if no field scored).
5. Bucket into `Confidence` via `Confidence::from_score`, and compare against `match_threshold` for `is_match`. Under strict mode `is_match` additionally requires `deterministic_match`.

### Deterministic path

1. Return `true` if any `(property_id, value)` pair is shared across the two `identifiers` lists.
2. Otherwise return `true` if both things share any `same_as` URL after `normalize_url`.
3. Otherwise return `true` if both things have a `url` and the two normalise equal.
4. Otherwise return `false`.

## Adding a new module

If you genuinely need a new module:

1. Add `pub mod foo;` in `lib.rs`.
2. Decide its layer (data / utility / orchestration).
3. Update [`../spec.md`](../spec.md) (module layout and dependency graph) **before** writing code.
4. If it exposes public types, add re-exports in `lib.rs` and document the new public surface in the spec.

## Avoid these shapes

- Trait objects (`dyn Whatever`) where a `Copy` enum like `SimilarityAlgorithm` would suffice. The current design dispatches on the enum.
- Builder structs with hidden default state. `ThingBuilder` is `#[derive(Default)]`; preserve that pattern.
- Struct-literal construction of `Thing` or `MatchingError` from outside the crate. Both carry `#[non_exhaustive]`.
- Public struct-literal construction of `Identifier`. Use `Identifier::new` so empty-component rejection runs.
- "God modules" — if `matcher.rs` grows past ~1,500 lines, split scoring helpers into a child file/module under `matcher/`, not into a peer.

## Threading and sharing

- `MatchingEngine` is immutable after construction. Cloning it is cheap (only the config).
- All public types are `Send + Sync` because their fields are. Don't add `Rc`, `Cell`, or interior mutability.
- Consumers that want parallel evaluation can wrap `match_one_to_many` in `rayon::par_iter` or `tokio::task::spawn_blocking` without changes to this crate.

## When refactoring

- Refactors that do not change behaviour do not need a spec update, **but** they should not be bundled with behaviour changes. Keep diffs reviewable.
- If you split a module, update the spec.
- If you change the dependency graph, update the spec.
