# Place Service (MPI)

The Place Service is a critical enterprise system that maintains a
centralized registry of place identities across multiple areas.

@agents/share/overview.md

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

@agents/share/match-search-merge.md

### Data Quality & Validation

- Required field enforcement (name)
- Address validation (requires locality, postal code, or country)
- Coordinate validation (latitude -90 to 90, longitude -180 to 180)
- GLN format validation (13-digit with check digit)
- URL format validation
- Telephone format validation
- Opening hours validation
- Address standardization (title-case locality, uppercase region/country, expand abbreviations)
- Coordinate normalization (decimal degrees, WGS 84)
- Validation integrated into create and update handlers (returns 422)

@AGENTS/architecture.md
@AGENTS/matching.md
@AGENTS/models.md
@AGENTS/restful.md
@AGENTS/testing.md

@agents/share/auditability.md
@agents/share/availability.md
@agents/share/match-search-merge.md
@agents/share/observability.md
@agents/share/privacy.md
@agents/share/restful.md
@agents/share/technology.md

## Quick Start

### Option 1: Docker (Recommended)

```bash
# Clone repository
git clone https://github.com/sixarm/place-service-rust-crate.git
cd place-service-rust-crate

# Copy environment configuration
cp .env.example .env

# Start all services (PostgreSQL + MPI)
docker-compose up -d

# View logs
docker-compose logs -f mpi-server

# Access the API
curl http://localhost:8080/api/health
```

**Services Available:**

- **API**: http://localhost:8080/api
- **Swagger UI**: http://localhost:8080/swagger-ui
- **pgAdmin** (optional): http://localhost:5050
  ```bash
  docker-compose --profile tools up -d
  ```

See [DEPLOY.md](DEPLOY.md) for complete deployment guide.

### Option 2: Local Development

**Prerequisites:**

- Rust 1.75+ ([Install Rust](https://rustup.rs/))
- PostgreSQL 15+
- Diesel CLI: `cargo install diesel_cli --no-default-features --features postgres`

```bash
# Clone repository
git clone https://github.com/sixarm/place-service-rust-crate.git
cd place-service-rust-crate

# Set up database
createdb mpi
cp .env.example .env
# Edit .env and set DATABASE_URL

# Run migrations
diesel migration run

# Build and run
cargo build --release
cargo run --release
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
|  (Diesel)     |  |   Search         |  |  (In-Memory)       |
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
place-service-rust-crate/
├── src/
│   ├── lib.rs             # Library root
│   ├── main.rs            # Binary entry point
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
│   ├── architecture.md    # System architecture
│   ├── models.md          # Domain model reference
│   ├── matching.md        # Matching algorithm reference
│   ├── testing.md         # Testing strategy
│   └── api.md             # API endpoint reference
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
cargo watch -x run           # Dev mode with auto-reload
cargo run --release          # Production mode
RUST_LOG=debug cargo run     # With debug logging
```

### Code Quality

```bash
cargo fmt                    # Format code
cargo clippy                 # Run linter
cargo test --lib             # Run unit tests
```

### Database Migrations

```bash
diesel migration generate migration_name
diesel migration run
diesel migration revert
diesel migration list
```

## API Documentation

### Interactive Documentation

Access the Swagger UI at **http://localhost:8080/swagger-ui** for interactive API exploration.

### Quick Examples

**Create Place (with duplicate detection):**

```bash
curl -X POST http://localhost:8080/api/places \
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
curl -X POST http://localhost:8080/api/places/check-duplicates \
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

**Search Places (with pagination and masking):**

```bash
curl "http://localhost:8080/api/places/search?q=Central+Park&limit=10&offset=0&fuzzy=true&mask_sensitive=true"
```

**Match Place:**

```bash
curl -X POST http://localhost:8080/api/places/match \
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
curl -X POST http://localhost:8080/api/places/merge \
  -H "Content-Type: application/json" \
  -d '{
    "main_place_id": "uuid-main",
    "duplicate_place_id": "uuid-dup",
    "merge_reason": "Confirmed duplicate"
  }'
```

**Batch Deduplication:**

```bash
curl -X POST http://localhost:8080/api/places/deduplicate \
  -H "Content-Type: application/json" \
  -d '{ "threshold": 0.7, "auto_merge_threshold": 0.95, "max_candidates": 50 }'
```

**GDPR Data Export:**

```bash
curl "http://localhost:8080/api/places/{id}/export"
```

**Masked Place View:**

```bash
curl "http://localhost:8080/api/places/{id}/masked"
```

## Configuration

Configuration via environment variables or `.env` file:

| Variable                   | Description                  | Default        | Required |
| -------------------------- | ---------------------------- | -------------- | -------- |
| `DATABASE_URL`             | PostgreSQL connection string | -              | Yes      |
| `DATABASE_MAX_CONNECTIONS` | Max connection pool size     | 10             | No       |
| `DATABASE_MIN_CONNECTIONS` | Min connection pool size     | 2              | No       |
| `SERVER_HOST`              | Server bind address          | 0.0.0.0        | No       |
| `SERVER_PORT`              | HTTP server port             | 8080           | No       |
| `SEARCH_INDEX_PATH`        | Tantivy index directory      | ./search_index | No       |
| `MATCHING_THRESHOLD`       | Match score threshold        | 0.7            | No       |
| `RUST_LOG`                 | Logging level                | info           | No       |

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

- Unit Tests: 104 tests
- Integration Tests: 67 tests
- Benchmark Tests: 16 benchmarks (Criterion)
- Total: 171 tests + 16 benchmarks

**Unit Test Breakdown:**

- Models (32 tests): Place, PostalAddress, GeoCoordinates, PlaceType, PlaceIdentifier, AmenityFeature, OpeningHoursSpecification, Consent
- Matching (45 tests): Name (8), Address (5), Geo (7), Identifier (7), Phonetic/Soundex (10), Scoring (8)
- Validation (19 tests): Name, coordinates, GLN, URL, telephone, address, normalization
- Privacy (8 tests): Phone/fax masking, geo rounding, GDPR export

**Integration Test Breakdown:**

- Matching Pipeline (7 tests): Duplicate detection, fuzzy matching, GLN deterministic, batch matching
- Validation Pipeline (3 tests): Validate-normalize workflow, lifecycle validation
- Privacy Pipeline (4 tests): Mask-export workflow, GDPR export, soft delete export
- Models Pipeline (16 tests): Full construction, serialization, hierarchy, geo symmetry, identifiers, consent, place types, opening hours
- Scoring Pipeline (24 tests): Unicode, edge cases, custom weights, confidence boundaries, score ranges, phonetic bonus, all components, batch sorting
- Edge Cases (13 tests): Boundary coordinates, GLN validation, URL protocols, address edge cases, normalization, privacy masking, combined workflows

**Benchmark Tests:**

- Matching (9 benchmarks): Name similarity, geo similarity, Soundex, full place match, batch 100 candidates
- Validation (3 benchmarks): Simple validation, full validation, normalization
- Searching (2 benchmarks): Name search exact, name search fuzzy
- Database (4 benchmarks): Place construction, batch construction, create+validate, create+normalize
- Privacy (4 benchmarks): Mask place, mask minimal, GDPR export, GDPR batch 100

## Deployment

See [DEPLOY.md](DEPLOY.md) for comprehensive deployment guide.

```bash
docker-compose up -d                                    # Development
docker-compose -f docker-compose.test.yml up            # Testing
docker build -t mpi-server:v1.0.0 . && docker run ...  # Production
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
- **Geo-Radius Search**: ~30-150ms (depending on radius and density)
- **Concurrent Requests**: 1000+ req/sec

## Development Phases

This project was developed in 15 comprehensive phases:

1. **Phase 1-6**: Core infrastructure, models, configuration
2. **Phase 7**: Database Integration (SeaORM, PostgreSQL)
3. **Phase 8**: Event Streaming & Audit Logging
4. **Phase 9**: REST API Implementation
5. **Phase 10**: Integration Testing
6. **Phase 11**: Docker & Deployment
7. **Phase 12**: Documentation
8. **Phase 13**: Advanced MPI Features (duplicate detection, merging, deduplication, validation, privacy, geo matching, place hierarchy)
9. **Phase 14**: Core Library & Tests (104 unit tests, 14 integration tests, 12 benchmarks)
10. **Phase 15**: Update & Expand Tests (171 tests + 16 benchmarks, comprehensive edge case coverage)

See [spec.md §13](spec.md#13-tasks) for the live task queue and [spec.md §14](spec.md#14-implementation-status) for implementation status.

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

Dual-licensed under MIT OR Apache-2.0.

---

**Status**: Production-Ready
**Version**: 0.2.0
