# Testing Strategy & Guide

## Overview

The MPI uses a multi-layer testing strategy: unit tests, integration tests, and benchmark tests.

## Unit Tests

Run with: `cargo test --lib`

Unit tests are embedded in source files using `#[cfg(test)]` modules. They test individual functions and modules without external dependencies (no database, no network).

### Test Coverage by Module

| Module                      | Tests | What's Tested                                                                       |
| --------------------------- | ----- | ----------------------------------------------------------------------------------- |
| `matching::algorithms`      | 8+    | Name matching, DOB matching, gender, address, identifier, tax ID, document matching |
| `matching::phonetic`        | 4+    | Soundex encoding, matching, similarity scoring                                      |
| `matching::scoring`         | 5+    | Probabilistic scoring, deterministic scoring, match quality classification          |
| `matching::mod`             | 3+    | ProbabilisticMatcher, DeterministicMatcher, MatchScoreBreakdown                     |
| `search::index`             | 3+    | Schema creation, index creation, open/create                                        |
| `search::mod`               | 5+    | Index and search, fuzzy search, bulk indexing, deletion, name+year search           |
| `validation`                | 3+    | Valid person, missing family name, phone normalization, address standardization    |
| `privacy`                   | 2+    | Value masking, person masking                                                      |
| `models::person`           | 2+    | Person construction, serialization                                                 |
| `models::document`          | 2+    | Document types, serialization                                                       |
| `models::emergency_contact` | 2+    | Construction, serialization                                                         |
| `lib`                       | 1     | Module imports                                                                      |

### Running Specific Tests

```bash
cargo test --lib                              # All unit tests
cargo test --lib test_person_matcher        # Tests matching "test_person_matcher"
cargo test --lib matching::                   # All matching module tests
cargo test --lib -- --nocapture               # With stdout output
```

## Integration Tests

Run with: `cargo test --test api_integration_test`

**Requires:** PostgreSQL database running (see docker-compose.test.yml)

Integration tests are in `tests/` and test full HTTP request/response cycles against real dependencies.

### Test Files

| File                            | Tests | What's Tested                                             |
| ------------------------------- | ----- | --------------------------------------------------------- |
| `tests/api_integration_test.rs` | 7+    | Health check, CRUD lifecycle, search, match, merge, audit |
| `tests/common/mod.rs`           | —     | Shared test setup, helpers, test app creation             |

### Running Integration Tests

```bash
# With Docker
docker-compose -f docker-compose.test.yml up

# Locally (requires running PostgreSQL)
DATABASE_URL=postgres://user:pass@localhost/mpi_test cargo test --test api_integration_test
```

## Benchmark Tests

Run with: `cargo bench`

Benchmark tests use Criterion for statistical benchmarking.

### Benchmark Files

| File                          | What's Benchmarked                                               |
| ----------------------------- | ---------------------------------------------------------------- |
| `benches/matching_bench.rs`   | Name matching, full person matcher, phonetic encoding          |
| `benches/search_bench.rs`     | Person indexing, full-text search, fuzzy search                 |
| `benches/validation_bench.rs` | Person validation, phone normalization, address standardization |

### Running Benchmarks

```bash
cargo bench                           # All benchmarks
cargo bench -- matching               # Only matching benchmarks
cargo bench -- search                 # Only search benchmarks
```

## Test Utilities

### Creating Test Persons

```rust
use person_service::models::*;

let person = Person::new(
    HumanName {
        use_type: None,
        family: "Smith".to_string(),
        given: vec!["John".to_string()],
        prefix: vec![],
        suffix: vec![],
    },
    Gender::Male,
);
```

### Temporary Search Index

```rust
let temp_dir = tempfile::tempdir().unwrap();
let engine = SearchEngine::new(temp_dir.path().to_str().unwrap()).unwrap();
```

## CI/CD Testing

GitHub Actions workflows:

| Workflow       | What It Does                                             |
| -------------- | -------------------------------------------------------- |
| `test.yml`     | Unit tests + integration tests (with PostgreSQL service) |
| `quality.yml`  | `cargo fmt --check` + `cargo clippy`                     |
| `security.yml` | Security scanning                                        |

### CI Test Pipeline

1. **unit-tests** job: `cargo test --lib --verbose` + `cargo test --doc --verbose`
2. **integration-tests** job: PostgreSQL 18 service → migrations → `cargo test --test api_integration_test`
3. **test-summary** job: Consolidates results

## Writing New Tests

### Unit Test Guidelines

1. Place tests in `#[cfg(test)] mod tests` at the bottom of the source file
2. Use descriptive names: `test_<function>_<scenario>`
3. Test both success and failure paths
4. Test edge cases: empty strings, None values, boundary conditions
5. Use `Person::new()` for test person construction
6. Use `tempfile::tempdir()` for search index paths

### Integration Test Guidelines

1. Place tests in `tests/` directory
2. Use `tests/common/mod.rs` for shared setup
3. Test full HTTP request/response cycles
4. Verify status codes and response body structure
5. Test error cases (404, 422, 409)
6. Clean up test data after each test

## Bridge Integration Tests

`tests/duplicate_detection.rs` is a black-box test that drives the
service-side domain model through [`matching::adapter::to_matcher_person`]
and asserts on `MatchingEngine::match_persons` output. The suite pins
**both sides of the contract** — the adapter's field-routing rules and
the matcher's scoring algorithm — so a regression on either side fails
a test here.

Run with: `cargo test --test duplicate_detection`

### Coverage (14 tests)

| Category | What it pins |
|---|---|
| Identical / near-duplicate | identical-clone score ≥ 0.95, name-typo fuzzy match, ordering invariants (closer-evidence outscores farther) |
| Deterministic short-circuits | NHS number deterministic match (FHIR system URI routing), shared tax_id default-routed to US SSN, passport-book identity, DOB off-by-one ordering, strict ⊆ lenient config invariant |
| Negative cases | unrelated records score low, common-name + divergent demographics not flagged as duplicate |
| Field-routing pinning | per-adapter mapping tests (telecom → phone/email, address field renames, identifier-system-URI routing) |
| Edge cases | sparse records, empty fields, config presets |

### Running

```bash
cargo test --test duplicate_detection                       # all bridge tests
cargo test --test duplicate_detection identical             # just the identical-clone tests
cargo test --test duplicate_detection -- --nocapture        # with stdout
```

### When to add a new test here

Add a bridge test when:

- The adapter (`src/matching/adapter.rs`) gains a new routing rule.
- The person-matcher crate exposes a new scoring component the service
  needs to surface.
- A regression escapes the adapter's own `#[cfg(test)] mod tests`.
