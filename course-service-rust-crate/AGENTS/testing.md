# Testing strategy & guide — Course Service

## Test categories

### Unit tests

Embedded in source files via `#[cfg(test)] mod tests`. Run with
`cargo test --lib`.

| Module | What's covered |
|---|---|
| `models::course` | Construction, serde round-trip, `Course::new` defaults |
| `models::course_instance` | Schedule shape, status enum, serde round-trip |
| `models::identifier` | `IdentifierType::is_deterministic` for every variant |
| `matching::adapter` | Service-`Course` → matcher-`Course` field routing (planned T-6) |
| `validation` | FR-21..FR-28 (planned T-5) |
| `search` | Schema creation, index lifecycle (planned T-4) |
| `privacy` | Masking + GDPR export (planned T-10) |

### Integration tests

In `tests/`. Run with `cargo test --tests`. Require Postgres
via `docker-compose.test.yml`.

| File | Coverage |
|---|---|
| `tests/api_integration_test.rs` | Health + CRUD + search + match + merge + audit |
| `tests/duplicate_detection.rs` | Bridge test pinning matcher contract (T-11) |
| `tests/common/mod.rs` | Shared test harness, test-app construction |

### Bridge integration tests

`tests/duplicate_detection.rs` drives the service-side `Course`
through `matching::adapter::to_matcher_course` and asserts on
`course_matcher::MatchingEngine::match_courses` output. The suite
pins **both sides of the contract** — the adapter's field-routing
rules and the matcher's scoring algorithm — so a regression on
either side fails a test here.

Run with: `cargo test --test duplicate_detection`

When to add a new bridge test:

- The adapter (`src/matching/adapter.rs`) gains a new routing rule.
- The course-matcher crate exposes a new scoring component the
  service needs to surface.
- A regression escapes the adapter's own `#[cfg(test)] mod tests`.

### Benchmark tests

In `benches/` (planned T-13). Use Criterion.

| File | What's measured |
|---|---|
| `benches/matching_bench.rs` | Name match, full course match, soundex |
| `benches/search_bench.rs` | Index, full-text, fuzzy |
| `benches/validation_bench.rs` | FR-21..FR-28 |

## Running tests

```bash
# All unit tests
cargo test --lib

# Integration tests (requires Postgres)
podman compose -f docker-compose.test.yml up -d
DATABASE_URL=… cargo test --tests

# Bridge tests
cargo test --test duplicate_detection

# Benchmarks
cargo bench
```

## Writing new tests

### Unit-test pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let course = Course::new("CS101");

        // Act
        let json = serde_json::to_string(&course).unwrap();
        let round_trip: Course = serde_json::from_str(&json).unwrap();

        // Assert
        assert_eq!(round_trip.name, "CS101");
    }
}
```

### Integration-test pattern

Use `tests/common/mod.rs::create_test_app_state` to build an
`AppState` with a per-test tempdir Tantivy index. Each test creates
its own records with a timestamped name so re-runs are idempotent.
Cleanup via `softDelete` in `Drop` or `tear_down`.

## Test-data conventions

- Use well-known canonical courses for readability — "CS101
  Introduction to Computer Science", "MAT221 Linear Algebra".
- Use realistic schema.org-style URLs in `same_as` (Wikidata Q
  identifiers, OER repositories).
- Use `Course::new("name")` for the minimal test course; layer on
  fields only when the test requires them.
