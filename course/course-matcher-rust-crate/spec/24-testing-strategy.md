## 24. Testing strategy

See [`AGENTS/testing.md`](../AGENTS/testing.md). Summary:

- Unit tests in `#[cfg(test)] mod tests` blocks.
- Tests pin the deliberately-unscored fields: `learning_resource_type`
  and `in_language` are modelled on `Course` (§6) but carry no scoring
  weight (cross-language matching is out of scope, §2.2), and the
  off-ladder `EducationalLevel` variants (`Vocational`,
  `ProfessionalDevelopment`, `Custom`) earn no adjacency credit (§12).
- Property tests (`tests/proptests.rs`, `proptest`, SEC-M6): never-panic
  on arbitrary UTF-8, score always finite in `[0.0, 1.0]`, symmetry,
  self-match above threshold, well-formed Soundex output.
- Fuzzing (`fuzz/`, `cargo-fuzz`, nightly-only, standalone crate):
  `match_courses` and `normalize` libFuzzer targets (SEC-I2).
- Bridge tests live in the embedding service crate.
- Benchmarks live at
  [`course-service-with-loco/benches/matching_bench.rs`](../../course-service-with-loco/benches/matching_bench.rs)
  (driven through the service-side `CourseMatcher` facade so the
  baseline reflects production paths).

