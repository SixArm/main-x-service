# Testing — course-matcher

## Unit tests

Embedded in `#[cfg(test)] mod tests` blocks in each source file.
Run with `cargo test --lib`.

### Coverage targets

| Module | What's covered |
|---|---|
| `course` | Default construction, serde round-trip, `IdentifierScheme::is_deterministic` for every variant. |
| `config` | Default weights sum to 1.0; `strict()` / `lenient()` thresholds. |
| `normalize` | `fold`, `course_code`, `fold_set` — pin each rule. |
| `scoring` | `weighted_average` ignores `None`; `Confidence::classify` boundaries. |
| `matcher` | Identical → 1.0; DOI short-circuit; same-provider course-code short-circuit; same_as overlap; unrelated → low; rank ordering. |

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

Out of MVP scope. Once `criterion` is wired in, the benches live
under `benches/` and cover:

- Name match (exact, near-match, unrelated).
- Full `match_courses` (worst-case all-components).
- `rank` against N = 50 / 100 / 1000 candidates.
- `normalize::fold` throughput.

## When tests fail

Symptom decoder:

| Symptom | Likely cause |
|---|---|
| Score drift on identical inputs | Default weight or threshold changed in `MatchConfig::default`. |
| New false-positive at 1.0 | A new variant added to `IdentifierScheme::is_deterministic` without bridge-test coverage. |
| Stale serde shape | Variant renamed in `IdentifierScheme` / `EducationalLevel` / `LearningResourceType` without coordinating with the service-side adapter. |
| Course-code component disappears | A test forgot to set `provider_id` on both sides. |
