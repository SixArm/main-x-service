# Testing Strategy & Guide

## Test Categories

### Unit Tests (125 tests)

Located in `#[cfg(test)] mod tests` within each source file.
Counts below are the largest modules; `search` (6), `streaming` (2),
`metrics` (1), and `api::rest` (1) round the total to 125.

| Module                  | Tests | What's Covered                                                                                                                                                                                                  |
| ----------------------- | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `models::place`         | 6     | Construction, defaults, address/geo, serialization, soft delete                                                                                                                                                 |
| `models::address`       | 4     | Construction, fields, serialization, partial                                                                                                                                                                    |
| `models::geo`           | 7     | Construction, elevation, Haversine (same point, known distance, short, antipodal), serialization                                                                                                                |
| `models::place_type`    | 4     | Display, equality, serialization, Other variant                                                                                                                                                                 |
| `models::identifier`    | 3     | GLN, custom, serialization                                                                                                                                                                                      |
| `models::amenity`       | 2     | Construction, with value                                                                                                                                                                                        |
| `models::opening_hours` | 2     | Construction, serialization                                                                                                                                                                                     |
| `models::consent`       | 4     | Active, revoked, expired by date, not yet expired                                                                                                                                                               |
| `matching::name`        | 8     | Exact, case-insensitive, similar, different, empty, both empty, substring, prefix bonus                                                                                                                         |
| `matching::address`     | 5     | Identical, different, partial, no overlap, case-insensitive                                                                                                                                                     |
| `matching::geo`         | 7     | Same point, close, moderate, far, within radius (true/false), custom reference                                                                                                                                  |
| `matching::identifier`  | 7     | Matching/different GLN, empty, mixed, has_gln_match (true, false type, false value)                                                                                                                             |
| `matching::phonetic`    | 10    | Robert, Rupert, match, no match, Ashcraft, empty, single char, case, Washington, place names                                                                                                                    |
| `matching::scoring`     | 8     | Identical places, name only, different, GLN deterministic, confidence levels, weights sum, fuzzy, phonetic bonus                                                                                                |
| `matching::adapter`     | 5     | Service → matcher field routing (telecom, address renames, identifier-scheme URIs, place-type mapping, sparse records)                                                                                          |
| `validation`            | 25    | Valid place, empty/whitespace name, invalid lat/lon, valid coords, invalid/valid GLN + check-digit helper, valid/invalid opening-hours times + `time_is_valid` helper, invalid/valid URL, invalid/valid telephone, address missing fields, address with locality, multiple errors, normalization |
| `privacy`               | 8     | Mask telephone/fax/geo, preserve name, no sensitive fields, short phone, GDPR export, export fields                                                                                                             |

### Integration Tests (72 tests in `integration_*.rs`; 86 total in `tests/`)

Located in `tests/` directory. Test end-to-end workflows and edge
cases. The `integration_*.rs` files below total 72; the
`duplicate_detection.rs` bridge suite (14, documented separately
below) brings the `tests/` directory total to 86.

| File                        | Tests | What's Covered                                                                                                                                                                                                                                                            |
| --------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `integration_matching.rs`   | 7     | Exact duplicate, typo match, completely different, same name different city, GLN override, name only, batch candidates                                                                                                                                                    |
| `integration_validation.rs` | 4     | Validate-normalize workflow, invalid place handling, full lifecycle (incl. opening hours), invalid opening-hours times                                                                                                                                                     |
| `integration_privacy.rs`    | 4     | Mask-export workflow, full GDPR export, immutability, soft delete export                                                                                                                                                                                                  |
| `integration_models.rs`     | 13    | Full construction serialization, soft delete timestamps, unique IDs, place hierarchy, geo distance symmetry/triangle inequality, multiple identifier types, consent lifecycle/serialization, all place types, full week opening hours, address default/equality           |
| `integration_scoring.rs`    | 24    | Unicode names, long names, single char, reversed words, address edge cases, geo poles/date line/radius boundary, identifier edge cases, Soundex consistency, custom weights, confidence boundaries, score range validation, phonetic bonus, all components, batch sorting |
| `integration_geo_radius.rs` | 4     | Geo-radius candidate filtering over a collection (the future `nearby` endpoint primitive), radius monotonicity, matcher-bridge worked example (near-duplicate match + unrelated reject) |
| `integration_edge_cases.rs` | 16    | Boundary coordinates, GLN length validation, URL protocols, address minimal/empty fields, multi-word normalization, idempotent normalization, all sensitive fields masking, empty phone masking, GDPR field preservation, combined workflows, GLN deterministic override  |

### Benchmark Tests (16 benchmarks)

Located in `benches/` directory. Uses Criterion for statistical benchmarking.

| File                        | Benchmarks | What's Measured                                                                                                              |
| --------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `matching_bench.rs`         | 9          | name_similarity (exact/fuzzy/different), geo_similarity (close/far), soundex (short/long), full_place_match, batch_match_100 |
| `validation_bench.rs`       | 3          | validate_simple, validate_full, normalize_place                                                                              |
| `searching_bench.rs`        | 2          | search_by_name_100, search_by_name_fuzzy_100                                                                                 |
| `database_reading_bench.rs` | 2          | place_construction, place_batch_construction_100                                                                             |
| `database_writing_bench.rs` | 2          | place_create_and_validate, place_create_and_normalize                                                                        |
| `privacy_bench.rs`          | 4          | mask_place, mask_place_minimal, gdpr_export, gdpr_export_batch_100                                                           |

## Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Specific module
cargo test --lib models::place
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

## Writing New Tests

### Unit Test Pattern

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

### Integration Test Pattern

```rust
// tests/integration_feature.rs
use place_service::models::place::Place;

#[test]
fn test_end_to_end_workflow() {
    // Setup
    let place = Place::new("Test");

    // Execute pipeline
    let validated = validate_place(&place);
    let matched = compute_match(&place, &other, &weights);

    // Verify
    assert!(validated.is_empty());
    assert!(matched.score > 0.8);
}
```

## Test Data Conventions

- Use well-known places for readability (Central Park, Eiffel Tower, etc.)
- Use realistic coordinates (NYC: 40.7829, -73.9654)
- Use valid GLN format for identifier tests (13 digits)
- Use `Place::new("name")` for simple test places

## Bridge Integration Tests

`tests/duplicate_detection.rs` is a black-box test that drives the
service-side domain model through [`matching::adapter::to_matcher_place`]
and asserts on `MatchingEngine::match_places` output. The suite pins
**both sides of the contract** — the adapter's field-routing rules and
the matcher's scoring algorithm — so a regression on either side fails
a test here.

Run with: `cargo test --test duplicate_detection`

### Coverage (14 tests)

| Category | What it pins |
|---|---|
| Identical / near-duplicate | identical-clone score ≥ 0.95, name-typo fuzzy match, ordering invariants (closer-evidence outscores farther) |
| Deterministic short-circuits | GLN deterministic short-circuit, OSM identifier → `OsmNode` scheme, geo-distance ranking (closer > farther), PlaceType match/mismatch, antipodal-coordinates negative case |
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
- The place-matcher crate exposes a new scoring component the service
  needs to surface.
- A regression escapes the adapter's own `#[cfg(test)] mod tests`.
