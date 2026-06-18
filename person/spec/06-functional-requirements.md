## 6. Functional Requirements

Entity-level requirements, each mapped to its owning subproject.
Detail lives in the owner's spec: the entry here is the contract that
the trio composes correctly. Owners: **S** = person-service, **M** =
person-matcher, **F** = person-front-end.

| ID | Requirement | Owner | Detail |
|---|---|---|---|
| FR-1 | Person CRUD with soft delete and full audit trail | S | [service §6.1](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-2 | Multiple identifiers per record (typed, system-qualified: MRN, SSN, DL, NPI, PPN, TAX, Other) | S | [service §6.1](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-3 | Identity documents with expiry tracking (passport, national ID, driver's licence, …) | S | [service §6.1](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-4 | Multiple addresses, telecom contacts, emergency contacts | S | [service §6.1](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-5 | Probabilistic matching — weighted fuzzy scoring with per-component breakdown | S + M | [service §6.2](../person-service-with-loco/spec/06-functional-requirements.md), [matcher §12](../person-matcher-rust-crate/spec/12-algorithm-specifications.md) |
| FR-6 | Deterministic matching — short-circuit rules on tax ID, identifier, document; matcher adds 42 national-identifier schemes + passport books | S + M | [matcher §6](../person-matcher-rust-crate/spec/06-functional-requirements.md) |
| FR-7 | The service MUST expose the matcher's canonical algorithm through the adapter (§5.3); routing rules are normative and test-pinned | S | [adapter.rs](../person-service-with-loco/src/matching/adapter.rs) |
| FR-8 | Full-text + fuzzy + phonetic search (Tantivy, 11 indexed fields), pagination, optional masking | S | [service §6.3](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-9 | Duplicate detection: real-time `409` on create, explicit `check-duplicates`, batch `deduplicate` | S | [service §6.4](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-10 | Merge: transfer data, "former" alias, `Replaces` link, soft-delete duplicate, snapshot, `Merged` event | S | [service §6.4](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-11 | Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`) with auto-merge threshold | S | [service §6.4](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-12 | Validation + normalisation at the boundary (required fields, formats, E.164-like phone, address standardisation; `422` on failure) | S | [service §6.5](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-13 | Privacy: per-field masking, GDPR Article 15 export, consent model with status tracking | S | [service §6.6](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-14 | Audit: every CRUD / merge / link writes old + new JSON, user context, timestamp; per-person / recent / per-user queries | S | [service §6.7](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-15 | Event streaming: publish on every CRUD / merge / link (`*Created`, `*Updated`, `*Deleted`, `*Merged`, `*Linked`, `*Unlinked`) | S | [agents/share/auditability.md](../../agents/share/auditability.md) |
| FR-16 | FHIR R5 Person resource, bidirectional, with search parameters | S | [service §6.8](../person-service-with-loco/spec/06-functional-requirements.md) |
| FR-17 | Operator UI: list/search grid, create with inline 409-duplicate candidates, detail, edit, soft delete | F | [front-end §6](../person-front-end-with-svelte/spec/06-functional-requirements.md) |
| FR-18 | Operator UI: match check (score a hypothetical record), merge with preview, per-person audit view | F | [front-end §6](../person-front-end-with-svelte/spec/06-functional-requirements.md) |

### 6.1 Composition requirements

- **FR-19** — The front-end MUST consume only the service's public
  REST API (`/api/*`), never the database, search index, or matcher
  directly.
- **FR-20** — The matcher MUST remain a pure library (no IO, no
  async runtime, deterministic); the service is its only in-entity
  embedder.
- **FR-21** — Duplicate candidates returned in a `409` create
  response MUST render inline in the front-end create flow, so the
  operator can divert to match/merge without losing input.
