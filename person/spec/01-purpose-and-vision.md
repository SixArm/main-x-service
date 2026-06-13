## 1. Purpose and Vision

### 1.1 Purpose

The **person entity** is the general-purpose person-identity registry
of the Main X Index — a federated identity index serving a worldwide
public governmental system with millions of users. It is delivered as
a trio of subprojects that compose into one capability:

| Subproject | Role |
|---|---|
| [person-service-rust-crate](../person-service-rust-crate/) | Registry service — CRUD, search, matching, merge, audit, privacy over REST + FHIR R5 |
| [person-matcher-rust-crate](../person-matcher-rust-crate/) | Canonical pairwise matching library — deterministic + probabilistic, embedded by the service |
| [person-front-end-with-svelte](../person-front-end-with-svelte/) | Operator UI — SvelteKit SPA over the service's REST API |

The entity gives government agencies one canonical record per
real-world person regardless of how many departmental source systems
hold a shard of that identity, with the structured fields (tax ID,
identity documents, emergency contacts, multinational national
identifiers) a population-scale civil registry needs.

### 1.2 Vision

One canonical person record per real-world person, at population
scale:

- **Population-scale.** Hundreds of millions of person records,
  millions of operator and machine users, sustained ingestion from
  many government source systems.
- **Multinational by design.** The embedded matcher carries 42
  national personal-identifier schemes plus passport-book matching;
  the operator surface localizes to the locales in
  [`agents/share/locales.md`](../../agents/share/locales.md).
- **Explainable matching.** Every match decision returns a
  per-component score breakdown that an operator, auditor, or court
  can inspect — no black boxes.
- **Audit-grade.** Every CRUD / merge / link operation writes an
  audit-log row and publishes an event, suitable for HIPAA-grade
  trails where healthcare-relevant and GDPR / UK DPA accountability
  everywhere.
- **Operator-complete.** The front-end covers the full duplicate
  lifecycle — surface candidates on create, score hypotheticals,
  review, merge, and audit — so registry stewardship needs no SQL.

### 1.3 Non-goals

- **Not** an authentication / authorisation provider. Sign-on for the
  whole index is the [authentication entity](../../authentication/)
  (passwordless magic-link, RS256 JWT + JWKS); the person entity is a
  JWT *verifier* (roadmap, §15).
- **Not** a system of record for domain-specific data (encounters,
  benefits, transactions, conditions) — departments keep their own
  systems and link to the person registry via `person_id`.
- **Not** a workforce credentialing registry — that is the
  [worker entity](../../worker/).
- **Not** an analytics or population-statistics platform; it exposes
  data for export, it does not aggregate it.
