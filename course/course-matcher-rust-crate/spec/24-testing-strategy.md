## 24. Testing strategy

See [`AGENTS/testing.md`](../AGENTS/testing.md). Summary:

- Unit tests in `#[cfg(test)] mod tests` blocks.
- Tests pin the deliberately-unscored fields: `learning_resource_type`
  and `in_language` are modelled on `Course` (§6) but carry no scoring
  weight (cross-language matching is out of scope, §2.2), and the
  off-ladder `EducationalLevel` variants (`Vocational`,
  `ProfessionalDevelopment`, `Custom`) earn no adjacency credit (§12).
- Bridge tests live in the embedding service crate.
- Benchmarks live at
  [`course-service-with-loco/benches/matching_bench.rs`](../../course-service-with-loco/benches/matching_bench.rs)
  (driven through the service-side `CourseMatcher` facade so the
  baseline reflects production paths).

