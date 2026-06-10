## 24. Testing strategy

See [`AGENTS/testing.md`](../AGENTS/testing.md). Summary:

- Unit tests in `#[cfg(test)] mod tests` blocks.
- Bridge tests live in the embedding service crate.
- Benchmarks live at
  [`course-service-rust-crate/benches/matching_bench.rs`](../../course-service-rust-crate/benches/matching_bench.rs)
  (driven through the service-side `CourseMatcher` facade so the
  baseline reflects production paths).

