## 15. Roadmap

- **Authentication & authorisation** — T-1a and T-1b are complete:
  peer PASETO v4.public verification via the `authentication-verifier`
  crate (T-1a, per
  [authentication-sessions](../../../agents/share/authentication-sessions.md)),
  the default-off `WORKER_REQUIRE_AUTH` blanket `/api/*` enforcement
  middleware and the boot-time key-set fetch from
  `/.well-known/paseto-keys` behind `WORKER_PASETO_KEYS_URL` (both
  2026-07-04), and **ABAC authorization** (2026-07-05, per
  [authorization-attributes](../../../agents/share/authorization-attributes.md)
  — the shared policy engine over the token's `attrs` claim; supersedes
  the earlier RBAC sketch for HR-admin / credentialing-officer /
  service roles). Remaining: operational activation (set the flag once
  the SSO token flow is live); richer deployment policies are
  configuration (`WORKER_ABAC_POLICY*`), not code; record-level
  resource attributes are a shared-design open question; periodic
  key-set refresh / refetch on `UnknownKid` (today the fetch is once
  at boot only), rate limiting, user endpoints, security headers.
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
- **Feature enhancements** — extend the gRPC surface landed 2026-09-02
  (T-6: `UpdateWorker`, match/merge/search/assessments/FHIR over gRPC,
  the remaining domain fields on the proto `Worker` message, disclosure
  accounting + per-record masking parity with REST — see T-6's own
  record for the full list), complete FHIR (typed
  `Bundle`/`POST`-bundle support — the CapabilityStatement and ad hoc
  searchset Bundle already ship — plus an Organization resource),
  Fluvio production + consumers,
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

