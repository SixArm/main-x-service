## 15. Roadmap

Roadmap items become §13 tasks when they are concrete enough to size
and accept.

- **Authentication & authorisation** — blanket PASETO enforcement on
  `/api/*` landed as T-1b (default-off behind `PERSON_REQUIRE_AUTH`;
  peer PASETO v4.public verification landed as T-1a; boot-time key-set
  fetch behind `PERSON_PASETO_KEYS_URL` landed as the T-1c fetch item;
  **ABAC authorization** landed 2026-07-05 as the T-1c authorization
  item, per
  [authorization-attributes](../../../agents/share/authorization-attributes.md)
  — the shared policy engine over the token's `attrs` claim;
  supersedes the earlier RBAC-on-`roles`/`scope` sketch). Still open:
  the DB-gated request test (T-1c), periodic key-set refresh / refetch
  on `UnknownKid` (today the fetch is once at boot only), operational
  activation of the flag once the SSO token flow is live (richer
  deployment policies are configuration — `PERSON_ABAC_POLICY*` — not
  code; record-level resource attributes are a shared-design open
  question), rate limiting, user endpoints, security headers.
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

