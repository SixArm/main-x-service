# Testing Strategy & Guide

## Test Categories

### Unit Tests (221 tests: 219 run by default + 2 DB-gated `#[ignore]`)

Located in `#[cfg(test)] mod tests` within each source file. Verified
2026-08-26 via `cargo test --lib -- --list` (was 207, verified
2026-08-04); the only counts that moved are `matching::geo` (T-9's
`bounding_box` + the exact-boundary `within_radius` test) and `search`
(T-9's `search_page` offset/total tests, plus a regression test for an
over-length-token indexing edge case `search_page` surfaced). The rest
of the models/matching/validation/privacy core is unchanged since
2026-08-04.

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
| `matching::geo`         | 12    | Same point, close, moderate, far, within radius (true/false, plus the exact-boundary `<=` inclusive case), custom reference; T-9's `bounding_box` (straddles center + grows with radius, contains every true-Haversine-circle point at exactly `radius_km`, zero radius collapses to a point, clamps near a pole) |
| `matching::identifier`  | 7     | Matching/different GLN, empty, mixed, has_gln_match (true, false type, false value)                                                                                                                             |
| `matching::phonetic`    | 10    | Robert, Rupert, match, no match, Ashcraft, empty, single char, case, Washington, place names                                                                                                                    |
| `matching::scoring`     | 8     | Identical places, name only, different, GLN deterministic, confidence levels, weights sum, fuzzy, phonetic bonus                                                                                                |
| `matching::adapter`     | 5     | Service → matcher field routing (telecom, address renames, identifier-scheme URIs, place-type mapping, sparse records)                                                                                          |
| `validation`            | 29    | Valid place, empty/whitespace name, invalid lat/lon, valid coords, invalid/valid GLN + check-digit helper, valid/invalid opening-hours times + `time_is_valid` helper, invalid/valid URL, invalid/valid telephone, address missing fields, address with locality, multiple errors, normalization, 4× SEC-M1 input-size-cap tests |
| `privacy`               | 8     | Mask telephone/fax/geo, preserve name, no sensitive fields, short phone, GDPR export, export fields                                                                                                             |
| `search` (+ `search::index`) | 9 | Index/exact/fuzzy search, delete-removes, empty-index, create-or-open round trip; T-9's `search_page` offset+total (skips + reports the true total, offset past total is empty); an over-length (>40 char) single token is silently dropped by Tantivy's default tokenizer, not found even by exact query on the same string (a sharp edge for a "unique token" test fixture built by concatenating a prefix directly against a 32-hex-char UUID with no separator) |
| `streaming` (+ `streaming::envelope`) | 11 | `EventTransport::parse`; `Envelope` construction (`for_place`/`for_merge`), `EventView` projection, `OutboxInsert` field mapping (T-12) |
| `metrics`               | 1     | Registry render includes default counters                                                                                                                                                                       |
| `api::rest` (+ `state`, `version`, `handlers::review_report_tests`) | 11 | OpenAPI path/schema assertions, `AppState` construction, `Accepts-version` negotiation, review-queue report shaping |
| `api::rest::auth`       | 23    | Bearer verification (valid/missing/non-bearer/expired/tampered/no-key), blanket-enforcement matrix, ABAC action derivation + policy matrix, boot-time key fetch (URL wins / fetch-failure fallback) |
| `fhir` (+ `fhir::search`) | 10  | Identifier scheme round-trip, DTO↔`Location` round-trip, missing-name reject, soft-delete⇒`inactive`, search-predicate matching, `CapabilityStatement` (T-11) |
| `controllers::fhir`     | 1     | `CapabilityStatement` lists `Location` + its search params                                                                                                                                                      |
| `compliance::mac` (+ `record_integrity`, `audit_integrity`) | 11 | Key-set loading, record/audit digest + MAC compute, `mac_absent` when unkeyed |
| `config`                | 5     | Env-var parsing / defaults                                                                                                                                                                                      |
| `db::outbox` (+ `db::tests`) | 6 (2 `#[ignore]`, need `DATABASE_URL`) | `OutboxInsert::from_envelope` field mapping + non-UUID-pid reject; DB-gated: `create` writes one `created` row, `merge` writes `merged`+`deleted` atomically (T-12) |
| `relay`                 | 3     | Logging-sink smoke test, capturing-sink contract, config defaults (T-12b)                                                                                                                                       |

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
| `integration_geo_radius.rs` | 4     | Geo-radius candidate filtering over a collection (the `matching::geo::within_radius` primitive the wired `nearby` endpoint uses — DB-free, independent of `api_nearby_and_search_offset.rs` below), radius monotonicity, matcher-bridge worked example (near-duplicate match + unrelated reject) |
| `integration_edge_cases.rs` | 16    | Boundary coordinates, GLN length validation, URL protocols, address minimal/empty fields, multi-word normalization, idempotent normalization, all sensitive fields masking, empty phone masking, GDPR field preservation, combined workflows, GLN deterministic override  |

**DB/broker-gated additions (2026-08).** Four more files exist in
`tests/` beyond the 86 counted above, all `#[ignore]`d so a plain
`cargo test --tests` is unaffected — they need real infrastructure:

| File | Tests | Gate | What's covered |
|---|---|---|---|
| `api_integration_test.rs` | 3 | `DATABASE_URL` | QA-SERVER-FIELDS regression: a minimal hand-written create body round-trips a fresh id + "now" timestamps; two hand-written creates don't collide; an omitted `name` fails via `validation_error`, not the JSON extractor |
| `api_nearby_and_search_offset.rs` | 5 | `DATABASE_URL` | T-9 end-to-end over the real router: `GET /api/places/nearby` filters to places within `radius_km` (and correctly includes/excludes a place a hair inside/outside the radius); out-of-range `lat`/`lon`/`radius_km` is `400`; `GET /api/places/search?offset=` skips the requested rows while `X-Total-Count` stays the true total; an `offset` past the bound is `400` on both endpoints. Each test overrides `SEARCH_INDEX_PATH` to a private `TempDir` (see the file's `test_router` doc comment) rather than sharing the crate's default `./data/search_index` — a directory every local test run and every concurrent agent session on the same machine writes to, and racing writers there silently drop an index write. |
| `enforcement.rs` | 1 | `DATABASE_URL`, run with `-- --ignored` | `PLACE_REQUIRE_AUTH=1` activation proof over the real production router (public paths open, protected read/write need a token, valid-but-empty-`attrs` reads but not writes) |
| `fluvio_relay.rs` | 1 | `--features fluvio` **and** a live Fluvio broker | `FluvioSink` round-trip (create → outbox → relay → real topic); verified today only by compiling under the feature, not by an actual run |

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
