# Testing — course-matcher

## Unit tests

Embedded in `#[cfg(test)] mod tests` blocks in each source file.
Run with `cargo test --lib`.

### Coverage targets (78 tests today)

| Module | What's covered |
|---|---|
| `course` | `Course::new` defaults + `Course::default`; `IdentifierScheme::is_deterministic` for every variant (six deterministic, six provider-scoped); serde round-trip of a fully-populated `Course`, the `Custom` scheme label, and name-only JSON via `#[serde(default)]`. |
| `config` | Default weights sum to 1.0; default threshold + every weight pinned; `strict()` / `lenient()` change only the threshold; config serde round-trip. |
| `normalize` | `fold`, `course_code`, `fold_set` — each rule plus empty/whitespace, diacritic preservation, and NFKC compatibility folding. |
| `scoring` | `weighted_average` ignores `None` + renormalises + all-`None` → 0.0; `Confidence::classify` boundaries (inclusive lower bounds) + extremes + default. |
| `phonetic` | Russell-style examples (`Smith` → `S530`, `Robert` → `R163`); empty input returns `None`; short-code zero-padding; case-insensitivity; non-alphabetic stripping; same-group collapse; `same()` helper matches phonetic pairs while respecting the initial-letter contract and is false when either side has no letters. |
| `matcher` | Identical → 1.0; R-0 DOI short-circuit; R-1 same-provider course-code; R-2 `same_as` overlap; non-deterministic scheme + differing/empty values do NOT short-circuit; component functions (`course_code_score` skipped across providers / zero on mismatch / `None` when missing; `provider_score` exact-id + name fallback + `None`; `educational_level_score` exact/one-off/unrelated/`None`; `set_jaccard` exact fraction / both-empty `None` / one-side `0.0` / case-insensitive); name scoring uses alternate names, round-trips diacritics, empty names don't panic; renormalisation ignores absent components; `find_matches` filters below threshold; strict config rejects a merely-probable match; rank ordering; `match_one_to_many` preserves input order + handles empty input; Soundex bonus fires on homophones, doesn't fire on unrelated names, capped at `0.95`; off-ladder `EducationalLevel` variants (`Vocational`, `ProfessionalDevelopment`, `Custom`) score `1.0` only when identical and `0.0` otherwise (no adjacency credit); `learning_resource_type` and `in_language` are modelled but unscored — inputs differing only in them score identically. |

### Integration tests

[`tests/public_api.rs`](../tests/public_api.rs) drives the public
re-exported surface only (everything reachable via
`use course_matcher::…`). Run with `cargo test --test public_api`.
It is the in-crate contract test for the public API — a rename of any
re-export breaks it. Coverage: the worked example (R-1), R-0 for every
deterministic scheme, provider-scoped schemes NOT short-circuiting,
R-2 `same_as` overlap, renormalisation, confidence bands,
strict/lenient threshold effects on `is_match`, the one-to-many surface
(`match_one_to_many` / `rank` / `find_matches`, including empty input),
that `learning_resource_type` / `in_language` are unscored, and
`MatchResult` JSON serialisation. (The service-side bridge test —
see below — remains the cross-crate contract test through the adapter.)

### Property tests (SEC-M6)

[`tests/proptests.rs`](../tests/proptests.rs) (`proptest = "1.11"`,
dev-dependency only; run with `cargo test --test proptests`) proves
the matcher is robust on adversarial / arbitrary input rather than
just the hand-picked cases above. Six invariants: the engine and every
pure helper (`normalize::fold` / `course_code` / `fold_set`,
`phonetic::soundex` / `same`) never panic on arbitrary UTF-8;
`MatchResult::score` is always finite and in `[0.0, 1.0]` (never NaN);
matching is symmetric (`match(a,b) == match(b,a)` on score / `is_match`
/ confidence); an identical clone of a well-formed course self-matches
above threshold; and `soundex` output is `None` or a well-formed
`[A-Z][0-9]{3}` code.

### Fuzzing

A standalone `fuzz/` `cargo-fuzz` crate (not a workspace member, so it
never affects the normal stable build/test/clippy) ships two
coverage-guided libFuzzer targets: `match_courses` (deserialize a JSON
`[course_a, course_b]` tuple → `MatchingEngine::match_courses`; finite
score in `[0,1]`, both orders) and `normalize` (the pure `normalize`
free functions over arbitrary UTF-8, never-panic). Run on nightly:
`cargo +nightly fuzz run <target>` from `fuzz/` — see
[`fuzz/README.md`](../fuzz/README.md).

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

The embedding [`course-service`](../../course-service-with-loco/)
ships a bridge test
[`tests/duplicate_detection.rs`](../../course-service-with-loco/tests/duplicate_detection.rs)
that drives the service-side `Course` through
`matching::adapter::to_matcher_course` and asserts on
`MatchingEngine::match_courses`. That suite is the contract test for
the public surface — a rename on either side breaks it.

When the matcher's public surface changes, **also** edit the
service-side bridge test in the same PR.

## Benchmarks

Benches live in the embedding
[`course-service-with-loco/benches/matching_bench.rs`](../../course-service-with-loco/benches/matching_bench.rs)
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
