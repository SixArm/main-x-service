## 15. Roadmap

The path from today's single-node trio to a worldwide public
governmental deployment. Items become §13 tasks (here or in a
subproject spec) when concrete enough to size and accept. Ordered
roughly by dependency.

### 15.1 Security first

- **Blanket auth enforcement** on every service endpoint — PASETO
  v4.public bearer tokens verified offline against the
  [authentication entity](../../authentication/)'s published key set,
  with ABAC policy authorization inside the guard (E-1; per
  [jwt-enforcement](../../agents/share/jwt-enforcement.md) and
  [authorization-attributes](../../agents/share/authorization-attributes.md)).
  Activation (`PERSON_REQUIRE_AUTH`) is an operations decision; rate
  limiting follows.
- Security audit + penetration test before any public-sector pilot.

### 15.2 Durable integration

- **Durable event bus** (Fluvio first per service T-2; evaluate
  Kafka / NATS for agency-scale fan-out) with consumers, replay, and
  documented broker-failover behaviour (E-3).
- **Bulk import** pipeline for onboarding agency source systems:
  validated batch ingest, dedup-on-ingest, per-batch audit manifest.
- **Complete gRPC** for high-throughput machine-to-machine callers
  (service T-6).

### 15.3 Population scale

- **Multi-region deployment**: Kubernetes/Helm, HPA, per-region
  replicas; PostgreSQL cross-region replication with managed
  failover; backup / DR runbook + drills.
- **Externalized search**: move the Tantivy index off per-instance
  local disk so replicas share one consistent view; re-evaluate
  engine choice at hundreds of millions of records.
- **Load testing** at realistic population volumes; profile matching
  hot paths; query caching.

### 15.4 Interoperability maturity

- **FHIR maturity**: capability statement, bundle handling,
  Organization resource (service T-3, T-4, §16 OQ-1).
- Adapter scheme-routing coverage for all 42 matcher identifier
  schemes (E-8).

### 15.5 Operator experience

- **Localization** of the front-end across the
  [`agents/share/locales.md`](../../agents/share/locales.md) set,
  starting with one non-English locale (E-7); RTL support for ar /
  fa / ur.
- **Accessibility**: full WCAG 2.2 AA audit; deepen Lily Headless
  usage (front-end T-14).
- Consent UI (E-5), masked view + GDPR-export download (E-4),
  batch-dedup results UI (front-end T-18), sub-record editing
  (front-end T-15).
- Resolve SVAR licensing for public deployment (E-6).

### 15.6 Governance

- Spec-drift CI across the entity (service T-7 generalized): fail a
  PR that changes an integration seam without editing this spec.
- ISO 27001 ISMS evidence pack; ISO 42001 AIMS controls if/when
  match scoring becomes ML-assisted (service roadmap lists ML scoring
  with A/B testing — it MUST NOT land before explainability parity).
