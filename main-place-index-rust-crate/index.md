# Main Place Index (MPI) - Project Index

## Overview

A high-performance Main Place Index system built with Rust for managing centralized place identity registries. Based on [schema.org/Place](https://schema.org/Place).

## Documentation

| Document               | Description                                             |
| ---------------------- | ------------------------------------------------------- |
| [CLAUDE.md](CLAUDE.md) | Project overview, features, architecture, configuration |
| [plan.md](plan.md)     | Implementation plan, technology stack, domain model     |
| [tasks.md](tasks.md)   | Task summary and phase details                          |
| [AGENTS/](AGENTS/)     | Detailed reference documentation                        |

## Quick Reference

### Build & Test

```bash
cargo check          # Check compilation
cargo test           # Run all tests (171 tests)
cargo test --lib     # Unit tests only (104 tests)
cargo test --tests   # Integration tests only (67 tests)
cargo bench          # Run benchmarks (16 benchmarks)
cargo clippy         # Run linter
cargo fmt            # Format code
```

### Project Structure

```
src/
├── lib.rs           # Library root
├── models/          # Domain models (Place, Address, Geo, etc.)
├── matching/        # Matching algorithms (name, address, geo, phonetic, scoring)
├── validation/      # Validation rules, normalization
└── privacy/         # Data masking, GDPR export

tests/               # Integration tests (5 test files, 67 tests)
benches/             # Criterion benchmarks (6 bench files, 16 benchmarks)
AGENTS/              # Reference documentation
```

### Key Types

| Type                        | Location                | Description                                 |
| --------------------------- | ----------------------- | ------------------------------------------- |
| `Place`                     | `models::place`         | Core place entity (schema.org/Place)        |
| `PostalAddress`             | `models::address`       | Structured address                          |
| `GeoCoordinates`            | `models::geo`           | Lat/lon with Haversine distance             |
| `PlaceType`                 | `models::place_type`    | Place classification enum                   |
| `PlaceIdentifier`           | `models::identifier`    | External identifiers (GLN, FIPS, GNIS, OSM) |
| `AmenityFeature`            | `models::amenity`       | Place amenity features                      |
| `OpeningHoursSpecification` | `models::opening_hours` | Operating hours                             |
| `Consent`                   | `models::consent`       | GDPR consent management                     |
| `MatchResult`               | `matching::scoring`     | Match score + confidence + breakdown        |
| `MatchWeights`              | `matching::scoring`     | Configurable scoring weights                |
| `MatchConfidence`           | `matching::scoring`     | Certain/Probable/Possible/Unlikely          |
| `ValidationError`           | `validation`            | Validation error with field + message       |

### Key Functions

| Function                | Location               | Description                            |
| ----------------------- | ---------------------- | -------------------------------------- |
| `compute_match`         | `matching::scoring`    | Match two places with weighted scoring |
| `name_similarity`       | `matching::name`       | Jaro-Winkler name comparison           |
| `address_similarity`    | `matching::address`    | Weighted address comparison            |
| `geo_similarity`        | `matching::geo`        | Haversine-based geo comparison         |
| `within_radius`         | `matching::geo`        | Check if two points within distance    |
| `identifier_similarity` | `matching::identifier` | Identifier matching                    |
| `has_gln_match`         | `matching::identifier` | GLN deterministic match check          |
| `soundex`               | `matching::phonetic`   | Soundex phonetic code                  |
| `soundex_match`         | `matching::phonetic`   | Soundex code comparison                |
| `validate_place`        | `validation`           | Validate place fields                  |
| `normalize_place`       | `validation`           | Normalize address formatting           |
| `mask_place`            | `privacy`              | Mask sensitive fields                  |
| `gdpr_export`           | `privacy`              | Export place as JSON                   |
