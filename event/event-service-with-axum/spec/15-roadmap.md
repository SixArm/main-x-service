## 15. Roadmap

- **Authentication & authorisation** — T-8 is complete: peer offline
  PASETO v4.public verification via the `authentication-verifier`
  crate, the default-off `EVENT_REQUIRE_AUTH` blanket `/api/*`
  enforcement middleware, the boot-time `EVENT_PASETO_KEYS_URL` HTTP
  key-set fetch (all 2026-07-04, per
  [authentication-sessions](../../../agents/share/authentication-sessions.md)),
  and **ABAC authorization** (2026-07-05, per
  [authorization-attributes](../../../agents/share/authorization-attributes.md)
  — the shared policy engine over the token's `attrs` claim;
  supersedes the earlier RBAC scheduler / admin / read-only / service
  roles sketch). Remaining: activation is an operations decision;
  richer deployment policies are configuration (`EVENT_ABAC_POLICY*`),
  not code; record-level resource attributes are a shared-design open
  question. Also: a periodic key-set **re-fetch/refresh loop** for key
  rotation (today the fetch happens once at boot), rate limiting,
  security headers.
- **Observability** — Prometheus is done (`GET /metrics.prom`,
  `src/metrics.rs`); remaining: the OTLP exporter is still a `TODO`
  stub (`src/observability/mod.rs` builds an OTel `Resource` and
  installs a plain JSON `tracing` subscriber, but exports no span or
  metric over OTLP today), custom domain metrics (`event_created`,
  `event_duration_seconds`, `match_score`), Grafana dashboards +
  alerting.
- **Performance** — time-range query caching, btree_gist exclusion
  constraints for no-overlap policies, load test at realistic event
  volumes.
- **Infrastructure as code** — OpenTofu modules, multi-cloud, secrets,
  backup + DR.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index,
  ingress, probes.
- **Production readiness** — security audit + pen test, GDPR
  validation, DR runbook, backup / restore, CI/CD pipeline.
- **Feature enhancements** — extend the gRPC surface landed 2026-09-02
  (T-6: `UpdateEvent`, match/merge/search/FHIR over gRPC, the
  remaining schema.org/Event fields on the proto `Event` message — see
  T-6's own record for the full list); the `Appointment`
  `CapabilityStatement` and the ad hoc searchset `Bundle` already ship
  via T-10; a FHIR `Encounter` mapping alongside `Appointment`; bulk
  import/export (T-9); Fluvio consumers (the production sink itself,
  `FluvioSink`, shipped via T-11/BUS-3 — what remains is a deployment
  actually pointing `EVENT_FLUVIO_ENDPOINT` at a live broker, and the
  link-graph aggregator's own consumer, tracked as BUS-2 elsewhere);
  ML-based match scoring; iCalendar import / export; RFC 5545 RRULE
  recurrence; time-zone-aware fuzzy matching; consent enforcement in
  the query layer.

