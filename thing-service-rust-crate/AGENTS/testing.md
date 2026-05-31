# Testing strategy — Thing Service

## Test categories

### Unit tests

Embedded in source files via `#[cfg(test)] mod tests`. Run with `cargo test --lib`.

| Module                  | What's covered                                                                                  |
|-------------------------|-------------------------------------------------------------------------------------------------|
| `models::thing`         | Construction, defaults, identifiers, URLs, serialization, soft delete                           |
| `models::identifier`    | Constructors, custom variant, `is_deterministic`, serialization, PropertyValue fields           |
| `models::consent`       | Active, revoked, expired by date, not yet expired                                               |
| `matching::name`        | Exact, case-insensitive, similar, different, empty, both empty, substring, prefix bonus         |
| `matching::description` | Exact, case-insensitive, similar, different, both empty, one empty                              |
| `matching::url`         | Identical, scheme-insensitive, trailing slash, case-insensitive, same host, different host, list|
| `matching::identifier`  | Matching/different ISBN, mixed identifiers, deterministic (ISBN/DOI), non-deterministic (SKU)   |
| `matching::phonetic`    | Robert, Rupert, Ashcraft, empty, single char, case, Washington, typo pairs                      |
| `matching::scoring`     | Identical things, name only, different, ISBN/DOI deterministic, SKU not deterministic, weights, fuzzy, phonetic bonus |
| `validation`            | Valid, empty name, URL formats, additional_type, images, same_as, ISBN/DOI/GTIN/UUID, custom skip, alternate names, normalization, scheme-lowercase, dedupe |
| `privacy`               | Mask owner, identifier values, identifier URL cleared, short identifier, preserves property_id, GDPR export top-level fields |

### Integration tests

In `tests/`. Run with `cargo test --tests`.

| File                           | What's covered                                                                                                |
|--------------------------------|---------------------------------------------------------------------------------------------------------------|
| `integration_matching.rs`      | Exact duplicate, typo match, completely different, ISBN/DOI deterministic, batch ranking, same_as contribution|
| `integration_validation.rs`    | Validate-normalize workflow, invalid thing handling, full lifecycle                                           |
| `integration_privacy.rs`       | Mask-export workflow, full GDPR export, immutability, soft delete export                                      |
| `integration_models.rs`        | Construction serialization, soft delete timestamps, unique IDs, identifier round-trip, PropertyValue, consent |
| `integration_scoring.rs`       | Unicode names, edge cases, description, URL, identifier edge cases, custom weights, confidence boundaries     |
| `integration_edge_cases.rs`    | URL protocols, ISBN/GTIN/DOI/UUID lengths, custom identifier skip, scheme lowercasing, dedupe, mask, full workflows |

### Benchmark tests

In `benches/`. Run with `cargo bench` (Criterion).

| File                        | What's measured                                                                                                              |
|-----------------------------|------------------------------------------------------------------------------------------------------------------------------|
| `matching_bench.rs`         | `name_similarity` (exact/fuzzy/different), `url_similarity` (identical/different), Soundex (short/long), full match, batch_match_100 |
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
use thing_service::models::thing::Thing;

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

- Use well-known canonical things for readability — books (Pride and
  Prejudice, War and Peace), software (Linux kernel, The Rust
  Programming Language), papers — never place-flavoured data.
- Use real ISBNs/DOIs in tests where format validation matters.
- Use `Thing::new("name")` for simple test things.
