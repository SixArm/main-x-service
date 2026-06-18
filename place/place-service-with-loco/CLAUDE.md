# Place Service

The Place Service is a critical enterprise system that maintains a
centralized registry of place identities across multiple areas.

@../../agents/share/overview.md

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Docker Deployment](#docker-deployment)
- [Technology Stack](#technology-stack)
- [Architecture](#architecture)
- [Development](#development)
- [API Documentation](#api-documentation)
- [Configuration](#configuration)
- [Testing](#testing)
- [Deployment](#deployment)
- [Security & Compliance](#security--compliance)
- [Performance](#performance)
- [Contributing](#contributing)

## Features

### Place Identity Management

Based on [schema.org/Place](https://schema.org/Place):

- Place identifier management (Global Location Number, branch code, FIPS, GNIS, OSM ID)
- Multiple names and alternate names per place
- Structured address management (PostalAddress: street, locality, region, country, postal code)
- Geo coordinate management (latitude, longitude, elevation)
- Place type classification (LocalBusiness, CivicStructure, AdministrativeArea, Landform, etc.)
- Place hierarchy (containedInPlace / containsPlace relationships)
- Contact information management (telephone, fax, URL)
- Opening hours specification
- Amenity features and accessibility information
- Automatic event publishing for all CRUD operations

### Place matcher

- **Match Components**:
  - Name matching (Jaro-Winkler, Levenshtein, Soundex phonetic)
  - Address matching (street, postal code, locality, region, country)
  - Geo coordinate matching (Haversine distance calculation)
  - Place type matching
  - Identifier matching (GLN, branch code, FIPS, GNIS, OSM ID)
  - GLN exact match (deterministic, short-circuits to 1.0)
- **Score Breakdown**: Full per-component score breakdown in API responses

@../../agents/share/match-search-merge.md

### Data Quality & Validation

- Required field enforcement (name)
- Address validation (requires locality, postal code, or country)
- Coordinate validation (latitude -90 to 90, longitude -180 to 180)
- GLN format validation (13-digit with check digit)
- URL format validation
- Telephone format validation
- Opening hours validation (24-hour HH:MM times)
- Address standardization (title-case locality, uppercase region/country, expand abbreviations)
- Coordinate normalization (decimal degrees, WGS 84)
- Validation integrated into create and update handlers (returns 422)

@../../agents/share/architecture.md
@AGENTS/matching.md
@AGENTS/models.md
@AGENTS/restful.md
@AGENTS/testing.md

@../../agents/share/auditability.md
@../../agents/share/availability.md
@../../agents/share/match-search-merge.md
@../../agents/share/observability.md
@../../agents/share/privacy.md
@../../agents/share/restful.md
@../../agents/share/loco.md

## Quick Start

**Prerequisites:**

- Rust 1.95+ (2024 edition) ([Install Rust](https://rustup.rs/))
- PostgreSQL 18+
- No extra CLI tooling: migrations are a SeaORM migration crate (`migration/`) run through the built-in loco CLI

```bash
cd place-service-with-loco

# Point DATABASE_URL at the database, then start. Migrations run
# automatically in development (auto_migrate).
export DATABASE_URL=postgres://localhost/place_service_development
cargo loco start            # or: cargo run -- start

# Service binds on the configured port (config/development.yaml):
curl http://localhost:5150/api/health
```

**Services Available (default development port `5150`):**

- **API**: http://localhost:5150/api
- **Swagger UI**: http://localhost:5150/swagger-ui
- **Prometheus metrics**: http://localhost:5150/metrics.prom (root path, not under `/api`)

Container deployment uses Podman (NOT Docker):

```bash
podman compose up -d
```

## Architecture

### System Architecture

```
+------------------------------------------------------------------+
|                         Client Layer                              |
|  (Web Apps, Mobile Apps, GIS Systems, Analytics Platforms)        |
+------------------------------+-----------------------------------+
                               |
+------------------------------v-----------------------------------+
|                      REST API Layer (Axum)                        |
|  - OpenAPI/Swagger Documentation                                 |
|  - Validation & Data Quality                                     |
|  - Privacy & Data Masking                                        |
|  - CORS, Error Handling                                          |
+------------------------------+-----------------------------------+
                               |
+------------------------------v-----------------------------------+
|                   Business Logic Layer                            |
|  +---------------+ +----------------+ +-----------------------+  |
|  |   Place       | |   Matching     | |   Search Engine       |  |
|  |  Repository   | |  Algorithms    | |    (Tantivy)          |  |
|  +---------------+ +----------------+ +-----------------------+  |
|  +---------------+ +----------------+ +-----------------------+  |
|  |    Event      | |    Audit       | |   Deduplication       |  |
|  |  Publisher    | |  Log Tracking  | |   Engine              |  |
|  +---------------+ +----------------+ +-----------------------+  |
|  +---------------+ +----------------+                            |
|  |  Validation   | |   Privacy      |                            |
|  |  & Quality    | |   & Masking    |                            |
|  +---------------+ +----------------+                            |
+------------------------------+-----------------------------------+
                               |
         +---------------------+---------------------+
         |                     |                     |
+--------v------+  +-----------v------+  +-----------v--------+
|  PostgreSQL   |  |   Tantivy        |  |  Event Stream      |
|  (SeaORM)     |  |   Search         |  |  (In-Memory)       |
|               |  |   Index          |  |                    |
|  - places     |  |                  |  |  - PlaceEvents     |
|  - audit_log  |  |                  |  |  - Subscribers     |
+---------------+  +------------------+  +--------------------+
```

### Data Flow

**Place Creation Flow:**

1. HTTP POST -> REST API Handler
2. Validation (required fields, format checks)
3. Duplicate Detection (search + match against existing)
4. If duplicates found: return 409 with matches
5. Repository `create()` -> Database INSERT
6. Search Engine `index_place()` -> Tantivy Index
7. Event Publisher -> PlaceCreated Event
8. Audit Logger -> audit_log INSERT
9. HTTP Response -> Client

**Place Merge Flow:**

1. HTTP POST /merge -> REST API Handler
2. Fetch main and duplicate from database
3. Transfer data from duplicate to main
4. Update main in database
5. Soft-delete duplicate
6. Update search index
7. Publish Merged event
8. Return merge record with transferred data

**Place Search Flow:**

1. HTTP GET -> REST API Handler
2. Search Engine `search()` -> Tantivy Query
3. Place IDs -> Repository `get_by_id()` batch
4. Optional: mask sensitive data
5. Place Records -> JSON Serialization
6. HTTP Response -> Client (with pagination)

## Project Structure

```
place-service-with-loco/
├── src/
│   ├── lib.rs             # Library root
│   ├── app.rs             # Loco Hooks impl (App)
│   ├── bin/main.rs        # Binary entry point (loco CLI)
│   ├── api/rest/          # Axum router, handlers, state, OpenAPI
│   ├── db/                # SeaORM repositories + audit
│   ├── search/            # Tantivy search engine
│   ├── streaming/         # InMemoryEventPublisher
│   ├── metrics.rs         # Prometheus registry + /metrics.prom
│   ├── models/
│   │   ├── mod.rs         # Module re-exports
│   │   ├── place.rs       # Place model (schema.org/Place based)
│   │   ├── address.rs     # PostalAddress model
│   │   ├── geo.rs         # GeoCoordinates with Haversine distance
│   │   ├── place_type.rs  # PlaceType enum
│   │   ├── identifier.rs  # PlaceIdentifier, IdentifierType (GLN, FIPS, GNIS, OSM)
│   │   ├── amenity.rs     # AmenityFeature model
│   │   ├── opening_hours.rs # OpeningHoursSpecification, DayOfWeek
│   │   └── consent.rs     # Consent management (GDPR)
│   ├── matching/
│   │   ├── mod.rs         # Module re-exports
│   │   ├── name.rs        # Name matching (Jaro-Winkler)
│   │   ├── address.rs     # Address matching (weighted fields)
│   │   ├── geo.rs         # Geo coordinate matching (Haversine distance)
│   │   ├── identifier.rs  # Identifier matching (GLN deterministic)
│   │   ├── phonetic.rs    # Soundex phonetic matching
│   │   └── scoring.rs     # Weighted scoring, confidence levels
│   ├── validation/
│   │   └── mod.rs         # Validation rules, address normalization
│   └── privacy/
│       └── mod.rs         # Data masking, GDPR export
├── tests/
│   ├── integration_matching.rs   # Matching pipeline tests
│   ├── integration_validation.rs # Validation pipeline tests
│   ├── integration_privacy.rs    # Privacy pipeline tests
│   ├── integration_models.rs     # Model pipeline tests
│   ├── integration_scoring.rs    # Scoring edge case tests
│   └── integration_edge_cases.rs # Edge case and workflow tests
├── benches/
│   ├── matching_bench.rs         # Matching algorithm benchmarks
│   ├── validation_bench.rs       # Validation benchmarks
│   ├── searching_bench.rs        # Search benchmarks
│   ├── database_reading_bench.rs # Database read benchmarks
│   ├── database_writing_bench.rs # Database write benchmarks
│   └── privacy_bench.rs          # Privacy benchmarks
├── AGENTS/                # Detailed reference documentation
│   ├── index.md           # Directory index
│   ├── spec-driven-development.md # SDD discipline
│   ├── models.md          # Domain model reference
│   ├── matching.md        # Matching algorithm reference
│   ├── restful.md         # REST API + library API reference
│   └── testing.md         # Testing strategy
├── spec/                  # Single source of truth (§1–§18)
├── Cargo.toml             # Project manifest
└── AGENTS.md              # Project overview
```

## Development

### Building the Project

```bash
cargo build          # Development build
cargo build --release # Release build
cargo check          # Check compilation
```

### Running the Server

```bash
cargo loco start             # Start the server (dev)
cargo run -- start           # Equivalent via the binary
RUST_LOG=debug cargo loco start  # With debug logging
```

### Code Quality

```bash
cargo fmt                    # Format code
cargo clippy                 # Run linter
cargo test --lib             # Run unit tests
```

### Database Migrations

Migrations live in the `migration/` SeaORM migration crate and run
through the loco CLI (`src/bin/main.rs` wires `cli::main::<App, Migrator>`).
In development they also run automatically on startup (`auto_migrate`):

```bash
cargo loco db migrate      # apply pending migrations
cargo loco db status       # show migration status
cargo loco db reset        # drop and re-run all migrations
```

Add a migration by creating a new `m*.rs` file in `migration/src/` and
registering it in `migration/src/lib.rs`.

## API Documentation

### Interactive Documentation

Access the Swagger UI at **http://localhost:5150/swagger-ui** for interactive API exploration.

### Quick Examples

**Create Place (with duplicate detection):**

```bash
curl -X POST http://localhost:5150/api/places \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Central Park",
    "alternate_name": "The Central Park",
    "description": "Urban park in Manhattan, New York City",
    "place_type": "Park",
    "address": {
      "street_address": "14 E 60th St",
      "address_locality": "New York",
      "address_region": "NY",
      "address_country": "US",
      "postal_code": "10022"
    },
    "geo": {
      "latitude": 40.7829,
      "longitude": -73.9654
    },
    "telephone": "+1-212-310-6600",
    "url": "https://www.centralparknyc.org",
    "is_accessible_for_free": true,
    "public_access": true
  }'
```

**Check for Duplicates:**

```bash
curl -X POST http://localhost:5150/api/places/check-duplicates \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Central Park",
    "address": {
      "address_locality": "New York",
      "address_region": "NY"
    },
    "geo": { "latitude": 40.7829, "longitude": -73.9654 }
  }'
```

**Search Places (full-text, fuzzy, masking):**

```bash
curl "http://localhost:5150/api/places/search?q=Central+Park&limit=10&fuzzy=true&mask_sensitive=true"
```

**Match Place:**

```bash
curl -X POST http://localhost:5150/api/places/match \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Centrl Park",
    "address": { "address_locality": "New York" },
    "geo": { "latitude": 40.783, "longitude": -73.965 },
    "threshold": 0.7
  }'
```

**Merge Places:**

```bash
curl -X POST http://localhost:5150/api/places/merge \
  -H "Content-Type: application/json" \
  -d '{
    "main_place_id": "uuid-main",
    "duplicate_place_id": "uuid-dup",
    "merge_reason": "Confirmed duplicate"
  }'
```

**Batch Deduplication:**

```bash
curl -X POST http://localhost:5150/api/places/deduplicate \
  -H "Content-Type: application/json" \
  -d '{ "threshold": 0.7, "auto_merge_threshold": 0.95, "max_candidates": 50 }'
```

**GDPR Data Export:**

```bash
curl "http://localhost:5150/api/places/{id}/export"
```

**Masked Place View:**

```bash
curl "http://localhost:5150/api/places/{id}/masked"
```

## Configuration

Configuration is loaded from `config/{development,test,production}.yaml`
(Loco convention). Environment-overridable variables:

| Variable             | Description                | Default                 |
| -------------------- | -------------------------- | ----------------------- |
| `PORT`               | REST bind port             | `5150` (dev config)     |
| `DATABASE_URL`       | Postgres connection string | per config file         |
| `SEARCH_INDEX_PATH`  | Tantivy index directory    | `./data/search_index`   |
| `MATCHING_THRESHOLD` | Probabilistic match cutoff | `0.85`                  |
| `RUST_LOG`           | tracing-subscriber filter  | `info`                  |
| `OTLP_ENDPOINT`      | OpenTelemetry collector    | `http://localhost:4317` |

## Testing

### Unit Tests

```bash
cargo test --lib                              # All unit tests
cargo test --lib test_place_matcher          # Specific test
cargo test --lib -- --nocapture               # With output
```

### Integration Tests

```bash
cargo test --tests                            # All integration tests
cargo test --test integration_matching        # Matching pipeline tests
cargo test --test integration_validation      # Validation pipeline tests
cargo test --test integration_privacy         # Privacy pipeline tests
cargo test --test integration_models          # Model pipeline tests
cargo test --test integration_scoring         # Scoring edge case tests
cargo test --test integration_edge_cases      # Edge case tests
```

### Benchmark Tests

```bash
cargo bench                                   # Run all benchmarks
cargo bench -- name_similarity                # Specific benchmark
```

### Test Coverage

**Current Coverage:**

- Unit Tests: 125 tests (`cargo test --lib`)
- Integration Tests: 86 tests in `tests/` (72 in `integration_*.rs` + 14 bridge)
- Benchmark Tests: 16 benchmarks (Criterion)

**Unit Test Breakdown:**

- Models (32 tests): Place, PostalAddress, GeoCoordinates, PlaceType, PlaceIdentifier, AmenityFeature, OpeningHoursSpecification, Consent
- Matching (50 tests): Name (8), Address (5), Geo (7), Identifier (7), Phonetic/Soundex (10), Scoring (8), Adapter (5)
- Validation (25 tests): Name, coordinates, GLN, opening-hours times, URL, telephone, address, normalization
- Privacy (8 tests): Phone/fax masking, geo rounding, GDPR export
- Search (6), Streaming (2), Metrics (1), API (1)

**Integration Test Breakdown:**

- Matching Pipeline (7 tests): Duplicate detection, fuzzy matching, GLN deterministic, batch matching
- Validation Pipeline (4 tests): Validate-normalize workflow, lifecycle validation, opening-hours time validation
- Privacy Pipeline (4 tests): Mask-export workflow, GDPR export, soft delete export
- Models Pipeline (13 tests): Full construction, serialization, hierarchy, geo symmetry, identifiers, consent, place types, opening hours
- Scoring Pipeline (24 tests): Unicode, edge cases, custom weights, confidence boundaries, score ranges, phonetic bonus, all components, batch sorting
- Edge Cases (16 tests): Boundary coordinates, GLN validation, URL protocols, address edge cases, normalization, privacy masking, combined workflows
- Bridge (`duplicate_detection.rs`, 14 tests): service ↔ place-matcher contract pinning

**Benchmark Tests:**

- Matching (9 benchmarks): Name similarity, geo similarity, Soundex, full place match, batch 100 candidates
- Validation (3 benchmarks): Simple validation, full validation, normalization
- Searching (2 benchmarks): Name search exact, name search fuzzy
- Database (4 benchmarks): Place construction, batch construction, create+validate, create+normalize
- Privacy (4 benchmarks): Mask place, mask minimal, GDPR export, GDPR batch 100

## Deployment

```bash
podman compose up -d                                    # Development
podman build -t place-server:v0.5.0 . && podman run ...  # Production
```

## Security & Compliance

### Implemented

- Audit Logging: Complete audit trail for compliance
- Soft Delete: Place records never truly deleted
- Non-Root Containers: Docker containers run as non-root user
- Environment-Based Secrets: No secrets in code or images
- CORS Configuration: Configurable cross-origin policies
- Data Masking: Sensitive fields (coordinates, telephone) masked on demand
- GDPR Data Export: Full place data export endpoint
- Consent Management: Consent model with type/status tracking
- Input Validation: Comprehensive validation on create/update

### Compliance Standards

- **GDPR**: Right of access (export), right to deletion (soft delete), consent management
- **Data Protection**: Audit logging, access controls, data encryption

## Performance

### Benchmarks

- **Place Create**: ~50ms (includes DB + search index + duplicate check)
- **Place Read**: ~5ms
- **Place Search**: ~20-100ms (depending on result size)
- **Place Match**: ~100-500ms (depending on candidate count)
- **Concurrent Requests**: 1000+ req/sec

(A geo-radius `nearby` HTTP endpoint is not yet delivered — see
[spec.md §13](spec/13-tasks.md) T-9.)

## Status

See [spec.md §13](spec/13-tasks.md) for the live task queue and
[spec.md §14](spec/14-implementation-status.md) for implementation
status (the canonical record of delivered capability and open gaps).

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Guidelines

- Follow Rust style guide (`cargo fmt`)
- Pass all tests (`cargo test --lib`)
- Pass clippy lints (`cargo clippy`)
- Add tests for new features
- Update documentation

## License

Licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only (see `Cargo.toml`).

---

**Status**: Production-Ready
**Version**: 0.5.0
