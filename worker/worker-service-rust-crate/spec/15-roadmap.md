## 15. Roadmap

- **Authentication & authorisation** — JWT, RBAC for HR-admin /
  credentialing-officer / service roles, rate limiting, user
  endpoints, security headers.
- **Observability** — Prometheus alongside OTLP, complete OTLP trace
  exporter, custom metrics (`worker_created`,
  `credential_expiry_within_30d`, …), Grafana dashboards + alerting.
- **Performance** — query caching, N+1 batch fixes, load test at
  realistic workforce volumes.
- **Infrastructure as code** — OpenTofu modules, multi-cloud, secrets,
  backup + DR.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index,
  ingress, probes.
- **Production readiness** — security audit + pen test, HIPAA + GDPR
  validation, DR runbook, backup / restore, CI/CD pipeline.
- **Feature enhancements** — complete gRPC, complete FHIR (capability
  statement, bundles, Organization), Fluvio production + consumers,
  ML-based match scoring, worker photo storage, consent enforcement,
  **credential-expiry-warning workflow**, **role + assignment history
  timeline**, NPI / DEA registry import pipelines.

