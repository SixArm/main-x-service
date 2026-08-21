## 6. Functional Requirements

Entity-level requirements, each mapped to the owning subproject.
Detail lives in the owner's spec — link down, don't duplicate.

| # | Requirement | Owner | Detail |
|---|---|---|---|
| FR-1 | Create / read / update / soft-delete worker records, with audit trail on every mutation | service | [service §6.1](../worker-service-with-loco/spec/06-functional-requirements.md) |
| FR-2 | Multiple professional identifiers per worker (type + system + value; NPI, DEA, employee #, ODS, national IDs) | service | service §6.1 |
| FR-3 | Credential / identity documents with expiry tracking (passport, licence, permits, …) | service | service §6.1, [`agents/models.md`](../worker-service-with-loco/agents/models.md) |
| FR-4 | Multiple addresses, telecom contacts, and emergency contacts per worker | service | service §6.1 |
| FR-5 | Probabilistic matching — weighted fuzzy scoring with per-component breakdown | service (in-service) + matcher (canonical) | [service §6.2](../worker-service-with-loco/spec/06-functional-requirements.md), [matcher §12](../worker-matcher-rust-crate/spec/12-algorithm-specifications.md) |
| FR-6 | Deterministic matching — rule-based with short-circuits (tax-ID, identifier, document exact match) | service + matcher | service §6.2, matcher §12 |
| FR-7 | National-identifier matching MUST be scheme-local across all 42 supported schemes; never cross-match schemes | matcher (enforced); service adapter (respected) | matcher §12.1; this spec §5.3 |
| FR-8 | The service MUST reach the canonical matcher only through `to_matcher_worker()`; routing rules are pinned by the bridge tests | service | this spec §5.3 |
| FR-9 | Full-text + fuzzy + phonetic search with pagination and optional masking | service | [service §6.3](../worker-service-with-loco/spec/06-functional-requirements.md) |
| FR-10 | Real-time duplicate detection on create (`409` with candidates) | service | service §6.4 |
| FR-11 | Explicit duplicate check without creating (`check-duplicates`) | service | service §6.4 |
| FR-12 | Batch deduplication scan with configurable thresholds | service | service §6.4 |
| FR-13 | Review queue for borderline duplicates (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`) | service | service §6.4 |
| FR-14 | Merge: transfer data, alias former name, `Replaces` link, soft-delete duplicate, JSON snapshot, `Merged` event | service | service §6.4 |
| FR-15 | Validation and normalisation at the boundary; failures → `422` | service | service §6.5 |
| FR-16 | Privacy: per-field masking, masked view, GDPR export, consent model | service | service §6.6 |
| FR-17 | Audit: every CRUD / merge / link writes old + new JSON with user context; queryable per-worker / recent / per-user | service | service §6.7 |
| FR-18 | Event streaming: publish Created / Updated / Deleted / Merged / Linked / Unlinked | service | [`agents/share/auditability.md`](../../agents/share/auditability.md) |
| FR-19 | FHIR R5 Practitioner bidirectional conversion + search parameters | service | service §6.8 |
| FR-20 | Operator UI: dashboard, list/search, create with 409 surfacing, detail, edit, audit view, match check, merge | front-end | [front-end §6](../worker-front-end-with-svelte/spec/06-functional-requirements.md) |
| FR-21 | Front-end MUST mirror the service wire format in its own `types.ts`; drift between sibling front-ends is accepted, drift against the service is not | front-end | [front-end `AGENTS.md`](../worker-front-end-with-svelte/AGENTS.md) |
| FR-22 | Per-field explainability: every probabilistic match result carries a per-component score breakdown end-to-end (matcher → service API → front-end match UI) | all three | matcher §11; service §6.2; front-end §6 |
