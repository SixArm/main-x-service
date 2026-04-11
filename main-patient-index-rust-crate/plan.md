# Main Patient Index (MPI) - Implementation Plan

## Technology Stack

Application:

- Programming language: Rust 1.93+ 2024 Edition <https://rust-lang.org/>
- Asynchronous runtime: Tokio <https://docs.rs/tokio/latest/tokio/>
- Web framework: Axum <https://docs.rs/axum/latest/axum/>
- Full-stack framework: Loco <https://docs.rs/loco-rs/latest/loco_rs/>
- Templating: Tera <https://docs.rs/tera/latest/tera/>
- Search engine: Tantivy <https://docs.rs/tantivy/latest/tantivy/>
- Observability: OpenTelemetry logs metrics traces <https://docs.rs/opentelemetry/latest/opentelemetry/>
- String matching: strsim (Jaro-Winkler, Levenshtein) <https://docs.rs/strsim/latest/strsim/>

Data:

- Database: PostgreSQL 18+ <https://www.postgresql.org/>
- Database ORM: SeaORM <https://docs.rs/sea-orm/latest/sea_orm/>
- Data streaming: Fluvio <https://docs.rs/fluvio/latest/fluvio/>

API:

- HTTP: Hyper <https://docs.rs/hyper/latest/hyper>
- RESTful: Axum web application framework <https://docs.rs/axum/latest/axum/>
- JSON: Serde JSON <https://docs.rs/serde_json/latest/serde_json/>
- OpenAPI v3: Utoipa <https://docs.rs/utoipa/latest/utoipa/>
- HL7 FHIR R5
- gRPC: Tonic <https://docs.rs/tonic/latest/tonic/>

Testing:

- Unit testing: Assertables <https://docs.rs/assertables/latest/assertables/>
- Benchmark testing: Criterion <https://docs.rs/criterion/latest/criterion/>
- Mocking: Mockall <https://docs.rs/mockall/latest/mockall/>
- Mutation testing: cargo-mutants

Deployment:

- Infrastructure as Code: OpenTofu <https://opentofu.org>
- Multi-cloud deployments: Google Cloud, Amazon Cloud, Microsoft Cloud
- Containerization: Docker, Docker Compose, Kubernetes

## Production Requirements

- Millions of patients
- Thousands of clinics
- High availability disaster recovery (HADR)
- Fault tolerance
- HIPAA and GDPR compliance

## Completed Phases

### Phase 1: Project Setup & Foundation

- Rust project with Cargo, 40+ dependencies configured
- Modular architecture: api, models, db, matching, search, streaming, observability, config, error, validation, privacy

### Phase 2: Database Schema & Models

- 12+ PostgreSQL tables with SeaORM
- 5 migrations, SeaORM entity models
- Strategic indexes, HIPAA-compliant audit triggers

### Phase 3: Core MPI Logic

- Probabilistic matching (Jaro-Winkler, Levenshtein, Soundex phonetic)
- Deterministic matching (exact identifier, tax ID, document number)
- Configurable scoring weights and thresholds
- Common name variant recognition

### Phase 4: Search Engine Integration

- Tantivy full-text search with 11 indexed fields
- Fuzzy search, bulk indexing, blocking strategy

### Phase 5: RESTful API (Axum)

- 15 endpoints with OpenAPI/Swagger documentation
- CORS, structured error handling

### Phase 6: FHIR R5 Support

- Bidirectional Patient resource conversion
- FHIR search parameters and OperationOutcome

### Phase 7: Database Integration

- SeaOrmPatientRepository with full CRUD, transactions, soft delete
- Bidirectional domain/DB model conversion

### Phase 8: Event Streaming & Audit Logging

- InMemoryEventPublisher for all patient lifecycle events
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

- README.md, CLAUDE.md, AGENTS/\*.md, architecture diagrams, API examples

### Phase 13: Advanced MPI Features

- Patient identity: tax_id, identity documents, emergency contacts
- Duplicate detection: real-time (409 on create) + explicit endpoint
- Record merging: data transfer, link creation, soft-delete duplicate
- Batch deduplication: pairwise scan, review queue, auto-merge
- Phonetic matching: Soundex algorithm integrated into name matching
- Data quality: validation rules, phone normalization, address standardization
- Privacy: data masking, GDPR export, consent model

### Phase 14: Compilation Fixes & Test Expansion

- Fixed 60 compilation errors (missing .await on async calls, type references, imports)
- Added comprehensive unit tests across all modules
- Added Criterion benchmark tests (matching, search, validation)
- Updated all AGENTS documentation files

## Current Status

- 60+ unit tests passing
- 7 integration tests (require PostgreSQL)
- 3 benchmark suites (matching, search, validation)
- 0 compilation errors, 0 warnings
- Build: cargo check passes clean

## Next Phases (Planned)

### Phase 15: Authentication & Authorization

- JWT-based authentication
- Role-based access control (RBAC)
- Rate limiting

### Phase 16: Observability & Monitoring

- Prometheus metrics integration
- Distributed tracing with OpenTelemetry
- Custom dashboards and alerting

### Phase 17: Performance Optimization

- Database query caching
- Load testing at scale (millions of patients)
- Query optimization

### Phase 18: Infrastructure as Code (OpenTofu)

- Multi-cloud deployment modules (GCP, AWS, Azure)
- VPC, load balancers, security groups

### Phase 19: Kubernetes

- Helm charts
- Horizontal pod autoscaling
- Persistent volume claims

### Phase 20: Production Readiness

- Security review and penetration testing
- Disaster recovery drills
- HIPAA/GDPR compliance validation

### Phase 21: Continuous Improvement

- ML-based match scoring
- A/B testing for matching algorithms
- CI/CD pipeline improvements
