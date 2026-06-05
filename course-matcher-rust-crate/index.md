# course-matcher — index

| Document | Purpose |
|---|---|
| [spec.md](spec.md) | Single source of truth — §1–§25 (matcher shape) |
| [README.md](README.md) | User-facing intro + quick start |
| [CHANGELOG.md](CHANGELOG.md) | Keep-a-Changelog history |
| [AGENTS.md](AGENTS.md) | Agent guide |
| [CLAUDE.md](CLAUDE.md) | Re-export of `AGENTS.md` for Claude Code |

| AGENTS/ | Purpose |
|---|---|
| [AGENTS/index.md](AGENTS/index.md) | Directory index |
| [AGENTS/spec-driven-development.md](AGENTS/spec-driven-development.md) | SDD discipline |
| [AGENTS/matching-algorithm.md](AGENTS/matching-algorithm.md) | The algorithm — components, weights, deterministic rules |
| [AGENTS/normalization.md](AGENTS/normalization.md) | String normalisation rules |
| [AGENTS/testing.md](AGENTS/testing.md) | Testing strategy |

## Quick examples

### Identical courses

```rust
use course_matcher::{Course, MatchConfig, MatchingEngine};
let engine = MatchingEngine::new(MatchConfig::default());
let a = Course::new("CS101 Introduction to Computer Science");
let b = Course::new("CS101 Introduction to Computer Science");
let r = engine.match_courses(&a, &b);
assert!(r.score >= 0.99);
```

### DOI short-circuit

```rust
use course_matcher::{Course, CourseIdentifier, IdentifierScheme, MatchingEngine, MatchConfig};
let engine = MatchingEngine::new(MatchConfig::default());
let mut a = Course::new("Course A");
let mut b = Course::new("Completely different title");
a.identifiers.push(CourseIdentifier { scheme: IdentifierScheme::Doi, value: "10.1234/abc".into() });
b.identifiers.push(CourseIdentifier { scheme: IdentifierScheme::Doi, value: "10.1234/abc".into() });
let r = engine.match_courses(&a, &b);
assert_eq!(r.score, 1.0);
assert!(r.breakdown.deterministic_match);
```

### Phonetic name bonus (T-6)

```rust
use course_matcher::{Course, MatchingEngine, MatchConfig};
let engine = MatchingEngine::new(MatchConfig::default());
// "Smyth" and "Smith" both encode to Soundex S530, so name_score
// gets a +0.05 bonus (capped at 0.95) on top of the underlying
// Jaro-Winkler. Catherine ↔ Katheryn would NOT get the bonus —
// Soundex retains the first letter by design.
let r = engine.match_courses(&Course::new("Smyth"), &Course::new("Smith"));
assert!(r.breakdown.name_score.unwrap() > 0.85);
```

### One-to-many — sorted by score

```rust
let ranked: Vec<(usize, _)> = engine.rank(&query, &candidates);
// returned sorted by score descending; (index, MatchResult) per candidate
```

### One-to-many — input order (T-10)

```rust
// Parity shape with sibling matcher crates: returns Vec<MatchResult>
// in the same order as `candidates`, no sort, no threshold filter.
let results = engine.match_one_to_many(&query, &candidates);
assert_eq!(results.len(), candidates.len());
```

## See also

- [`../course-service-rust-crate/`](../course-service-rust-crate/) — embedding service
- [`../course-front-end-with-svelte/`](../course-front-end-with-svelte/) — front-end consumer
- Sibling matchers: [person](../person-matcher-rust-crate/), [event](../event-matcher-rust-crate/), [place](../place-matcher-rust-crate/), [thing](../thing-matcher-rust-crate/), [worker](../worker-matcher-rust-crate/)
