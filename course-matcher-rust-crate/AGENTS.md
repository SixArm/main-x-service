# AGENTS — Course Matcher

`course-matcher` is a small, dependency-light library crate for
pairwise course-record matching. Its canonical artefact is
[`spec.md`](spec.md) — when code and spec disagree, the spec wins.

## Crate-local docs (`AGENTS/`)

| Document | Description |
|---|---|
| [AGENTS/index.md](AGENTS/index.md) | Directory index |
| [AGENTS/spec-driven-development.md](AGENTS/spec-driven-development.md) | SDD discipline — three-part PRs |
| [AGENTS/matching-algorithm.md](AGENTS/matching-algorithm.md) | The algorithm — components, weights, deterministic rules |
| [AGENTS/normalization.md](AGENTS/normalization.md) | String normalisation rules (case-fold, NFKC, course-code shape) |
| [AGENTS/testing.md](AGENTS/testing.md) | Unit + bridge test strategy |

## Public surface

```rust
use course_matcher::{
    Course, CourseIdentifier, IdentifierScheme,
    EducationalLevel, LearningResourceType,
    MatchingEngine, MatchConfig,
    MatchResult, MatchBreakdown, Confidence,
};

let engine = MatchingEngine::new(MatchConfig::default());
let r: MatchResult = engine.match_courses(&a, &b);
```

The shape mirrors the family-wide matcher convention used by
[`person-matcher`](../person-matcher-rust-crate/) +
[`event-matcher`](../event-matcher-rust-crate/) so service crates can
adapt one code path across all entities.

## Where work lives

| Concern | Location |
|---|---|
| Behavioural truth | [`spec.md`](spec.md) (§1–§25; live task queue in §13) |
| Algorithm impl | `src/matcher.rs` |
| Score helpers | `src/scoring.rs` |
| Normalisation | `src/normalize.rs` |
| Domain types | `src/course.rs` |
| Public re-exports | `src/lib.rs` |
| Unit tests | inline `#[cfg(test)]` blocks (no `tests/` dir yet) |
