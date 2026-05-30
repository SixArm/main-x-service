# Testing strategy — Main Thing Service

## Test categories

### Unit tests

Embedded in source files via `#[cfg(test)] mod tests`. Run with `cargo test --lib`.

| Module                  | What's covered                                                                                                  |
| ----------------------- | --------------------------------------------------------------------------------------------------------------- |
| `models::thing`         | Construction, defaults, address/geo, serialization, soft delete                                                 |
| `models::address`       | Construction, fields, serialization, partial                                                                    |
| `models::geo`           | Construction, elevation, Haversine (same point, known distance, short, antipodal), serialization                |
| `models::thing_type`    | Display, equality, serialization, `Other` variant                                                               |
| `models::identifier`    | GLN, custom, serialization                                                                                      |
| `models::amenity`       | Construction, with value                                                                                        |
| `models::opening_hours` | Construction, serialization                                                                                     |
| `models::consent`       | Active, revoked, expired by date, not yet expired                                                               |
| `matching::name`        | Exact, case-insensitive, similar, different, empty, both empty, substring, prefix bonus                         |
| `matching::address`     | Identical, different, partial, no overlap, case-insensitive                                                     |
| `matching::geo`         | Same point, close, moderate, far, within radius (true/false), custom reference                                  |
| `matching::identifier`  | Matching/different GLN, empty, mixed, `has_gln_match` (true, false type, false value)                           |
| `matching::phonetic`    | Robert, Rupert, match, no match, Ashcraft, empty, single char, case, Washington, thing names                    |
| `matching::scoring`     | Identical things, name only, different, GLN deterministic, confidence levels, weights sum, fuzzy, phonetic bonus|
| `validation`            | Valid thing, empty/whitespace name, invalid lat/lon, GLN format, URL, telephone, address, normalization         |
| `privacy`               | Mask telephone/fax/geo, preserve name, no sensitive fields, short phone, GDPR export, export fields             |

### Integration tests

In `tests/`. Run with `cargo test --tests`.

| File                          | What's covered                                                                                                       |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `integration_matching.rs`     | Exact duplicate, typo match, completely different, same name different city, GLN override, name only, batch         |
| `integration_validation.rs`   | Validate-normalize workflow, invalid thing handling, full lifecycle                                                 |
| `integration_privacy.rs`      | Mask-export workflow, full GDPR export, immutability, soft delete export                                            |
| `integration_models.rs`       | Construction serialization, soft delete timestamps, unique IDs, thing hierarchy, geo symmetry, consent lifecycle    |
| `integration_scoring.rs`      | Unicode names, edge cases, geo poles/date line, identifier edge cases, custom weights, confidence boundaries        |
| `integration_edge_cases.rs`   | Boundary coordinates, GLN length, URL protocols, address minimal/empty fields, GLN deterministic override           |

### Benchmark tests

In `benches/`. Run with `cargo bench` (Criterion).

| File                        | What's measured                                                                                                              |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `matching_bench.rs`         | `name_similarity` (exact/fuzzy/different), `geo_similarity` (close/far), Soundex (short/long), full match, batch_match_100   |
| `validation_bench.rs`       | `validate_simple`, `validate_full`, `normalize_thing`                                                                        |
| `searching_bench.rs`        | `search_by_name_100`, `search_by_name_fuzzy_100`                                                                             |
| `database_reading_bench.rs` | `thing_construction`, `thing_batch_construction_100`                                                                         |
| `database_writing_bench.rs` | `thing_create_and_validate`, `thing_create_and_normalize`                                                                    |
| `privacy_bench.rs`          | `mask_thing`, `mask_thing_minimal`, `gdpr_export`, `gdpr_export_batch_100`                                                   |

## Running tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Specific module
cargo test --lib models::thing
cargo test --lib matching::scoring

# Integration tests only
cargo test --tests

# Specific integration test
cargo test --test integration_matching

# With output
cargo test -- --nocapture

# Benchmarks
cargo bench

# Specific benchmark
cargo bench -- name_similarity
```

## Writing new tests

### Unit test pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let input = setup_test_data();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Integration test pattern

```rust
// tests/integration_feature.rs
use main_thing_service::models::thing::Thing;

#[test]
fn test_end_to_end_workflow() {
    let thing = Thing::new("Test");
    let validated = validate_thing(&thing);
    let matched = compute_match(&thing, &other, &weights);

    assert!(validated.is_empty());
    assert!(matched.score > 0.8);
}
```

## Test data conventions

- Use well-known things for readability (Central Park, Eiffel Tower, etc.)
- Use realistic coordinates (NYC: 40.7829, -73.9654)
- Use valid GLN format for identifier tests (13 digits)
- Use `Thing::new("name")` for simple test things
