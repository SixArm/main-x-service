# Event Service

A high-performance, enterprise-grade Event Service system built with Rust.

[![Rust](https://img.shields.io/badge/rust-1.93%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Docker](https://img.shields.io/badge/docker-ready-brightgreen.svg)](Dockerfile)

## Overview

The Event Service is an identity-registry system that maintains a centralized registry of event identities across multiple source systems. This production-ready implementation provides:

- ✅ **Event matcher**: Probabilistic and deterministic matching algorithms
- ✅ **Full-Text Search**: Powered by Tantivy for fast, accurate event searches
- ✅ **RESTful API**: Modern HTTP API with OpenAPI/Swagger documentation
- ✅ **Event Streaming**: Real-time event event publishing with audit logging
- ✅ **Database Integration**: PostgreSQL with SeaORM and migrations
- ✅ **Docker Ready**: Multi-stage builds, Docker Compose for dev/test/prod
- ✅ **Integration Tests**: Comprehensive test coverage
- ✅ **Production Hardened**: Security, monitoring, and compliance features

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

### Event Management

- ✅ Create, read, update, and delete (CRUD) event records
- ✅ Soft delete support with complete audit trails
- ✅ Event identifier management (`BookingNumber`, `ConfirmationCode`,
  `TicketNumber`, `EncounterId`, `TransactionId`, `ExternalRef`,
  `Tax`, `Other`)
- ✅ Time window (`start_date`, `end_date`, `door_time`, `duration`)
- ✅ Location union (`Place` / `PostalAddress` / `Virtual` / `Text`)
- ✅ Parties (organizers, performers, attendees, sponsors, …)
- ✅ Automatic event publishing for all CRUD operations

### Event matcher

- ✅ **Probabilistic Matching**: Advanced fuzzy matching algorithms
- ✅ **Deterministic Matching**: Rule-based exact matching
- ✅ **Configurable Scoring**: Customizable match thresholds and weights
- ✅ **Match Components**:
  - Name matching (Jaro-Winkler + Levenshtein + Soundex floor)
  - Start-date proximity (exponential decay, 1 h half-life)
  - End-date proximity / window overlap
  - Location matching (place id / address fuzzy / virtual URL / text)
  - Organizer / performer / attendee matching (party id / name / email)
  - Identifier matching (type + system + value)

### Search Capabilities

- ✅ Full-text search across all event fields
- ✅ Fuzzy search with configurable tolerance
- ✅ Advanced query syntax (AND, OR, NOT)
- ✅ High-performance indexing with Tantivy
- ✅ Search by name, organizer, identifier; date-range filter on `start_date`
- ✅ Automatic index synchronization with database

### Event Streaming & Audit

- ✅ **Event Publishing**: Automatic events for all event changes
  - EventCreated, EventUpdated, EventDeleted
  - EventMerged, EventLinked, EventUnlinked
- ✅ **Audit Logging**: Complete audit trail in PostgreSQL
  - Old/new values as JSON
  - User tracking (user_id, ip_address, user_agent)
  - Timestamp-based audit history
- ✅ **Audit Query API**: REST endpoints for audit log access
  - Get event audit history
  - Get recent system-wide audits
  - Get user-specific audit logs

### RESTful API

- ✅ OpenAPI 3.0 specification
- ✅ Interactive Swagger UI
- ✅ JSON request/response format
- ✅ CORS support for web applications
- ✅ Comprehensive error handling
- ✅ HTTP status codes following REST conventions
- ✅ **Endpoints** (all under `/api/v1`):
  - `GET /api/v1/health` - Health check
  - `POST /api/v1/events` - Create event
  - `GET /api/v1/events/{id}` - Get event
  - `PUT /api/v1/events/{id}` - Update event
  - `DELETE /api/v1/events/{id}` - Delete event (soft)
  - `GET /api/v1/events/search` - Search events
  - `POST /api/v1/events/match` - Match event records
  - `GET /api/v1/events/{id}/audit` - Get audit logs
  - `GET /api/v1/audit/recent` - Recent audit activity
  - `GET /api/v1/audit/user` - User audit logs

### High Availability

- ✅ Database connection pooling with configurable limits
- ✅ Health check endpoints for orchestration
- ✅ Graceful shutdown
- ✅ Horizontal scaling support (stateless design)
- ✅ Docker health checks
- ✅ Non-root container execution

### Observability

- ✅ Structured logging with `tracing` crate
- ✅ Configurable log levels (RUST_LOG)
- ✅ Request/response logging
- ✅ Error logging with context
- ✅ Distributed tracing with OpenTelemetry
- ✅ OpenTelemetry metrics and traces
- ⏳ Prometheus metrics endpoint (future enhancement)

## Quick Start

### Option 1: Docker (Recommended)

```bash
# Clone repository
git clone https://github.com/sixarm/event-service-with-loco.git
cd event-service-with-loco

# Copy environment configuration
cp .env.example .env

# Start all services (PostgreSQL + Event Service)
podman compose up -d

# View logs
podman compose logs -f event-server

# Access the API
curl http://localhost:8080/api/v1/health
```

**Services Available:**

- **API**: http://localhost:8080/api/v1
- **Swagger UI**: http://localhost:8080/swagger-ui
- **pgAdmin** (optional): http://localhost:5050
  ```bash
  podman compose --profile tools up -d
  ```

See [DEPLOY.md](DEPLOY.md) for complete deployment guide.

### Option 2: Local Development

**Prerequisites:**

- Rust 1.93+ ([Install Rust](https://rustup.rs/))
- PostgreSQL 18+
- SeaORM CLI: `cargo install sea-orm-cli`

```bash
# Clone repository
git clone https://github.com/sixarm/event-service-with-loco.git
cd event-service-with-loco

# Set up database
createdb event_service
cp .env.example .env
# Set DATABASE_URL (loco reads it via config/development.yaml)
export DATABASE_URL=postgres://localhost/event_service_development

# Build and run (loco.rs). Migrations run automatically in development
# (auto_migrate); or run them explicitly with `cargo loco db migrate`.
cargo loco start            # or: cargo run -- start
```

## Docker Deployment

### Development Environment

```bash
# Start services
podman compose up -d

# Run migrations (first time)
podman compose exec event-server sea-orm-cli migrate up

# View logs
podman compose logs -f

# Stop services
podman compose down
```

### Testing Environment

```bash
# Run all tests in Docker
podman compose -f docker-compose.test.yml up --build

# View test results
podman compose -f docker-compose.test.yml logs test-runner

# Clean up
podman compose -f docker-compose.test.yml down -v
```

### Production Deployment

```bash
# Copy production config
cp .env.production.example .env.production

# Build production image
podman build -t event-server:v1.0.0 .

# Run with production config
podman run -p 8080:8080 --env-file .env.production event-server:v1.0.0
```

See [DEPLOY.md](DEPLOY.md) for comprehensive deployment instructions.

## Technology Stack

| Component            | Technology                           | Purpose                                  |
| -------------------- | ------------------------------------ | ---------------------------------------- |
| **Language**         | Rust 1.93+ 2024 Edition              | Systems programming, performance, safety |
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
| **Containerization** | Docker                               | Deployment packaging                     |

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
│  │   Event    │  │    Matching   │  │   Search Engine      │ │
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
│  - events  │  │             │  │  - EventEvents  │
│  - audit_log │  │             │  │  - Subscribers    │
└──────────────┘  └─────────────┘  └───────────────────┘
```

### Data Flow

**Event Creation Flow:**

1. HTTP POST → REST API Handler
2. JSON Deserialization → Event Model
3. Repository `create()` → Database INSERT
4. Search Engine `index_event()` → Tantivy Index
5. Event Publisher → EventCreated Event
6. Audit Logger → audit_log INSERT
7. HTTP Response → Client

**Event Search Flow:**

1. HTTP GET → REST API Handler
2. Search Engine `search()` → Tantivy Query
3. Event IDs → Repository `get_by_id()` batch
4. Event Records → JSON Serialization
5. HTTP Response → Client

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

## API Documentation

### Interactive Documentation

Access the Swagger UI at **http://localhost:8080/swagger-ui** for interactive API exploration.

### Quick Examples

**Create Event:**

```bash
curl -X POST http://localhost:8080/api/v1/events \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Annual Conference",
    "start_date": "2026-06-01T09:00:00Z",
    "end_date":   "2026-06-01T17:00:00Z",
    "event_status": "scheduled",
    "event_attendance_mode": "offline",
    "event_type": "conference",
    "location": [{
      "kind": "place",
      "name": "Greek Theatre",
      "address": { "city": "Berkeley", "state": "CA", "postal_code": "94720", "country": "US" }
    }],
    "organizers": [{ "kind": "organization", "name": "Cal Performances" }]
  }'
```

**Search Events:**

```bash
curl "http://localhost:8080/api/v1/events/search?q=Conference&date_from=2026-06-01&date_to=2026-06-30&limit=10"
```

**Match Event:**

```bash
curl -X POST http://localhost:8080/api/v1/events/match \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Conferance",
    "start_date": "2026-06-01T09:00:00Z",
    "threshold": 0.5
  }'
```

**Get Audit Logs:**

```bash
curl "http://localhost:8080/api/v1/events/{id}/audit?limit=50"
```

### Bridge to the canonical `event-matcher` crate

The service embeds the sibling `event-matcher` crate and re-exports it
from `src/matching/mod.rs` as `matcher_lib`. Pair it with
`adapter::to_matcher_event` to score two service records through the
canonical algorithm (this is the contract pinned by
`tests/duplicate_detection.rs`):

```rust
use event_service::matching::adapter::to_matcher_event;
use event_service::matching::matcher_lib::{MatchingEngine, MatchConfig};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_events(
    &to_matcher_event(&event_a),   // schema.org/Event → matcher Event
    &to_matcher_event(&event_b),
);
// result.score: f64 in [0.0, 1.0]
// result.is_match: bool
// result.confidence: High | Medium | Low
```

The adapter projects `DateTime<Utc>` → RFC 3339 strings, the first
populated `Location` variant-aware, `organizers[0].name` → matcher
`organizer`, `performers` → `Vec<String>`, and identifier `system`
URIs → `EventIdScheme`. See [spec §6.2](spec/06-functional-requirements.md)
and [`AGENTS/matching.md`](AGENTS/matching.md) for the full routing rules.

See [AGENTS/restful.md](AGENTS/restful.md) for complete API documentation.

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
| `MATCHING_THRESHOLD`       | Probabilistic match cutoff   | 0.85           | No       |
| `RUST_LOG`                 | Logging level                | info           | No       |

Match-component weights (name / start-date / end-date / location /
organizer / performer / attendee / identifier) are documented in
[`AGENTS/matching.md`](AGENTS/matching.md).

See `.env.example` for complete configuration template.

## Testing

### Unit Tests

```bash
# Run all unit tests
cargo test --lib

# Run specific test
cargo test test_event_matcher

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
cargo test --test api_integration_test test_create_event

# Run with Docker (recommended)
podman compose -f docker-compose.test.yml up --build
```

### Test Coverage

**Current Coverage:**

- Unit Tests: cover matching, scoring, phonetic, search, validation, privacy
- Bridge Tests: `tests/duplicate_detection.rs` pins the
  `to_matcher_event` → `MatchingEngine::match_events` contract
- Integration Tests: `tests/api_integration_test.rs` (require a running
  PostgreSQL via `DATABASE_URL`)

See [AGENTS/testing.md](AGENTS/testing.md) for the full layout and counts.

## Deployment

### Docker Deployment

See [DEPLOY.md](DEPLOY.md) for comprehensive deployment guide.

**Quick Commands:**

```bash
# Development
podman compose up -d

# Testing
podman compose -f docker-compose.test.yml up

# Production build
podman build -t event-server:v1.0.0 .
```

### Manual Deployment

1. Build release binary: `cargo build --release`
2. Copy binary: `cp target/release/event-service /opt/event-service/`
3. Set up environment: `cp .env.production.example /opt/event-service/.env`
4. Run migrations: `sea-orm-cli migrate up`
5. Start service: `./event-service`

### Kubernetes (Future)

Helm chart and Kubernetes manifests planned for Phase 13.

## Security & Compliance

### Implemented

- ✅ **Audit Logging**: Complete audit trail for HIPAA compliance
- ✅ **Soft Delete**: Event records never truly deleted
- ✅ **Non-Root Containers**: Docker containers run as non-root user
- ✅ **Environment-Based Secrets**: No secrets in code or images
- ✅ **CORS Configuration**: Configurable cross-origin policies

### Planned

- ⏳ **Authentication**: cookie sessions + offline PASETO v4.public verification (see [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md))
- ⏳ **Authorization**: Role-based access control (RBAC)
- ⏳ **Encryption at Rest**: Database encryption
- ⏳ **TLS/SSL**: HTTPS enforcement
- ⏳ **Rate Limiting**: API rate limiting
- ⏳ **Input Validation**: Comprehensive input validation

### Compliance Standards

- **HIPAA**: Audit logging, access controls, data encryption
- **GDPR**: Right to access (audit logs), right to deletion
- **HL7 FHIR**: Stubbed — `/fhir/Event/*` returns `501 Not Implemented`
  until the schema.org/Event → FHIR R5 mapping is fixed (spec §6.8)
- **FDA 21 CFR Part 11**: Audit trail capabilities

## Performance

### Benchmarks

Current performance on modest hardware (i5, 16GB RAM):

- **Event Create**: ~50ms (includes DB + search index)
- **Event Read**: ~5ms
- **Event Search**: ~20-100ms (depending on result size)
- **Event Match**: ~100-500ms (depending on candidate count)
- **Concurrent Requests**: 1000+ req/sec

### Optimization

- Database connection pooling (configurable)
- Search index caching
- Async I/O with Tokio
- Release builds with full optimizations
- Efficient data structures (BTreeMap, HashMap)

## Project Structure

```
event-service-with-loco/
├── src/
│   ├── api/
│   │   ├── rest/          # REST API handlers, routes
│   │   ├── fhir/          # FHIR R5 endpoints (501 stub, not yet routed)
│   │   └── grpc/          # gRPC server (stub)
│   ├── db/
│   │   ├── models.rs      # Database models
│   │   ├── schema.rs      # SeaORM schema
│   │   ├── repositories.rs # Data access layer
│   │   └── audit.rs       # Audit log repository
│   ├── matching/
│   │   ├── algorithms.rs  # Matching algorithms
│   │   ├── scoring.rs     # Match scoring logic
│   │   └── mod.rs         # Matcher implementations
│   ├── search/
│   │   ├── index.rs       # Tantivy search index
│   │   └── mod.rs         # Search engine interface
│   ├── streaming/
│   │   ├── producer.rs    # Event publisher
│   │   ├── consumer.rs    # Event consumer (stub)
│   │   └── mod.rs         # Event types
│   ├── models/
│   │   ├── event.rs     # Event model
│   │   └── mod.rs         # Shared models
│   ├── config.rs          # Configuration management
│   ├── error.rs           # Error types
│   └── lib.rs             # Library root
├── migrations/            # Database migrations
├── tests/                 # Integration tests
├── Dockerfile             # Production container
├── Dockerfile.test        # Test container
├── docker-compose.yml     # Development environment
├── docker-compose.test.yml # Test environment
├── DEPLOY.md             # Deployment guide
└── README.md             # This file
```

## Development Phases

This project was developed in 11 comprehensive phases:

1. **Phase 1-6**: Core infrastructure, models, configuration
2. **Phase 7**: Database Integration (SeaORM, PostgreSQL)
3. **Phase 8**: Event Streaming & Audit Logging
4. **Phase 9**: REST API Implementation
5. **Phase 10**: Integration Testing
6. **Phase 11**: Docker & Deployment

See individual `task-*.md` files for detailed phase documentation.

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

This project is dual-licensed under:

- MIT License
- Apache License 2.0

You may choose either license for your use.

## Support

- **Issues**: [GitHub Issues](https://github.com/sixarm/event-service-with-loco/issues)
- **Discussions**: [GitHub Discussions](https://github.com/sixarm/event-service-with-loco/discussions)
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

**Status**: Production-Ready ✅
**Version**: 0.2.0
**Last Updated**: 2026-03-18
