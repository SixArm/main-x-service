## 15. Roadmap

The path from today's single-node MVP trio (§14) to a worldwide
public governmental deployment (§1, §7). Ordered roughly by
dependency; crate-internal items link to the owning roadmap.

### 15.1 Security first

- **JWT enforcement** on `/api/*` (service §13 T-1) with RS256
  verification against the [authentication entity's](../../authentication/)
  JWKS, then trio-wide SSO wiring (§13 T-4): roles for HR-admin,
  credentialing-officer, read-only, and service callers; rate
  limiting; security headers.
- Security audit + penetration test before any production population
  data (service §15).

### 15.2 Durable, observable backbone

- **Durable event bus** replacing the in-memory publisher: Fluvio
  production publisher (service §13 T-2), with Kafka/NATS evaluated
  for government-infrastructure fit; event consumers for downstream
  agencies.
- Complete OTLP trace export, custom metrics
  (`credential_expiry_within_30d`, …), Grafana dashboards + alerting.

### 15.3 Population scale

- **Multi-region replication**: PostgreSQL cross-region replication +
  read replicas; Kubernetes (Helm, HPA, probes) per service roadmap;
  multi-region failover runbooks, backup + DR.
- **Externalised search**: today's local-disk Tantivy index binds the
  service to one node; move to a replicated / shared index tier so
  the app tier stays stateless.
- **Bulk import pipelines** from licensing authorities (NPI / DEA
  registry imports per service §15), with batch dedup as the intake
  gate.
- Load testing at realistic national-workforce volumes; query caching
  and N+1 fixes.

### 15.4 Workforce-domain depth

- **Credential-verification integrations**: employer/verifier-facing
  checks against issuing authorities; credential-expiry warning
  workflow (service §13 T-7); role + assignment history timeline
  (service §13 T-8).
- Complete FHIR (capability statement, bundles, Organization) and
  gRPC for high-throughput agency callers.
- Consent **enforcement** (model exists; checks are not wired into
  read paths).

### 15.5 Operator experience worldwide

- **Localisation** of the front-end across the
  [locale catalogue](../../agents/share/locales.md); RTL support
  (ar, fa, ur); locale-aware date / identifier formatting.
- **Accessibility**: replace styled native controls with Lily
  Headless primitives (front-end T-14) and audit to WCAG 2.2 AA.
- Privacy UI (§13 T-5), sub-record editing (front-end T-15), batch
  dedup results UI (front-end T-18).
- Resolve SVAR DataGrid licensing for government procurement
  (front-end §16 OQ-1).
