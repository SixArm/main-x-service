# Main Event Index (MPI) - Task Tracker

## Phase 1: Project Setup & Foundation

- [x] 1.1 Initialize Rust project with Cargo
- [x] 1.2 Configure Cargo.toml with all required dependencies
- [x] 1.3 Set up project structure (modules, directories)
- [x] 1.4 Configure Diesel for PostgreSQL
- [x] 1.5 Set up OpenTelemetry framework
- [x] 1.6 Create README.md with project overview

## Phase 2: Database Schema & Models

- [x] 2.1 Design PostgreSQL schema for event records (13 tables)
- [x] 2.2 Design schema for clinic/organization data
- [x] 2.3 Design event identifier cross-reference tables
- [x] 2.4 Create Diesel migrations (5 migration sets, 365 lines SQL)
- [x] 2.5 Implement Rust models/structs for event entities (27 models)
- [x] 2.6 Implement models for clinic/organization entities
- [x] 2.7 Add database indexes for performance (40+ indexes)
- [x] 2.8 Implement soft delete and audit trail capabilities

## Phase 3: Core MPI Logic

- [x] 3.1 Implement event matching algorithms
  - [x] Name matching (Jaro-Winkler, Levenshtein, Soundex phonetic)
  - [x] Date of birth matching (with typo tolerance)
  - [x] Gender matching
  - [x] Address matching (postal code, city, state, street)
  - [x] Identifier matching (MRN, SSN, DL, NPI, PPN, TAX)
  - [x] Tax ID exact match (deterministic)
  - [x] Document number match
- [x] 3.2 Implement probabilistic matching scoring
- [x] 3.3 Implement deterministic matching rules
- [x] 3.4 Create event merge functionality
- [x] 3.5 Create event link/unlink functionality
- [x] 3.6 Implement event search functionality
- [x] 3.7 Add duplicate detection (real-time + batch)
- [x] 3.8 Implement event identifier management

## Phase 4: Search Engine Integration

- [x] 4.1 Set up Tantivy search index (11 fields)
- [x] 4.2 Implement event data indexing (single + bulk)
- [x] 4.3 Create search query builders
- [x] 4.4 Implement fuzzy search capabilities
- [x] 4.5 Implement search by name and birth year (blocking)
- [x] 4.6 Implement incremental index updates
- [x] 4.7 Add index optimization and reload

## Phase 5: RESTful API (Axum)

- [x] 5.1 Set up Axum web framework with CORS
- [x] 5.2 Implement event CRUD endpoints (POST, GET, PUT, DELETE)
- [x] 5.3 Implement search endpoint (with pagination, fuzzy, masking)
- [x] 5.4 Implement match endpoint
- [x] 5.5 Implement duplicate check endpoint
- [x] 5.6 Implement merge endpoint
- [x] 5.7 Implement batch deduplication endpoint
- [x] 5.8 Implement GDPR export endpoint
- [x] 5.9 Implement masked event view endpoint
- [x] 5.10 Implement audit log endpoints (event, recent, user)
- [x] 5.11 Add request validation (422 on invalid input)
- [x] 5.12 Implement error handling and response formatting

## Phase 6: HL7 FHIR R5 Support

- [x] 6.1 Implement FHIR Event resource mapping
- [x] 6.2 Create FHIR-compliant REST endpoints (foundation)
- [x] 6.3 Implement FHIR search parameters
- [x] 6.4 Implement bidirectional conversion (to/from FHIR)
- [ ] 6.5 Add FHIR capability statement
- [ ] 6.6 Implement FHIR bundle support (full)
- [ ] 6.7 Add FHIR Organization resource mapping

## Phase 7: Database Integration

- [x] 7.1 Implement DieselEventRepository (full CRUD)
- [x] 7.2 Transaction support for complex operations
- [x] 7.3 Bidirectional domain/DB model conversion
- [x] 7.4 Soft delete with timestamp tracking
- [x] 7.5 Paginated active event retrieval

## Phase 8: Event Streaming & Audit Logging

- [x] 8.1 Implement InMemoryEventPublisher
- [x] 8.2 Event types: Created, Updated, Deleted, Merged, Linked, Unlinked
- [x] 8.3 AuditLogRepository with CREATE/UPDATE/DELETE logging
- [x] 8.4 Audit query methods (entity, recent, user)
- [x] 8.5 Repository integration (auto event + audit on CRUD)
- [ ] 8.6 Implement Fluvio event streaming
- [ ] 8.7 Implement event consumers

## Phase 9: OpenAPI Documentation

- [x] 9.1 Add Utoipa annotations to all REST endpoints
- [x] 9.2 Generate OpenAPI v3 specification
- [x] 9.3 Create Swagger UI integration
- [x] 9.4 Schema components for all models

## Phase 10: Integration Testing

- [x] 10.1 Test infrastructure with common utilities
- [x] 10.2 Health check test
- [x] 10.3 Event CRUD tests (create, get, update, delete)
- [x] 10.4 Search integration test
- [x] 10.5 Error handling tests (404)
- [ ] 10.6 Dedup/merge integration tests
- [ ] 10.7 Privacy endpoint integration tests

## Phase 11: Docker & Deployment

- [x] 11.1 Multi-stage Dockerfile (85% smaller images)
- [x] 11.2 Docker Compose for development
- [x] 11.3 Docker Compose for testing
- [x] 11.4 Production environment config template
- [x] 11.5 DEPLOY.md deployment guide

## Phase 12: Documentation

- [x] 12.1 Comprehensive README.md
- [x] 12.2 Architecture documentation
- [x] 12.3 API usage examples
- [x] 12.4 Database schema documentation

## Phase 13: Advanced MPI Features

- [x] 13.1 Event identity: tax_id field, identity documents, emergency contacts
- [x] 13.2 Duplicate detection: real-time on create (409), explicit check endpoint
- [x] 13.3 Record merging: data transfer, links, soft-delete duplicate
- [x] 13.4 Batch deduplication: pairwise scan, review queue, auto-merge
- [x] 13.5 Phonetic matching: Soundex integrated into name matching
- [x] 13.6 Data quality: validation, phone normalization, address standardization
- [x] 13.7 Privacy: data masking, GDPR export, consent model
- [x] 13.8 Score breakdown in API responses (tax_id_score, document_score)

## Phase 14: Authentication & Authorization (Planned)

- [ ] 14.1 JWT-based authentication
- [ ] 14.2 Role-based access control (RBAC)
- [ ] 14.3 Rate limiting and request throttling
- [ ] 14.4 Security headers

## Phase 15: gRPC API (Planned)

- [ ] 15.1 Define Protocol Buffer schemas
- [ ] 15.2 Implement gRPC server
- [ ] 15.3 Event service RPC methods
- [ ] 15.4 Search service RPC methods
- [ ] 15.5 gRPC health checks

## Phase 16: Observability & Monitoring (Planned)

- [ ] 16.1 Prometheus metrics
- [ ] 16.2 Distributed tracing with OpenTelemetry
- [ ] 16.3 Custom dashboards
- [ ] 16.4 Alerting rules

## Phase 17: Performance Optimization (Planned)

- [ ] 17.1 Benchmark tests with Criterion
- [ ] 17.2 Database query caching
- [ ] 17.3 Load testing at scale
- [ ] 17.4 Memory and connection pool optimization

## Phase 18: Infrastructure as Code (Planned)

- [ ] 18.1 OpenTofu modules
- [ ] 18.2 Multi-cloud deployment (GCP, AWS, Azure)
- [ ] 18.3 VPC, load balancers, security groups
- [ ] 18.4 Backup and disaster recovery

## Phase 19: Kubernetes (Planned)

- [ ] 19.1 Helm charts
- [ ] 19.2 Horizontal pod autoscaling
- [ ] 19.3 Persistent volume claims
- [ ] 19.4 Ingress controllers

## Phase 20: Production Readiness (Planned)

- [ ] 20.1 Security review and penetration testing
- [ ] 20.2 Disaster recovery drills
- [ ] 20.3 HIPAA/GDPR compliance validation
- [ ] 20.4 Incident response procedures
- [ ] 20.5 CI/CD pipeline

---

## Summary

- **Completed**: Phases 1-5, 7-13 (core infrastructure through advanced features)
- **Partially complete**: Phase 6 (FHIR), Phase 8 (streaming), Phase 10 (testing)
- **Planned**: Phases 14-20 (auth, gRPC, observability, performance, infra, k8s, prod readiness)
- **Unit tests**: 34 passing
- **Integration tests**: 7 (require PostgreSQL)
- **API endpoints**: 15
