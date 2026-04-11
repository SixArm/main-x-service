# Main Place Index (MPI) - Implementation Plan

## Technology Stack

Application:

- Programming language: Rust <https://rust-lang.org/>
- Asynchronous runtime: Tokio <https://docs.rs/tokio/latest/tokio/>
- Search engine: Tantivy <https://docs.rs/tantivy/latest/tantivy/>
- Observability: OpenTelemetry logs metrics traces <https://docs.rs/opentelemetry/latest/opentelemetry/>

Data:

- Database: PostgreSQL 15+ <https://www.postgresql.org/>
- Database ORM: SeaORM <https://docs.rs/sea-orm/latest/sea_orm/>
- Data streaming: Fluvio <https://docs.rs/fluvio/latest/fluvio/>

API:

- HTTP: Hyper <https://docs.rs/hyper/latest/hyper>
- RESTful: Axum web application framework <https://docs.rs/axum/latest/axum/>
- JSON: Serde JSON <https://docs.rs/serde_json/latest/serde_json/>
- OpenAPI v3: Utoipa <https://docs.rs/utoipa/latest/utoipa/>
- gRPC: Tonic <https://docs.rs/tonic/latest/tonic/>

Geo:

- Geo coordinates: geo crate <https://docs.rs/geo/latest/geo/>
- Distance calculations: haversine <https://docs.rs/haversine/latest/haversine/>

Testing:

- Unit testing: Assertables <https://docs.rs/assertables/latest/assertables/>
- Benchmark testing: Criterion <https://docs.rs/criterion/latest/criterion/>
- Mutation testing: cargo-mutants

Deployment:

- Infrastructure as Code: OpenTofu <https://opentofu.org>
- Multi-cloud deployments: Google Cloud, Amazon Cloud, Microsoft Cloud
- Containerization: Docker, Docker Compose, Kubernetes

## Domain Model

Based on [schema.org/Place](https://schema.org/Place):

- `name` (Text) - Primary name of the place
- `alternate_name` (Text) - Alternate names / aliases
- `description` (Text) - Description of the place
- `place_type` (Text) - Type classification (LocalBusiness, CivicStructure, AdministrativeArea, Landform, etc.)
- `address` (PostalAddress) - street_address, address_locality, address_region, address_country, postal_code
- `geo` (GeoCoordinates) - latitude, longitude, elevation
- `global_location_number` (Text) - 13-digit GLN identifier
- `branch_code` (Text) - Short textual code for a place of business
- `contained_in_place` (Place) - Parent place in hierarchy
- `contains_place` (Place[]) - Child places in hierarchy
- `telephone` (Text) - Phone number
- `fax_number` (Text) - Fax number
- `url` (URL) - Website
- `keywords` (Text[]) - Tags / keywords
- `opening_hours_specification` (OpeningHoursSpecification) - Operating hours
- `is_accessible_for_free` (Boolean) - Free access
- `public_access` (Boolean) - Open to public
- `smoking_allowed` (Boolean) - Smoking permitted
- `maximum_attendee_capacity` (Integer) - Max capacity
- `amenity_feature` (LocationFeatureSpecification[]) - Features / amenities
- `same_as` (URL[]) - Links to authoritative sources for deduplication

## Production Requirements

- Millions of places
- Thousands of data sources
- High availability disaster recovery (HADR)
- Fault tolerance
- GDPR compliance

## Completed Phases

### Phase 1: Project Setup & Foundation

- Rust project with Cargo, 40+ dependencies configured
- Modular architecture: api, models, db, matching, search, streaming, observability, config, error, validation, privacy

### Phase 2: Database Schema & Models

- 13 PostgreSQL tables with Diesel ORM
- 5 migrations (365 lines SQL), 27 Diesel models
- 40+ strategic indexes, audit triggers

### Phase 3: Core MPI Logic

- Probabilistic matching (Jaro-Winkler, Levenshtein, Soundex phonetic)
- Deterministic matching (exact identifier, GLN)
- Geo coordinate distance matching (Haversine)
- Configurable scoring weights and thresholds

### Phase 4: Search Engine Integration

- Tantivy full-text search with 11 indexed fields
- Fuzzy search, bulk indexing, blocking strategy
- Geo-radius search support

### Phase 5: RESTful API (Axum)

- 15 endpoints with OpenAPI/Swagger documentation
- CORS, structured error handling

### Phase 6: Data Model (schema.org/Place)

- Full schema.org/Place property coverage
- PostalAddress and GeoCoordinates models
- Place hierarchy (containedInPlace / containsPlace)

### Phase 7: Database Integration

- DieselPlaceRepository with full CRUD, transactions, soft delete
- Bidirectional domain/DB model conversion

### Phase 8: Event Streaming & Audit Logging

- InMemoryEventPublisher for all place lifecycle events
- AuditLogRepository with old/new JSON snapshots and user context

### Phase 9: REST API Implementation

- 10 core endpoints + 5 new endpoints for dedup/privacy
- Automatic event publishing and audit logging

### Phase 10: Integration Testing

- 7 integration tests covering full HTTP lifecycle
- Real dependencies (PostgreSQL, Tantivy)

### Phase 11: Docker & Deployment

- Multi-stage Dockerfile (85% smaller images)
- Docker Compose for dev and test environments
- DEPLOY.md guide

### Phase 12: Documentation

- README.md, architecture diagrams, API examples

### Phase 13: Advanced MPI Features

- Place identity: GLN, place type, amenity features
- Duplicate detection: real-time (409 on create) + explicit endpoint
- Record merging: data transfer, link creation, soft-delete duplicate
- Batch deduplication: pairwise scan, review queue, auto-merge
- Phonetic matching: Soundex algorithm integrated into name matching
- Geo matching: Haversine distance for coordinate-based deduplication
- Data quality: validation rules, coordinate normalization, address standardization
- Privacy: data masking, GDPR export, consent model

## Current Status

- 104 unit tests passing
- 67 integration tests passing
- 16 Criterion benchmark tests
- 0 compilation errors, 0 clippy warnings
- Build: cargo check, cargo test, cargo clippy all pass clean

### Implemented Library Modules

- `models/` - Domain models: Place, PostalAddress, GeoCoordinates, PlaceType, PlaceIdentifier, AmenityFeature, OpeningHoursSpecification, Consent
- `matching/` - Matching algorithms: name (Jaro-Winkler), address (weighted), geo (Haversine), identifier (GLN deterministic), phonetic (Soundex), scoring (weighted aggregation with confidence levels)
- `validation/` - Validation rules and address normalization
- `privacy/` - Data masking and GDPR export

### Phase 15: Update & Expand Tests

- Fixed integration test imports for renamed package
- Fixed Cargo.toml metadata (description, keywords)
- Fixed file extension and typos in documentation
- Added 53 new integration tests (models, scoring, edge cases)
- Added 4 new privacy benchmarks
- Updated all documentation files

## Next Phases (Planned)

### Phase 16: Authentication & Authorization

- JWT-based authentication
- Role-based access control (RBAC)
- Rate limiting

### Phase 17: Observability & Monitoring

- Prometheus metrics integration
- Distributed tracing with OpenTelemetry
- Custom dashboards and alerting

### Phase 18: Performance Optimization

- Database query caching
- Benchmark tests with Criterion
- Load testing at scale (millions of places)

### Phase 19: Infrastructure as Code (OpenTofu)

- Multi-cloud deployment modules (GCP, AWS, Azure)
- VPC, load balancers, security groups

### Phase 20: Kubernetes

- Helm charts
- Horizontal pod autoscaling
- Persistent volume claims

### Phase 21: Production Readiness

- Security review and penetration testing
- Disaster recovery drills
- GDPR compliance validation

### Phase 22: Continuous Improvement

- ML-based match scoring
- A/B testing for matching algorithms
- CI/CD pipeline
