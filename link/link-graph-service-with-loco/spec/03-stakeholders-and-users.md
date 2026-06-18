## 3. Stakeholders and Users

### 3.1 Direct consumers

- **Operators / case workers** — query a record's neighbours and the
  "single view" of one real-world identity to understand how a person,
  worker, organization, or case connect across the federated registries.
  For `case ↔ person` edges this is governed access (§12).
- **Front-end UIs** — the per-entity `*-front-end-with-svelte`
  projects may surface a "linked records" panel and a single-view
  walk. They consume the read API and display the `as_of` watermark so
  the operator sees graph freshness.
- **Peer index services** — a service can call `GET /neighbors` to fan
  out (e.g. resolve a person's employer via `person → worker → org`)
  without each service traversing the others directly.
- **Analytics / data-quality teams** — consume the reconciliation
  divergence metric and freshness lag as health signals.

### 3.2 Upstream producers (the bus)

Every entity service that emits to the durable event bus is an upstream
producer this service consumes:

- **person-service**, **worker-service** — the federation backbone
  (`same_identity`).
- **organization-service** — affiliation targets (`works_at`,
  `member_of`, `employed_by`).
- **case-service** — high-governance `subject_of` / `about` edges (§12).
- **place / thing / event / course / care-pathway services** — their
  `created` / `deleted` events feed `entity_presence` even before any
  edge kind references them, so future kinds verify immediately.

### 3.3 Operators of this service

- **Platform / SRE** — run the bus consumers, the reconciliation
  worker, and the freshness/divergence dashboards; own the SLOs in §7.

### 3.4 Governance / compliance

- **Information governance** — owns the `case ↔ person` access-control,
  audit, and masking rules (§12), inherited from the case service's
  compliance posture.
