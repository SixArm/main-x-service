## 15. Roadmap

Roadmap items become §13 tasks when they are concrete enough to size
and accept.

- **Authentication & authorisation** — blanket PASETO enforcement on
  `/api/*` (T-1b; peer PASETO v4.public verification landed as T-1a),
  RBAC, rate limiting, user endpoints, security headers.
- **Observability** — Prometheus alongside OTLP, complete OTLP trace
  exporter, custom metrics (`person_created`, `match_score_histogram`,
  …), Grafana dashboards + alerting.
- **Performance** — query caching (in-memory), N+1 batch
  fixes, load test at realistic person volumes, profile matching hot
  paths.
- **Infrastructure as code** — OpenTofu modules (PostgreSQL + app
  deploy), multi-cloud (GCP, AWS, Azure), secrets management, backup
  and DR automation.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index, ingress
  controllers, Kubernetes health probes.
- **Production readiness** — security audit + pen test, HIPAA + GDPR
  validation, DR runbook + drills, backup / restore, incident
  response, CI/CD pipeline.
- **Feature enhancements** — complete gRPC, complete FHIR (capability
  statement, bundles, Organization), Fluvio production publisher +
  consumers, ML-based match scoring with A/B test framework, person
  photo storage and retrieval, consent enforcement in the query layer.

