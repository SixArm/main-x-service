# Main Event Index (MPI) - Tasks

## Completed

- [x] Fix 60 compilation errors (missing .await, type refs, Expr import)
- [x] Fix utoipa-swagger-ui build cache corruption
- [x] Add comprehensive unit tests across all modules
- [x] Add Criterion benchmark tests (matching, search, validation)
- [x] Update AGENTS/index.md with complete file listing
- [x] Fill in AGENTS/matching.md with algorithm details
- [x] Fill in AGENTS/models.md with domain model reference
- [x] Update AGENTS/restful.md with complete endpoint docs
- [x] Fill in AGENTS/testing.md with test strategy and coverage
- [x] Update AGENTS/architecture.md with current architecture
- [x] Update plan.md with current status and Phase 14
- [x] Create tasks.md

## In Progress

- [ ] Update CLAUDE.md test counts and coverage numbers

## Backlog

### Phase 15: Authentication & Authorization

- [ ] Add JWT-based authentication middleware
- [ ] Implement role-based access control (RBAC)
- [ ] Add rate limiting middleware
- [ ] Add authentication to all protected endpoints
- [ ] Add user management endpoints

### Phase 16: Observability & Monitoring

- [ ] Integrate Prometheus metrics exporter
- [ ] Complete OpenTelemetry OTLP trace exporter
- [ ] Add custom MPI metrics (event_created, match_score, etc.)
- [ ] Create Grafana dashboard templates

### Phase 17: Performance Optimization

- [ ] Add database query caching (Redis or in-memory)
- [ ] Optimize N+1 queries in repository (batch loading)
- [ ] Load test with realistic data volumes
- [ ] Profile and optimize matching algorithms

### Phase 18: Infrastructure as Code

- [ ] OpenTofu modules for PostgreSQL provisioning
- [ ] OpenTofu modules for application deployment
- [ ] Multi-cloud configuration (GCP, AWS, Azure)
- [ ] Secrets management integration

### Phase 19: Kubernetes

- [ ] Create Helm chart
- [ ] Configure horizontal pod autoscaling
- [ ] Set up persistent volume claims for search index
- [ ] Add Kubernetes health check probes

### Phase 20: Production Readiness

- [ ] Security audit and penetration testing
- [ ] HIPAA compliance validation
- [ ] GDPR compliance validation
- [ ] Disaster recovery runbook and drills
- [ ] Backup and restore procedures

### Phase 21: Feature Enhancements

- [ ] Complete gRPC API implementation
- [ ] Complete FHIR R5 full compliance
- [ ] Implement FluvioProducer for production event streaming
- [ ] Add ML-based match scoring
- [ ] Web UI with Tera templates, HTMX, Alpine.js
- [ ] Event photo storage and retrieval
- [ ] Consent enforcement in query layer
