# Architecture — agent guide

The authoritative description of the crate's data model, scoring pipeline, and safety guarantees lives in [`../spec.md`](../spec/index.md) §3 and §8. This guide is the practitioner's view of the module layout and the layering rules.

## Module layout

```text
src/
├── lib.rs        ← re-exports only; top-level crate docs
├── main.rs       ← demo binary (NOT part of the library API)
├── models.rs     ← Event, EventBuilder, Address, Location, EventCategory, EventStatus, EventAttendanceMode, EventId, EventIdScheme
├── normalizer.rs ← Normalizer + ParsedAddressLine; text / phone / address / email / phonetic / ISO-8601 transforms
├── scorer.rs     ← Scorer (Jaro-Winkler, Levenshtein, Combined, Haversine, Gaussian decay) + SimilarityAlgorithm
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
         ├── normalizer   (text / date transforms)
         ├── scorer       (similarity + geographic primitives)
         └── error        (error enum + Result alias)
```

### Rules

- `models` MUST NOT depend on any other module in this crate.
- `normalizer` and `scorer` MUST NOT depend on `matcher` (no upward references).
- `matcher` is the only orchestration layer. Component scoring helpers live as methods on `MatchingEngine` so they can read `self.config`. Pure helpers that don't need config live as free functions in the same file.
- `lib.rs` is **only** re-exports — no behaviour.
- `main.rs` is a demo binary; it is **not** part of the library API surface and is excluded from the `#![deny(missing_docs)]` audit obligation that applies to the library.

### Why these rules?

- Keeping `models` dependency-free means `Event` can be reused in upstream apps that don't pull in `strsim` or `unicode-normalization` at compile time.
- Keeping `matcher` as the only orchestrator means there is one place to read to understand "what happens when you call `match_events`" — see `spec.md` §5.

## Public surface

The only items considered API are re-exported from `lib.rs`:

```rust
pub use error::{MatchingError, Result};
pub use matcher::{Confidence, MatchBreakdown, MatchConfig, MatchResult, MatchingEngine};
pub use models::{
    Address, Event, EventAttendanceMode, EventBuilder, EventCategory, EventId, EventIdScheme,
    EventStatus, Location,
};
pub use normalizer::{Normalizer, ParsedAddressLine};
pub use scorer::{Scorer, SimilarityAlgorithm};
```

Anything not re-exported is private to the crate and may be refactored freely.

## The `Event` data model

`Event` carries 26 fields today, every one optional or defaulting to empty. The canonical field list and which fields are scored (vs data-only) lives in `spec.md` §3.1.1, including `relationships: Vec<RelationshipRef>` and `tags: Vec<String>` (both supporting-only signals, never consulted by `deterministic_match`).

`#[non_exhaustive]` on `Event`, `Address`, `Location`, `EventCategory`, `EventStatus`, `EventAttendanceMode`, `EventIdScheme`, and `MatchingError` formalises that adding variants / fields is a non-breaking change (`spec.md` §9.1). Downstream code MUST construct via `Event::builder()`, `Address::new()`, and `Location::new()` (plus fluent `with_*` setters), not struct-literal syntax.

## Match pipeline

`MatchingEngine` exposes four public methods:

- `match_events(&e1, &e2) -> MatchResult` — single-pair probabilistic match. (`spec.md` §5.2)
- `deterministic_match(&e1, &e2) -> bool` — single-pair deterministic match. (`spec.md` §5.1)
- `match_one_to_many(&query, candidates) -> Vec<MatchResult>` — score query against a slice, preserve order. (`spec.md` §5.3)
- `rank_one_to_many(&query, candidates) -> Vec<(usize, MatchResult)>` — same, sorted by descending score with ascending-index tiebreak. (`spec.md` §5.3)

### Probabilistic path

1. Compute per-field component scores (`Option<f64>` each — `spec.md` §6): name, start date, end date, location, category, country code, event IDs, organizer, performers, URL.
2. Sum `score × weight` over fields that scored; sum the participating weights.
3. If the phonetic-name gate fires (`name_phonetic_score > 0.9`), add `score × 0.05` to the weighted sum and `0.05` to the total weight.
4. Divide weighted sum by total weight (or `0.0` if no field scored).
5. Bucket into `Confidence` via `Confidence::from_score`, and compare against `match_threshold` for `is_match`. Under strict mode `is_match` additionally requires `deterministic_match`.

### Deterministic path

1. Return `true` if any `(scheme, value)` pair is shared across the two `event_ids` lists.
2. Otherwise return `true` if both `name` values normalise equal AND both `start_date` values parse to the same Unix instant (`Normalizer::parse_iso8601_unix_seconds`).
3. Otherwise return `false`.

Note there is **no window-overlap** scoring: `start_date`/`end_date` are scored independently by Gaussian decay over endpoint distance (`spec.md` §6.3). Scoring the `[start, end]` overlap fraction instead is tracked as an open question (`spec.md` §10 OQ-C), not implemented.

## Adding a new module

If you genuinely need a new module:

1. Add `pub mod foo;` in `lib.rs`.
2. Decide its layer (data / utility / orchestration).
3. Update [`../spec.md`](../spec/index.md) (module layout and dependency graph) **before** writing code.
4. If it exposes public types, add re-exports in `lib.rs` and document the new public surface in the spec.

## Avoid these shapes

- Trait objects (`dyn Whatever`) where a `Copy` enum like `SimilarityAlgorithm` would suffice. The current design dispatches on the enum.
- Builder structs with hidden default state. `EventBuilder` is `#[derive(Default)]`; preserve that pattern.
- Struct-literal construction of `Event`, `Address`, `Location`, `EventCategory`, `EventStatus`, `EventAttendanceMode`, `EventIdScheme`, or `MatchingError` from outside the crate. All carry `#[non_exhaustive]`.
- "God modules" — if `matcher.rs` grows past ~1,500 lines, split scoring helpers into a child file/module under `matcher/`, not into a peer.

## Threading and sharing

- `MatchingEngine` is immutable after construction. Cloning it is cheap (only the config).
- All public types are `Send + Sync` because their fields are. Don't add `Rc`, `Cell`, or interior mutability.
- Consumers that want parallel evaluation can wrap `match_one_to_many` in `rayon::par_iter` or `tokio::task::spawn_blocking` without changes to this crate.

## When refactoring

- Refactors that do not change behaviour do not need a spec update, **but** they should not be bundled with behaviour changes. Keep diffs reviewable.
- If you split a module, update the spec.
- If you change the dependency graph, update the spec.
