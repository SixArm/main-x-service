## 15. Roadmap

- **Authentication & authorisation** — T-4 is complete: peer offline
  PASETO v4.public verification, the default-off `THING_REQUIRE_AUTH`
  blanket `/api/*` enforcement middleware, the boot-time
  `THING_PASETO_KEYS_URL` HTTP key-set fetch (all 2026-07-04), and
  **ABAC authorization** (2026-07-05, per
  [authorization-attributes](../../../agents/share/authorization-attributes.md)
  — the shared policy engine over the token's `attrs` claim;
  supersedes the earlier roles/RBAC sketch). Remaining: activation is
  an ops decision; richer deployment policies are configuration
  (`THING_ABAC_POLICY*`), not code; record-level resource attributes
  are a shared-design open question. Also: a periodic key-set
  **re-fetch/refresh loop** for key rotation (today the fetch happens
  once at boot), rate limiting, user endpoints, security headers.
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

