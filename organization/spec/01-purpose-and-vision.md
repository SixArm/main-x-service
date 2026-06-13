## 1. Purpose and Vision

### 1.1 Purpose

The **organization entity** is the organization-identity registry of
the Main X Index — a federated identity index serving a worldwide
public governmental system with millions of users. It models
[schema.org/Organization](https://schema.org/Organization) and is
delivered as a trio of subprojects that compose into one capability:

| Subproject | Role |
|---|---|
| [organization-service-rust-crate](../organization-service-rust-crate/) | Registry service — loco.rs CRUD, name search, matching, audit log, event streaming, OpenAPI/Swagger over REST |
| [organization-matcher-rust-crate](../organization-matcher-rust-crate/) | Canonical pairwise matching library — deterministic identifier short-circuits + explainable probabilistic scoring, embedded by the service |
| [organization-front-end-with-svelte](../organization-front-end-with-svelte/) | Operator UI — SvelteKit SPA for CRUD + duplicate-check over the service's REST API |

The entity gives government agencies, company registers, and
procurement systems one canonical record per real-world organization —
companies, public agencies, NGOs, public bodies — regardless of how
many departmental source systems hold a shard of that identity.

### 1.2 Vision

One canonical organization record per real-world organization, at
national / international register scale:

- **Register-scale.** Millions of organization records, on the order
  of national company registers and the GLEIF LEI universe; sustained
  ingestion from many authoritative register feeds (roadmap, §15).
- **Register-grade identifiers.** Deterministic linkage on the
  globally unique schemes carried by real registers — LEI, DUNS,
  ISO 6523, GLN, Wikidata, ROR, ISNI, VAT — plus jurisdiction-scoped
  tax IDs and `same_as` URLs.
- **Multi-jurisdiction by design.** Legal-suffix-aware name matching
  (`"Acme, Inc."` ≡ `"ACME"`, `GmbH`, `Ltd`, …), diacritic-correct
  normalisation (`Müller` ≠ `Muller`), ISO 3166 jurisdictions; the
  operator surface localizes to the locales in
  [`agents/share/locales.md`](../../agents/share/locales.md) (roadmap).
- **Explainable matching.** Every match decision returns a
  per-component score breakdown that an operator, auditor, or court
  can inspect — no black boxes.
- **Audit-grade.** Every CRUD operation writes an audit-log row and
  publishes an event, suitable for GDPR / UK DPA accountability and
  ISO 27001 evidence.
- **One DTO, zero drift.** The matcher's `Organization` type is the
  API body, the stored payload, and the scoring input — there is no
  adapter layer to drift.

### 1.3 Non-goals

- **Not** a company-filings system. Annual accounts, returns,
  beneficial-ownership filings, and statutory documents stay in the
  authoritative registers; this entity links to them via identifiers.
- **Not** an org-chart or HR system. Internal structure, departments,
  employees, and reporting lines are out of scope (the matcher
  explicitly excludes `parentOrganization` / `subOrganization`
  traversal — matcher [spec §2](../organization-matcher-rust-crate/spec/index.md)).
- **Not** an authentication / authorisation provider. Sign-on for the
  whole index is the [authentication entity](../../authentication/)
  (passwordless magic-link, RS256 JWT + JWKS); this entity will be a
  JWT *verifier* (roadmap, §15).
- **Not** a credit-rating, risk-scoring, or due-diligence platform;
  it resolves identity, it does not assess organizations.
