## 15. Roadmap

- **Authentication & authorisation** — roles on top of the delivered
  blanket PASETO enforcement (env-gated `THING_REQUIRE_AUTH`,
  default-off middleware landed 2026-07-04; peer offline PASETO
  v4.public verification via the `authentication-verifier` crate and
  the boot-time `THING_PASETO_KEYS_URL` HTTP key-set fetch also
  2026-07-04, per
  [authentication-sessions](../../../agents/share/authentication-sessions.md)),
  activation as an ops decision, a periodic key-set
  **re-fetch/refresh loop** for key rotation (today the fetch happens
  once at boot), RBAC, rate limiting, user endpoints, security
  headers.
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

