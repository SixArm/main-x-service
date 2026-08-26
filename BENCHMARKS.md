# Benchmarks

Criterion benchmark suites ship per crate (matchers, services,
libraries) and are compiled — proven to link — by the CI `bench`
stage:

```sh
scripts/ci-check.sh bench                 # compile all benches
cd person/person-matcher-rust-crate
cargo bench                               # run one crate's benches
```

## Measured results worth knowing

Numbers below were measured, not estimated; see `tasks.md` (PERF-*)
for the full write-ups.

- **Tantivy indexing cost is per-commit, not per-document** (PERF-2,
  2026-08-20): `add_document` ≈ 0.02 ms, `commit()` ≈ **96 ms**,
  reader reload ≈ 0.6 ms. Durability (fsync) is 0.02–0.2% of the
  cost. Consequence: batching commits is the whole fix, and per-row
  indexing caps a service at ~10 writes/second.
- **Input-bounding pays for itself** (SEC-M8b, 2026-08-21):
  care-pathway's over-long-array rejection path dropped 112 µs →
  4.9 µs when the error report was bounded along with the work.
- **Bulk import streams flat** (SEC-B2, 2026-08-05): ~0.19 MiB peak
  allocation on a ~312 MiB JSONL import, measured by a counting
  global allocator (`person-service` `tests/bulk_streaming_memory.rs`).

## Method

Benchmarks use Criterion; memory measurements use an instrumented
global allocator in a dedicated test. When adding a benchmark, assert
the property you care about in a test where possible — a number nobody
compares is not a gate.
