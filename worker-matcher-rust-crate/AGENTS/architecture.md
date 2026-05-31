# Architecture — Agent Guide

This guide complements [`../spec.md`](../spec.md) §9 and §10. Read both before making structural changes.

## Layering

```
lib.rs               (re-exports only)
   │
   └── matcher       (orchestration — depends on the others)
         │
         ├── models         (data types, no logic)
         ├── identifiers    (parse_uk_nhs_number, parse_fr_nir, parse_es_tsi,
         │                   parse_ie_ihi, parse_uk_hc_number, parse_us_ssn)
         ├── normalizer     (text + phone + address + email + phonetic transforms)
         ├── scorer         (similarity primitives)
         ├── nicknames      (NicknameTable equivalence-class lookup)
         └── error          (error enum + Result alias)
```

### Rules
- `models` MUST NOT depend on any other module in this crate.
- `identifiers` MUST NOT depend on `matcher`, `normalizer`, or `scorer`. It is a leaf module beneath `matcher`.
- `normalizer`, `scorer`, and `nicknames` MUST NOT depend on `matcher` (no upward references). `nicknames` may depend on `normalizer` (to normalise class entries at insertion time).
- `matcher` is the only orchestration layer. Component scoring helpers live as methods on `MatchingEngine` so they can read `self.config`.
- `lib.rs` is **only** re-exports — no behaviour.
- `main.rs` is a demo binary; it is **not** part of the library API surface.

### Why these rules?
- Keeping `models` dependency-free means `Worker` can be reused in upstream apps that don't pull in `strsim` or `unicode-normalization` at compile time.
- Keeping `matcher` as the only orchestrator means there is one place to read to understand "what happens when you call `match_workers`".

## Public Surface

The only items considered API are re-exported from `lib.rs`:

```rust
pub mod identifiers;  // free-function parsers, one per scheme

pub use error::{MatchingError, Result};
pub use matcher::{Confidence, MatchBreakdown, MatchConfig, MatchResult, MatchingEngine};
pub use models::{Address, Gender, Worker, WorkerBuilder};
pub use nicknames::NicknameTable;
pub use normalizer::{Normalizer, ParsedAddressLine};
pub use scorer::{Scorer, SimilarityAlgorithm};
```

Anything not re-exported is private to the crate and may be refactored freely.

## Adding a New Module

If you genuinely need a new module:

1. Add `pub mod foo;` in `lib.rs`.
2. Decide its layer (data / utility / orchestration).
3. Update [`../spec.md`](../spec.md) §9.1 and §9.2 (module layout and dependency graph) **before** writing code.
4. If it exposes public types, add an entry in §11 (public API).

## Avoid These Shapes

- ❌ Trait objects (`dyn Whatever`) where a `Copy` enum like `SimilarityAlgorithm` would suffice. The current design dispatches on the enum.
- ❌ Builder structs with hidden default state. `WorkerBuilder` is `#[derive(Default)]`; preserve that pattern.
- ❌ Struct-literal construction of `Worker` or `Address` from outside the crate. Both carry `#[non_exhaustive]`; consumers must use `Worker::builder()` or `Address::new().with_*(...)` fluent setters. New fields are then non-breaking under SemVer.
- ❌ "God modules" — if `matcher.rs` grows past ~1,000 lines, split scoring helpers into `matcher/scoring.rs` (a child file/module), not into a peer. The current file already carries the full breakdown wiring, weighted-sum reducer, and per-scheme identifier helpers — keep new helpers grouped by concern.

## Threading and Sharing

- `MatchingEngine` is immutable after construction. Cloning it is cheap (only the config).
- All public types are `Send + Sync` because their fields are. Don't add `Rc`, `Cell`, or interior mutability.

## When Refactoring

- Refactors that do not change behaviour do not need a spec update, **but** they should not be bundled with behaviour changes. Keep diffs reviewable.
- If you split a module, update §9.1 of the spec.
- If you change the dependency graph, update §9.2.
