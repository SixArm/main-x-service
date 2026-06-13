# course-matcher

A small, library-friendly Rust crate for pairwise matching of
course records modelled on [schema.org/Course](https://schema.org/Course).

> **Status.** Stable. Public API + 21 unit tests; Soundex phonetic
> bonus (`+0.05` on the `name` component, capped at `0.95`) ships
> as T-6. Service-side bridge — 14 contract tests pinning identifier
> routing + deterministic short-circuits — lives in the embedding
> `course-service` crate at
> [`tests/duplicate_detection.rs`](../course-service-rust-crate/tests/duplicate_detection.rs).

## Quick start

```rust
use course_matcher::{Course, IdentifierScheme, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());

let mut a = Course::new("Introduction to Computer Science");
a.course_code   = Some("CS101".into());
a.provider_id   = Some("ror-021nxhr62".into());
a.keywords      = vec!["programming".into(), "algorithms".into()];

let mut b = Course::new("Intro to Computer Science");
b.course_code   = Some("CS 101".into());
b.provider_id   = Some("ror-021nxhr62".into());

let result = engine.match_courses(&a, &b);
assert!(result.is_match);                 // 1.0 — same provider + same course code
assert!(result.breakdown.deterministic_match);
```

## Algorithm

Two strategies:

- **Deterministic** — score 1.0 when both records share (a) a value
  on a deterministic identifier scheme (DOI, Wikidata, LOM, OER,
  URI, UUID), (b) `provider_id` + `course_code` (normalised), or
  (c) a `same_as` URL.
- **Probabilistic** — weighted average over the components both
  records actually carry. Absent fields don't penalise the score
  (renormalisation).

Default weights:

| Component | Weight |
|---|---|
| Name (Jaro-Winkler, with alternate-name fallbacks) | 0.35 |
| Provider-scoped course code | 0.15 |
| Provider (`provider_id` exact / `provider_name` Jaro-Winkler) | 0.15 |
| Educational level (with adjacent-ladder partial credit) | 0.10 |
| Keywords (Jaccard on folded set) | 0.10 |
| Teaches / competencies (Jaccard) | 0.15 |

Match `is_match` threshold: 0.85. Confidence bands: ≥0.95 High,
≥0.70 Medium, otherwise Low.

Full derivation: [`spec.md`](spec/index.md) +
[`AGENTS/matching-algorithm.md`](AGENTS/matching-algorithm.md).

## Public surface

```rust
use course_matcher::{
    Course, CourseIdentifier, IdentifierScheme,
    EducationalLevel, LearningResourceType,
    MatchingEngine, MatchConfig,
    MatchResult, MatchBreakdown, Confidence,
};
```

## Dependencies (intentionally tiny)

- `strsim` for Jaro-Winkler.
- `unicode-normalization` for NFKC normalisation.
- `serde` / `serde_json` for round-trip.
- `thiserror` for the error enum.
- `mimalloc` — demo binary only, under a `musl` target gate.

No `axum`, no `tokio`, no `tantivy`. Embedding services (e.g.
[`course-service`](../course-service-rust-crate/)) bring those
themselves and adapt their richer Course shape down to ours.

## Test + build

```bash
cargo test --lib       # 21 unit tests (encoder + scoring + normalisation + bonus)
cargo run              # demo binary — worked examples through the public API
cargo doc --open       # rustdoc

# Benches live in the embedding service crate so they exercise the
# adapter + facade path that production calls:
cd ../course-service-rust-crate && cargo bench
```

The demo binary ([`src/main.rs`](src/main.rs)) is illustrative only and
not part of the SemVer surface. It is the sole consumer of the
musl-gated `mimalloc` allocator; the library sets no global allocator.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR
GPL-2.0-only OR GPL-3.0-only.
