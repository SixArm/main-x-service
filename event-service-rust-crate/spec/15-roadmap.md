## 15. Roadmap

- **Authentication & authorisation** — JWT, RBAC for scheduler /
  admin / read-only / service, rate limiting, security headers.
- **Observability** — Prometheus alongside OTLP, complete OTLP trace
  exporter, custom metrics (`event_created`, `event_duration_seconds`,
  `match_score`), Grafana dashboards + alerting.
- **Performance** — time-range query caching, btree_gist exclusion
  constraints for no-overlap policies, load test at realistic event
  volumes.
- **Infrastructure as code** — OpenTofu modules, multi-cloud, secrets,
  backup + DR.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index,
  ingress, probes.
- **Production readiness** — security audit + pen test, GDPR
  validation, DR runbook, backup / restore, CI/CD pipeline.
- **Feature enhancements** — complete gRPC; complete FHIR (capability
  statement, bundles, Encounter / Appointment); Fluvio production +
  consumers; ML-based match scoring; iCalendar import / export; RFC
  5545 RRULE recurrence; time-zone-aware fuzzy matching; consent
  enforcement in the query layer.

