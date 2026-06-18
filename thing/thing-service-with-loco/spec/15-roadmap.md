## 15. Roadmap

- **Authentication & authorisation** — JWT, RBAC, rate limiting,
  user endpoints, security headers.
- **Observability** — Prometheus alongside OTLP, complete OTLP trace
  exporter, custom metrics (`thing_created`, `match_score_histogram`),
  Grafana dashboards + alerting.
- **Performance** — query caching, batch fixes, profile matching hot
  paths.
- **Infrastructure as code** — OpenTofu modules, multi-cloud, secrets,
  backup + DR.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index,
  ingress, probes.
- **Production readiness** — security audit, GDPR validation, DR
  runbook, backup / restore, CI/CD pipeline.
- **Feature enhancements** — complete gRPC, Fluvio production +
  consumers, ML / embedding-based match scoring, image storage and
  retrieval, schema.org sub-type registry (`additional_type` lookup),
  Wikidata + OpenLibrary import pipelines.

