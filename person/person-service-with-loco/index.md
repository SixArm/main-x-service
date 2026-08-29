# Person Service

A high-performance, enterprise-grade Person Service system built with Rust.

[![Rust](https://img.shields.io/badge/rust-1.96%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](Cargo.toml)
[![Podman](https://img.shields.io/badge/podman-ready-brightgreen.svg)](Dockerfile)

## Overview

The Person Service is an identity-registry system that maintains a centralized registry of person identities across multiple source systems. This production-ready implementation provides:

- ✅ **Person matcher**: Probabilistic and deterministic matching algorithms
- ✅ **Full-Text Search**: Powered by Tantivy for fast, accurate person searches
- ✅ **RESTful API**: Modern HTTP API with OpenAPI/Swagger documentation
- ✅ **Event Streaming**: Real-time person event publishing with audit logging
- ✅ **Database Integration**: PostgreSQL with SeaORM and migrations
- ✅ **Podman Ready**: Multi-stage container builds, Podman Compose for dev/test/prod
- ✅ **Integration Tests**: Comprehensive test coverage
- ✅ **Production Hardened**: Security, monitoring, and compliance features

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Podman Deployment](#podman-deployment)
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

### Person Management

- ✅ Create, read, update, and delete (CRUD) person records
- ✅ Soft delete support with complete audit trails
- ✅ Person identifier management (MRN, SSN, national IDs)
- ✅ Tax ID storage and matching (CPF, SSN, TIN)
- ✅ Identity document management (passport, birth certificate, national ID, driver's license, military ID, voter ID, residence/work permits)
- ✅ Multiple names and addresses per person
- ✅ Contact information management
- ✅ Emergency contact management (name, relationship, telecom, address, primary flag)
- ✅ Automatic event publishing for all CRUD operations

### Data Quality & Validation

- ✅ Required field enforcement (family name, given name)
- ✅ Birth date validation (no future dates)
- ✅ Tax ID format validation
- ✅ Email format validation
- ✅ Phone number digit count validation
- ✅ Address validation (requires city, postal code, or country)
- ✅ Document validation (required number, expiry check, issue-before-expiry)
- ✅ Emergency contact validation (name and relationship required)
- ✅ Phone number normalization (E.164-like format)
- ✅ Address standardization (title-case city, uppercase state/country, expand abbreviations)
- ✅ Per-field length + array-cardinality input-size caps (SEC-M1)
- ✅ Validation integrated into create and update handlers (returns 422)

### Person matcher

- ✅ **Probabilistic Matching**: Advanced fuzzy matching algorithms
- ✅ **Deterministic Matching**: Rule-based exact matching
- ✅ **Configurable Scoring**: Customizable match thresholds and weights
- ✅ **Match Components**:
  - Name matching (Jaro-Winkler, Levenshtein, Soundex phonetic)
  - Date of birth matching with error tolerance
  - Gender matching
  - Address matching (postal code, city, state)
  - Identifier matching
  - Tax ID exact match (deterministic, short-circuits to 1.0)
  - Document number match (type + number)

### Search Capabilities

- ✅ Full-text search across all person fields
- ✅ Fuzzy search with configurable tolerance
- ✅ Advanced query syntax (AND, OR, NOT)
- ✅ High-performance indexing with Tantivy
- ✅ Search by name and birth year
- ✅ Automatic index synchronization with database

### Event Streaming & Audit

- ✅ **Event Publishing**: Automatic events for all person changes
  - PersonCreated, PersonUpdated, PersonDeleted
  - PersonMerged, PersonLinked, PersonUnlinked
- ✅ **Audit Logging**: Complete audit trail in PostgreSQL
  - Old/new values as JSON
  - User tracking (user_id, ip_address, user_agent)
  - Timestamp-based audit history
- ✅ **Audit Query API**: REST endpoints for audit log access
  - Get person audit history
  - Get recent system-wide audits
  - Get user-specific audit logs

### RESTful API

- ✅ OpenAPI 3.0 specification
- ✅ Interactive Swagger UI
- ✅ JSON request/response format
- ✅ CORS support for web applications
- ✅ Comprehensive error handling
- ✅ HTTP status codes following REST conventions
- ✅ **Endpoints**:
  - `GET /api/health` - Health check
  - `POST /api/persons` - Create person
  - `GET /api/persons/{id}` - Get person
  - `PUT /api/persons/{id}` - Update person
  - `DELETE /api/persons/{id}` - Delete person (soft)
  - `GET /api/persons/search` - Search persons
  - `POST /api/persons/match` - Match person records
  - `GET /api/persons/review-queue` - Stored dedup review queue (filter `status`, `limit`)
  - `POST /api/persons/review-queue/{id}/decision` - Decide a pending review item (`confirmed` / `rejected`)
  - `GET /api/persons/{id}/audit` - Get audit logs
  - `GET /api/audit/recent` - Recent audit activity
  - `GET /api/audit/user` - User audit logs
  - Plus cross-service links, bulk import/export, and compliance
    endpoints — see the full table in
    [agents/restful.md](agents/restful.md)

### High Availability

- ✅ Database connection pooling with configurable limits
- ✅ Health check endpoints for orchestration
- ✅ Graceful shutdown
- ✅ Horizontal scaling support (stateless design)
- ✅ Podman health checks
- ✅ Non-root container execution

### Observability

- ✅ Structured logging with `tracing` crate
- ✅ Configurable log levels (RUST_LOG)
- ✅ Request/response logging
- ✅ Error logging with context
- ✅ Prometheus metrics endpoint (`GET /metrics.prom`)
- ⏳ OpenTelemetry export — an `src/observability/` module builds an OTel
  `Resource` and installs a plain JSON subscriber, but the OTLP exporter
  and metrics pipeline are `todo!()`/commented out pending the OTLP
  pipeline; no span or metric is exported over OTLP today (see
  [`agents/share/overview.md`](../../agents/share/overview.md)'s honest
  capability matrix — this holds for person, worker, and event alike)

## Quick Start

### Option 1: Podman (Recommended)

```bash
# Clone repository
git clone https://github.com/SixArm/main-x-service.git
cd main-x-service/person/person-service-with-loco

# Copy environment configuration
cp .env.example .env

# Start all services (PostgreSQL + Person Server)
podman compose up -d

# View logs
podman compose logs -f person-server

# Access the API
curl http://localhost:8080/api/health
```

**Services Available:**

- **API**: http://localhost:8080/api
- **Swagger UI**: http://localhost:8080/swagger-ui
- **pgAdmin** (optional): http://localhost:5050
  ```bash
  podman compose --profile tools up -d
  ```

See [DEPLOY.md](DEPLOY.md) for complete deployment guide.

### Option 2: Local Development

**Prerequisites:**

- Rust 1.96+ ([Install Rust](https://rustup.rs/))
- PostgreSQL 18+
- No extra CLI tooling required: migrations run through the built-in
  loco CLI (`cargo loco db migrate`)

```bash
# Clone repository
git clone https://github.com/SixArm/main-x-service.git
cd main-x-service/person/person-service-with-loco

# Set up database
createdb person_service
cp .env.example .env
# Edit .env and set DATABASE_URL

# Build and run (loco.rs). Migrations run automatically in development
# (auto_migrate); or run them explicitly with `cargo loco db migrate`.
export DATABASE_URL=postgres://localhost/person_service_development
cargo loco start            # or: cargo run -- start
```

## Podman Deployment

### Development Environment

```bash
# Start services
podman compose up -d

# Run migrations (first time)
podman compose exec person-server sea-orm-cli migrate up

# View logs
podman compose logs -f

# Stop services
podman compose down
```

### Testing Environment

```bash
# Start the containerised Postgres (Podman)
scripts/test-db.sh up person/person-service-with-loco

# Run the DB-gated suite exactly as CI does
scripts/ci-check.sh test-db person/person-service-with-loco

# Watch the database log / clean up
scripts/test-db.sh logs person/person-service-with-loco
scripts/test-db.sh down person/person-service-with-loco
```

### Production Deployment

```bash
# Copy production config
cp .env.production.example .env.production

# Build production image
podman build -t person-server:v1.0.0 .

# Run with production config
podman run -p 8080:8080 --env-file .env.production person-server:v1.0.0
```

See [DEPLOY.md](DEPLOY.md) for comprehensive deployment instructions.

## Technology Stack

| Component            | Technology                           | Purpose                                  |
| -------------------- | ------------------------------------ | ---------------------------------------- |
| **Language**         | Rust 1.96+ 2024 Edition              | Systems programming, performance, safety |
| **Async Runtime**    | Tokio                                | Asynchronous I/O and concurrency         |
| **Web Framework**    | Axum                                 | HTTP server and routing                  |
| **Web Framework**    | Loco                                 | HTTP server and routing                  |
| **Database**         | PostgreSQL 18+                       | Data persistence                         |
| **ORM**              | SeaORM                               | Async database object-relational mapper  |
| **Search Engine**    | Tantivy                              | Full-text search indexing                |
| **Event Streaming**  | In-Memory (extendable to Kafka/NATS) | Event publishing                         |
| **API Docs**         | Utoipa                               | OpenAPI 3.0 specification                |
| **Serialization**    | Serde                                | JSON serialization/deserialization       |
| **Logging**          | Tracing                              | Structured logging                       |
| **Observability**    | OpenTelemetry                        | Structured observability                 |
| **String Matching**  | strsim, fuzzy-matcher                | Jaro-Winkler, Levenshtein                |
| **Containerization** | Podman                               | Deployment packaging                     |

## Architecture

### System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Layer                            │
│  (Web Apps, Mobile Apps, EHR Systems, Analytics Platforms)     │
└────────────────────────┬────────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────────┐
│                      REST API Layer (Axum)                       │
│  - OpenAPI/Swagger Documentation                                 │
│  - JSON Request/Response                                         │
│  - CORS, Error Handling                                          │
└────────────────────────┬────────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────────┐
│                   Business Logic Layer                           │
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────────────┐ │
│  │   Person    │  │    Matching   │  │   Search Engine      │ │
│  │  Repository  │  │   Algorithms  │  │     (Tantivy)        │ │
│  └──────────────┘  └───────────────┘  └──────────────────────┘ │
│  ┌──────────────┐  ┌───────────────┐                            │
│  │    Event     │  │     Audit     │                            │
│  │  Publisher   │  │  Log Tracking │                            │
│  └──────────────┘  └───────────────┘                            │
└────────────────────────┬────────────────────────────────────────┘
                         │
         ┌───────────────┼───────────────────────┐
         │               │                       │
┌────────▼─────┐  ┌──────▼──────┐  ┌────────────▼──────┐
│  PostgreSQL  │  │   Tantivy   │  │  Event Stream     │
│  (SeaORM)    │  │   Search    │  │  (In-Memory)      │
│              │  │   Index     │  │                   │
│  - persons  │  │             │  │  - PersonEvents  │
│  - audit_log │  │             │  │  - Subscribers    │
└──────────────┘  └─────────────┘  └───────────────────┘
```

### Data Flow

**Person Creation Flow:**

1. HTTP POST → REST API Handler
2. JSON Deserialization → Person Model
3. Validation (required fields, format checks, size caps) → `422` on failure
4. Duplicate Detection (search + match against existing) → `409` with candidates on a hit
5. Repository `create()` → Database INSERT
6. Search Engine `index_person()` → Tantivy Index
7. Event Publisher → PersonCreated Event
8. Audit Logger → audit_log INSERT
9. HTTP Response → Client

**Person Merge Flow:**

1. HTTP POST `/merge` → REST API Handler (rejects `main == duplicate` with `422`)
2. Fetch main and duplicate from database (locked `FOR UPDATE`)
3. Transfer data from duplicate to main
4. Update main in database
5. Soft-delete duplicate
6. Update search index
7. Publish Merged event (+ `merged_from`) in the same transaction as the writes
8. Return merge record with transferred data

**Person Search Flow:**

1. HTTP GET → REST API Handler
2. Search Engine `search()` → Tantivy Query
3. Person IDs → Repository `get_by_id()` batch
4. Optional: mask sensitive data (per-record ABAC decision when `PERSON_REQUIRE_AUTH` is on)
5. Person Records → JSON Serialization
6. HTTP Response → Client (with pagination headers)

### Component Details

See [spec/08-architecture.md](spec/08-architecture.md) for detailed architecture documentation.

## Development

### Building the Project

```bash
# Development build (fast compile, unoptimized)
cargo build

# Release build (optimized, slower compile)
cargo build --release

# Check compilation without building
cargo check
```

### Running the Server

```bash
# Development mode with auto-reload (requires cargo-watch)
cargo install cargo-watch
cargo watch -x run

# Production mode
cargo run --release

# With custom log level
RUST_LOG=debug cargo run
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy

# Run linter with all warnings
cargo clippy -- -W clippy::all -W clippy::pedantic

# Fix auto-fixable issues
cargo fix --allow-dirty
```

### Database Migrations

```bash
# Create new migration
sea-orm-cli migrate generate migration_name

# Run pending migrations
sea-orm-cli migrate up

# Revert last migration
sea-orm-cli migrate down

# Check migration status
sea-orm-cli migrate status
```

### Seed demo data

`cargo loco task seed_examples` loads the repository's shared demo
fixture ([`examples/data/persons.jsonl`](../../examples/data/README.md),
50 rows including five deliberate duplicate pairs) into the `persons`
table, for the tutorials. It inserts via the model-layer create
(`db::repositories::SeaOrmPersonRepository::create`) rather than
`POST /api/persons`, deliberately bypassing real-time duplicate
detection — the normal create endpoint returns `409` on the second half
of every duplicate pair, which would silently drop half the fixture.
No audit row or event is written by the seed itself. It refuses to
insert into a non-empty `persons` table (prints a message and exits
cleanly), so a second run is a no-op:

```bash
cargo loco task seed_examples
```

See the sibling `organization-service`/`case-service` crates for the
matching `seed_examples` task over `examples/data/organizations.jsonl`
/ `cases.jsonl` (repo `tasks.md` EX-4).

## API Documentation

### Interactive Documentation

Access the Swagger UI at **http://localhost:8080/swagger-ui** for interactive API exploration.

### Quick Examples

**Create Person:**

```bash
curl -X POST http://localhost:8080/api/persons \
  -H "Content-Type: application/json" \
  -d '{
    "name": {
      "use": "official",
      "family": "Smith",
      "given": ["John", "Robert"]
    },
    "birth_date": "1980-01-15",
    "gender": "male"
  }'
```

**Search Persons:**

```bash
curl "http://localhost:8080/api/persons/search?q=Smith&limit=10"
```

**Match Person:**

```bash
curl -X POST http://localhost:8080/api/persons/match \
  -H "Content-Type: application/json" \
  -d '{
    "person": {
      "name": {
        "family": "Smyth",
        "given": ["Jon"]
      },
      "birth_date": "1980-01-15"
    },
    "threshold": 0.7
  }'
```

**Check for Duplicates (without creating):**

```bash
curl -X POST http://localhost:8080/api/persons/check-duplicates \
  -H "Content-Type: application/json" \
  -d '{ "name": { "family": "Smith", "given": ["John"] }, "birth_date": "1980-01-15", "gender": "male" }'
```

**List the Review Queue:**

```bash
curl "http://localhost:8080/api/persons/review-queue?status=pending&limit=20"
```

**Decide a Review Item:**

```bash
curl -X POST http://localhost:8080/api/persons/review-queue/{id}/decision \
  -H "Content-Type: application/json" \
  -d '{ "status": "confirmed" }'
```

**Merge Two Persons:**

```bash
curl -X POST http://localhost:8080/api/persons/merge \
  -H "Content-Type: application/json" \
  -d '{ "main_person_id": "uuid-main", "duplicate_person_id": "uuid-dup", "merge_reason": "Confirmed duplicate" }'
```

**Batch Deduplication:**

```bash
curl -X POST http://localhost:8080/api/persons/deduplicate \
  -H "Content-Type: application/json" \
  -d '{ "threshold": 0.7, "auto_merge_threshold": 0.95, "max_candidates": 50 }'
```

**GDPR Data Export:**

```bash
curl "http://localhost:8080/api/persons/{id}/export"
```

**Masked Person View:**

```bash
curl "http://localhost:8080/api/persons/{id}/masked"
```

**Get Audit Logs:**

```bash
curl "http://localhost:8080/api/persons/{id}/audit?limit=50"
```

See [agents/restful.md](agents/restful.md) for complete API documentation
(including the cross-service links, bulk import/export, and
audit/compliance endpoints not shown above).

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
| `GRPC_PORT` | gRPC server port (Tonic stub) | 50051 | No |
| `SEARCH_CACHE_SIZE_MB` | Tantivy cache budget in MB | 512 | No |
| `OTLP_SERVICE_NAME` | service.name sent to the collector | person-service | No |
| `OTLP_ENDPOINT` | OTLP collector endpoint | http://localhost:4317 | No |
| `STREAMING_BROKER_URL` | Event-broker connection URL | localhost:9003 | No |
| `STREAMING_TOPIC` | Topic events publish to | person-events | No |
| `MATCHING_NAME_WEIGHT`     | Name matching weight         | 0.4            | No       |
| `MATCHING_DOB_WEIGHT`      | DOB matching weight          | 0.3            | No       |
| `MATCHING_GENDER_WEIGHT`   | Gender matching weight       | 0.1            | No       |
| `MATCHING_ADDRESS_WEIGHT`  | Address matching weight      | 0.2            | No       |
| `RUST_LOG`                 | Logging level                | info           | No       |

See `.env.example` for complete configuration template.

## Testing

### Unit Tests

```bash
# Run all unit tests
cargo test --lib

# Run specific test
cargo test test_person_matcher

# Run with output
cargo test -- --nocapture

# Run with specific log level
RUST_LOG=debug cargo test
```

### Integration Tests

```bash
# Run all integration tests
cargo test --test api_integration_test

# Run specific integration test
cargo test --test api_integration_test test_create_person

# Run with Podman (recommended)
scripts/test-db.sh up person/person-service-with-loco --build
```

### Test Coverage

**Current Coverage** (2026-08-04; both floors, not exact — re-run the
commands for the live count):

- Unit Tests: 314+ tests covering matching, search, phonetic, validation, privacy, models, review queue, bulk, compliance (run `cargo test --lib` for the live count)
- Integration Tests: 45+ tests across `tests/` — full API workflows, the matcher bridge, ABAC enforcement, review queue, and `seed_examples` (run `scripts/test-db.sh up person/person-service-with-loco && scripts/ci-check.sh test-db person/person-service-with-loco` for the live DB-gated count; `cargo test --tests` alone skips the DB-gated ones)
- Benchmark Suites: 4 (matching, search, validation, and the service↔matcher adapter bridge) — see [benches/](benches/)

See [spec/13-tasks.md](spec/13-tasks.md) for integration testing details.

## Deployment

### Podman Deployment

See [DEPLOY.md](DEPLOY.md) for comprehensive deployment guide.

**Quick Commands:**

```bash
# Development
podman compose up -d

# Testing
scripts/test-db.sh up person/person-service-with-loco

# Production build
podman build -t person-server:v1.0.0 .
```

### Manual Deployment

1. Build release binary: `cargo build --release`
2. Copy binary: `cp target/release/person-service /opt/person-service/`
3. Set up environment: `cp .env.production.example /opt/person-service/.env`
4. Run migrations: `sea-orm-cli migrate up`
5. Start service: `./person-service`

### Kubernetes (Future)

Helm chart and Kubernetes manifests are not yet written.

## Security & Compliance

### Implemented

- ✅ **Audit Logging**: Complete, hash-chained audit trail for HIPAA compliance, plus checkpointing and out-of-band-edit detection (`src/compliance/`)
- ✅ **Soft Delete**: Person records never truly deleted
- ✅ **GDPR Erasure**: `POST /api/persons/{id}/erase` destroys personal data while keeping the audit chain linkage intact
- ✅ **Non-Root Containers**: Podman containers run as non-root user
- ✅ **Environment-Based Secrets**: No secrets in code or images
- ✅ **CORS Configuration**: Configurable cross-origin policies
- ✅ **Input Validation**: Comprehensive validation on create/update (`src/validation/`, returns 422; size-capped per SEC-M1)
- ✅ **Authentication**: offline PASETO v4.public (Ed25519) bearer-token verification against the central authentication-service's published key set, with periodic key-set refresh (no restart needed for a rotation) — this crate is a **resource server**; it verifies tokens, it does not issue the cookie sessions the auth-service itself owns (see [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md))
- ✅ **Authorization**: Attribute-based access control (ABAC) inside the blanket guard — the shared `authentication-verifier` policy engine over the token's `attrs` claim, hot-reloadable from a policy file with no restart (see [`agents/share/authorization-attributes.md`](../../agents/share/authorization-attributes.md)); default-off, a tracked release gate to turn on (`PERSON_REQUIRE_AUTH`)

### Planned

- ⏳ **Encryption at Rest**: Database encryption
- ⏳ **TLS/SSL**: HTTPS enforcement (deployment-layer today, e.g. a reverse proxy — not done in-process)
- ⏳ **Rate Limiting**: API rate limiting

### Compliance Standards

- **HIPAA**: Audit logging, access controls, data encryption
- **GDPR**: Right to access (audit logs), right to deletion
- **HL7 FHIR**: Partial compliance (Person resource)
- **FDA 21 CFR Part 11**: Audit trail capabilities

## Performance

### Benchmarks

Current performance on modest hardware (i5, 16GB RAM):

- **Person Create**: ~50ms (includes DB + search index)
- **Person Read**: ~5ms
- **Person Search**: ~20-100ms (depending on result size)
- **Person Match**: ~100-500ms (depending on candidate count)
- **Concurrent Requests**: 1000+ req/sec

### Optimization

- Database connection pooling (configurable)
- Search index caching
- Async I/O with Tokio
- Release builds with full optimizations
- Efficient data structures (BTreeMap, HashMap)

## Project Structure

```
person-service-with-loco/
├── src/
│   ├── api/
│   │   ├── rest/           # REST handlers, routes, state, auth, links, api-version
│   │   ├── fhir/           # FHIR R5 (Patient primary + Person alias), Bundle, OperationOutcome
│   │   └── grpc/           # gRPC server (stub)
│   ├── db/
│   │   ├── models.rs       # SeaORM entities
│   │   ├── schema.rs       # SeaORM schema
│   │   ├── repositories.rs # Data access layer (CRUD, search, merge)
│   │   ├── audit.rs        # Audit log repository
│   │   ├── outbox.rs       # Transactional outbox (durable event bus)
│   │   ├── entity_links.rs # Cross-service `entity_links` persistence
│   │   ├── review_queue.rs # Dedup review-queue persistence
│   │   ├── bulk_jobs.rs    # Bulk import/export job persistence
│   │   └── convert.rs      # Domain ⇄ SeaORM conversions
│   ├── matching/
│   │   ├── algorithms.rs   # Matching algorithms (name, DOB, gender, address, identifier, tax_id, document)
│   │   ├── phonetic.rs     # Soundex phonetic matching
│   │   ├── scoring.rs      # Probabilistic + deterministic scoring
│   │   ├── adapter.rs      # Bridge to the embedded `person-matcher` crate
│   │   └── mod.rs          # Matcher trait implementations
│   ├── search/
│   │   ├── index.rs        # Tantivy search index
│   │   ├── query.rs        # Query building
│   │   └── mod.rs          # Search engine interface
│   ├── streaming/
│   │   ├── envelope.rs     # Canonical durable event `Envelope`
│   │   ├── producer.rs     # Event publisher
│   │   ├── consumer.rs     # Event consumer (stub)
│   │   └── mod.rs          # Event types + transport selection
│   ├── models/
│   │   ├── person.rs       # Person, HumanName, PersonLink, …
│   │   ├── identifier.rs   # Identifier types
│   │   ├── document.rs     # Identity document types
│   │   ├── emergency_contact.rs
│   │   ├── merge.rs        # MergeRequest/Response/Record
│   │   ├── review_queue.rs # Dedup review queue items
│   │   ├── consent.rs      # Consent management
│   │   ├── organization.rs
│   │   └── mod.rs          # Shared models (Gender, Address, ContactPoint)
│   ├── validation/         # Data-quality validation, normalization → 422
│   ├── privacy/            # Data masking, consent checking, GDPR export
│   ├── bulk/                # Bulk import/export: JSONL/CSV/Parquet codecs, worker, artifact store
│   ├── compliance/         # SBOM/SOUP, audit-chain verification, checkpoints, erasure, disclosure accounting
│   ├── tasks/               # loco CLI tasks (`seed_examples`, integrity key/resign)
│   ├── config/              # Configuration management
│   ├── observability/       # Tracing, Prometheus metrics, OTel resource
│   ├── relay.rs             # Outbox relay (`LoggingSink` / `FluvioSink`)
│   ├── metrics.rs           # Prometheus metric inventory
│   ├── error.rs             # Error types
│   ├── app.rs               # loco `Hooks` (boot, routes, workers)
│   └── lib.rs                # Library root
├── migrations/             # Raw SQL, wrapped via include_str! by migration/
├── migration/              # Migrator crate root
├── tests/                  # Integration tests (api_integration_test, duplicate_detection, enforcement, review_queue_db, seed_examples_db, fluvio_relay)
├── benches/                # Criterion benchmarks (matching, search, validation)
├── fuzz/                   # cargo-fuzz targets
├── Dockerfile              # Production container (build context = repo root)
├── docker-compose.yml      # Development environment
├── compose.test.yaml       # Test database (Podman)
├── compose.fluvio.yaml     # Opt-in local Fluvio broker (manual runs only)
├── DEPLOY.md               # Deployment guide
└── README.md               # Symlink to this file
```

## Development Phases

This project was developed in phases, then continued as ongoing
spec-driven work (see [spec/13-tasks.md](spec/13-tasks.md) for what has
landed since):

1. **Phase 1-6**: Core infrastructure, models, configuration
2. **Phase 7**: Database Integration (SeaORM, PostgreSQL)
3. **Phase 8**: Event Streaming & Audit Logging
4. **Phase 9**: REST API Implementation
5. **Phase 10**: Integration Testing
6. **Phase 11**: Docker & Deployment
7. **Phase 12**: Documentation
8. **Phase 13**: Advanced identity-matching features (duplicate detection, merging, deduplication, validation, privacy, emergency contacts, identity documents, phonetic matching)
9. **Phase 14 onward**: FHIR R5 reconciliation, the durable event bus (outbox → relay → `FluvioSink`), cross-service links, bulk import/export (JSONL/CSV/Parquet + S3), the compliance module (audit-chain verification, checkpoints, erasure, disclosure accounting), PASETO key rotation + ABAC policy hot-reload, and the tutorial fixtures (`seed_examples`)

See [spec/13-tasks.md](spec/13-tasks.md) for the live task queue and [spec/14-implementation-status.md](spec/14-implementation-status.md) for implementation status.

## Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Guidelines

- Follow Rust style guide (`cargo fmt`)
- Pass all tests (`cargo test`)
- Pass clippy lints (`cargo clippy`)
- Add tests for new features
- Update documentation

## License

This project is multi-licensed under the SPDX expression declared in
[`Cargo.toml`](Cargo.toml):

```
MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only
```

You may choose any one of these licenses for your use.

## Support

- **Issues**: [GitHub Issues](https://github.com/sixarm/person-service-with-loco/issues)
- **Discussions**: [GitHub Discussions](https://github.com/sixarm/person-service-with-loco/discussions)
- **Email**: support@example.com

## Acknowledgments

Built with excellent Rust crates:

- [Tokio](https://tokio.rs/) - Async runtime
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SeaORM](https://www.sea-ql.org/SeaORM/) - Async ORM and query builder
- [Loco](https://loco.rs/) - Web framework conventions (backend-only)
- [OpenTelemetry](https://opentelemetry.io/) - Observability framework
- [Tantivy](https://github.com/tantivy-search/tantivy) - Search engine
- [Serde](https://serde.rs/) - Serialization
- [Utoipa](https://github.com/juhaku/utoipa) - OpenAPI documentation
- [Tracing](https://github.com/tokio-rs/tracing) - Logging

And many more listed in `Cargo.toml`.

---

**Status**: See [spec/13-tasks.md](spec/13-tasks.md) (live task queue) and [spec/14-implementation-status.md](spec/14-implementation-status.md) (canonical implementation status).
**Version**: 0.5.0 (see [Cargo.toml](Cargo.toml) / [CHANGELOG.md](CHANGELOG.md) for the current value — this field drifts easily, so treat it as a pointer, not a source of truth)
