## 15. Roadmap

- **Authentication & authorisation** — blanket PASETO enforcement on
  `/api/*` (T-1b; peer PASETO v4.public verification via the
  `authentication-verifier` crate landed as T-1a, per
  [authentication-sessions](../../../agents/share/authentication-sessions.md)),
  boot-time key-set fetch from `/.well-known/paseto-keys`, RBAC for
  HR-admin / credentialing-officer / service roles, rate limiting,
  user endpoints, security headers.
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
- **NHS ODS organization expansion** — align the embedded
  `Organization` model with the NHS Organisation Data Service: ODS-style
  fields (`ods_code`, status, record class, assigning authority,
  operative date periods) plus new domain models for
  `OrganizationRole`, `OrganizationRelationship`,
  `OrganizationSuccession`, `PostcodeGeography`, and ODS `CodeSystem`
  reference data — each with a domain model, migration, SeaORM entity,
  and tests. (Detailed task breakdown was folded here from a former
  `docs/superpowers/plans/2026-03-22-nhs-ods-organizations*.md`
  implementation plan, now removed.)

