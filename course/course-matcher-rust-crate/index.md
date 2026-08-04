# course-matcher

A small, library-friendly Rust crate for pairwise matching of
course records modelled on [schema.org/Course](https://schema.org/Course).

> **Status.** Stable. Public API + 78 unit tests + a 16-test
> integration suite ([`tests/public_api.rs`](tests/public_api.rs),
> driving the re-exported surface end-to-end) + 6 `proptest` property
> tests ([`tests/proptests.rs`](tests/proptests.rs), SEC-M6 —
> never-panic, symmetry, bounded score, self-match); Soundex phonetic
> bonus (`+0.05` on the `name` component, capped at `0.95`) ships
> as T-6. A `fuzz/` `cargo-fuzz` harness (two libFuzzer targets:
> `match_courses`, `normalize`) runs on nightly, standalone from the
> normal build — see [`fuzz/README.md`](fuzz/README.md). Service-side
> bridge — 14 contract tests pinning identifier routing + deterministic
> short-circuits — lives in the embedding `course-service` crate at
> [`tests/duplicate_detection.rs`](../course-service-with-loco/tests/duplicate_detection.rs).

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

## Worked example — probabilistic partial match

When no deterministic rule fires, the score is a renormalised
weighted average and every contributing component is reported in the
`breakdown` — the crate's headline explainability feature. Here two
courses share a near-identical name and overlapping keywords but no
provider/identifier signal:

```rust
use course_matcher::{Course, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());

let mut a = Course::new("Introduction to Computer Science");
a.keywords = vec!["programming".into(), "algorithms".into()];

let mut b = Course::new("Intro to Computer Science");
b.keywords = vec!["programming".into(), "data structures".into()];

let r = engine.match_courses(&a, &b);

assert!(!r.breakdown.deterministic_match); // no R-0/R-1/R-2 fired
// Components that BOTH records carry are scored; absent ones are None
// and excluded from the divisor (renormalisation).
assert!(r.breakdown.name_score.is_some());      // high Jaro-Winkler
assert_eq!(r.breakdown.keywords_score, Some(1.0 / 3.0)); // 1 shared of 3
assert!(r.breakdown.course_code_score.is_none());
assert!(r.breakdown.provider_score.is_none());
assert!(r.breakdown.educational_level_score.is_none());
assert!(r.breakdown.teaches_score.is_none());
// score = (name * 0.35 + keywords * 0.10) / (0.35 + 0.10)
```

`match_one_to_many` keeps candidate order; `rank` sorts by descending
score (returning `(index, result)`); `find_matches` is `rank` plus a
threshold filter — pick the shape that fits the caller.

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
[`course-service`](../course-service-with-loco/)) bring those
themselves and adapt their richer Course shape down to ours.

## Test + build

```bash
cargo test --lib       # 78 unit tests (encoder + scoring + normalisation + bonus)
cargo test --test public_api  # 16 integration tests (public re-exported surface)
cargo test --test proptests   # 6 proptest property tests (SEC-M6)
cargo run              # demo binary — worked examples through the public API
cargo doc --open       # rustdoc

# Benches live in the embedding service crate so they exercise the
# adapter + facade path that production calls:
cd ../course-service-with-loco && cargo bench
```

The demo binary ([`src/main.rs`](src/main.rs)) is illustrative only and
not part of the SemVer surface. It is the sole consumer of the
musl-gated `mimalloc` allocator; the library sets no global allocator.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR
GPL-2.0-only OR GPL-3.0-only.
