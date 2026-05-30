# Main Event Service (MPI) - Project Index

## Overview

A high-performance Main Event Service system built with Rust for managing centralized event identity registries across healthcare providers.

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
cargo test           # Run all tests
cargo test --lib     # Unit tests only (99 tests)
cargo test --tests   # Integration tests only (7 tests)
cargo bench          # Run benchmarks (3 suites)
cargo clippy         # Run linter
cargo fmt            # Format code
```

### Project Structure

```
src/
├── lib.rs           # Library root
├── api/             # REST, FHIR R5, gRPC API layers
├── models/          # Domain models (Event, Identifier, Document, etc.)
├── matching/        # Matching algorithms (name, DOB, gender, address, phonetic, scoring)
├── search/          # Full-text search engine (Tantivy)
├── db/              # Database layer (SeaORM, PostgreSQL)
├── streaming/       # Event publishing
├── validation/      # Validation rules, normalization
├── privacy/         # Data masking, GDPR export
├── config/          # Configuration management
├── observability/   # OpenTelemetry setup
└── error.rs         # Error types

tests/               # Integration tests
benches/             # Criterion benchmarks (matching, search, validation)
AGENTS/              # Reference documentation
```

### Key Types

| Type                  | Location                    | Description                                            |
| --------------------- | --------------------------- | ------------------------------------------------------ |
| `Event`               | `models::event`             | Core event identity record                             |
| `HumanName`           | `models::event`             | Structured name (family, given, prefix, suffix)        |
| `Gender`              | `models::mod`               | Male, Female, Other, Unknown                           |
| `Identifier`          | `models::identifier`        | External identifiers (MRN, SSN, DL, NPI, PPN, TAX)     |
| `IdentityDocument`    | `models::document`          | Identity documents (passport, birth certificate, etc.) |
| `EmergencyContact`    | `models::emergency_contact` | Emergency contact information                          |
| `Address`             | `models::mod`               | Physical address                                       |
| `ContactPoint`        | `models::mod`               | Phone, email, fax contacts                             |
| `Consent`             | `models::consent`           | GDPR consent management                                |
| `MergeRequest`        | `models::merge`             | Event merge operations                                 |
| `MatchResult`         | `matching::mod`             | Match score + breakdown                                |
| `MatchScoreBreakdown` | `matching::mod`             | Per-component score details                            |

### Key Functions

| Function              | Location               | Description                                  |
| --------------------- | ---------------------- | -------------------------------------------- |
| `match_events`        | `matching::mod`        | Match two events with weighted scoring       |
| `find_matches`        | `matching::mod`        | Find matches for a event in a candidate list |
| `match_name`          | `matching::algorithms` | Jaro-Winkler + Levenshtein name comparison   |
| `match_dob`           | `matching::algorithms` | Date of birth matching with tolerance        |
| `match_address`       | `matching::algorithms` | Weighted address comparison                  |
| `match_tax_id`        | `matching::algorithms` | Tax ID exact match                           |
| `match_document`      | `matching::algorithms` | Document type + number match                 |
| `soundex`             | `matching::phonetic`   | Soundex phonetic code                        |
| `validate_event`      | `validation`           | Validate event fields                        |
| `normalize_phone`     | `validation`           | E.164-like phone normalization               |
| `standardize_address` | `validation`           | Address standardization                      |
| `mask_event`          | `privacy`              | Mask sensitive fields                        |
| `export_event_data`   | `privacy`              | GDPR data export                             |
