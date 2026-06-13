## 2. Scope

### 2.1 In scope (entity level)

This spec owns the **cross-subproject contract**:

- How the trio composes: front-end → service REST API → embedded
  matcher (dependency direction is one-way).
- The DTO contract between service and matcher
  ([§5.3](05-domain-model.md)).
- Shared invariants every subproject must honour (GLN check digit,
  coordinate bounds, soft-delete-only deletion, no PII in logs).
- Entity-wide goals: governmental scale, locales, compliance,
  availability ([§7](07-non-functional-requirements.md), [§12](12-compliance.md)).

### 2.2 What each subproject owns

| Concern | Owner |
|---|---|
| Place CRUD, soft delete, audit trail, event streaming | [place-service](../place-service-rust-crate/spec/index.md) |
| PostgreSQL schema (13 tables), SeaORM entities, migrations | place-service |
| Tantivy full-text / fuzzy / boolean search; geo-radius search | place-service |
| Duplicate detection (real-time / explicit / batch), review queue, merge | place-service |
| Validation, normalisation, privacy masking, GDPR export, consent | place-service |
| REST API surface, OpenAPI / Swagger, response envelope | place-service |
| Canonical pairwise matching algorithm (deterministic + probabilistic) | [place-matcher](../place-matcher-rust-crate/spec/index.md) |
| Normalisation primitives (names, postcodes, phones, addresses, phonetics) | place-matcher |
| Match configuration presets (`strict` / `default` / `lenient`) | place-matcher |
| Operator UI: routes, forms, data grid, API client, wire types | [place-front-end](../place-front-end-with-svelte/spec/index.md) |
| The service→matcher adapter (`src/matching/adapter.rs`) **routing rules** | place-service (pinned by this spec's §5.3 contract) |

### 2.3 Out of scope (today)

- FHIR R5 — places are not a FHIR-resource concern (service spec §9).
- A shared front-end package — drift between front-ends is accepted
  per repo decision (2026-06-02).
- Tile serving, routing, map rendering ([§1.3](01-purpose-and-vision.md)).
- Candidate blocking inside the matcher — pre-filtering is the
  service's concern (matcher spec §1.2).
- Cross-entity matching (place ↔ organization site reconciliation) —
  see [§16](16-open-questions.md).
