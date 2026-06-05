# Testing — course-matcher

## Unit tests

Embedded in `#[cfg(test)] mod tests` blocks in each source file.
Run with `cargo test --lib`.

### Coverage targets (21 tests today)

| Module | What's covered |
|---|---|
| `course` | Default construction, serde round-trip, `IdentifierScheme::is_deterministic` for every variant. |
| `config` | Default weights sum to 1.0; `strict()` / `lenient()` thresholds. |
| `normalize` | `fold`, `course_code`, `fold_set` — pin each rule. |
| `scoring` | `weighted_average` ignores `None`; `Confidence::classify` boundaries. |
| `phonetic` | Russell-style examples (`Smith` → `S530`, `Robert` → `R163`); empty input returns `None`; short-code zero-padding; `same()` helper matches phonetic pairs while respecting the initial-letter contract. |
| `matcher` | Identical → 1.0; DOI short-circuit; same-provider course-code short-circuit; same_as overlap; unrelated → low; rank ordering; `match_one_to_many` preserves input order + handles empty input; Soundex bonus fires on homophones, doesn't fire on unrelated names, capped at `0.95`. |

### Pattern

```rust
#[test]
fn descriptive_name() {
    let engine = MatchingEngine::default_config();
    let a = Course::new("…");
    let b = Course::new("…");
    let r = engine.match_courses(&a, &b);
    assert!(/* directionality */);
}
```

Total functions, no `unwrap` / `expect` in library code, no `panic`
on bad input — tests pin those guarantees.

## Bridge tests (service-side)

The embedding [`course-service`](../../course-service-rust-crate/)
ships a bridge test
[`tests/duplicate_detection.rs`](../../course-service-rust-crate/tests/duplicate_detection.rs)
that drives the service-side `Course` through
`matching::adapter::to_matcher_course` and asserts on
`MatchingEngine::match_courses`. That suite is the contract test for
the public surface — a rename on either side breaks it.

When the matcher's public surface changes, **also** edit the
service-side bridge test in the same PR.

## Benchmarks

Benches live in the embedding
[`course-service-rust-crate/benches/matching_bench.rs`](../../course-service-rust-crate/benches/matching_bench.rs)
so the baseline reflects the production path (adapter +
`CourseMatcher` facade) rather than the bare library. Run with
`cargo bench` from that crate. Coverage:

- `match_courses/populated_pair` — full all-components scoring on
  two fully-populated courses.
- `match_courses/deterministic_short_circuit` — R-0 path via a DOI
  identifier match.
- `find_matches/rank_100_candidates` — ranking 100 candidates,
  the typical block-and-score path.

## When tests fail

Symptom decoder:

| Symptom | Likely cause |
|---|---|
| Score drift on identical inputs | Default weight or threshold changed in `MatchConfig::default`. |
| New false-positive at 1.0 | A new variant added to `IdentifierScheme::is_deterministic` without bridge-test coverage. |
| Stale serde shape | Variant renamed in `IdentifierScheme` / `EducationalLevel` / `LearningResourceType` without coordinating with the service-side adapter. |
| Course-code component disappears | A test forgot to set `provider_id` on both sides. |
| Phonetic bonus suddenly stops firing | Soundex is initial-letter-preserving; check that both names share the first letter (`Smyth↔Smith` matches; `Catherine↔Katheryn` does not). |
| Phonetic bonus never caps | The bonus must clamp at `0.95`; an unbounded `best += 0.05` would let a near-typo + Soundex match cross into High confidence. |
