## 14. Implementation Status

Honest snapshot per subproject. Worldwide-governmental-scale
capabilities that do not exist yet are **not** listed here — they are
§15 roadmap.

### 14.1 person-service-with-loco

| Capability | Status |
|---|---|
| CRUD + soft delete + audit + validation + privacy + consent model | ✔ delivered |
| Matching: in-service probabilistic + deterministic; embedded canonical matcher via adapter | ✔ delivered |
| Search (Tantivy 11-field, fuzzy + phonetic + bulk) | ✔ delivered |
| Duplicate detection (real-time / explicit / batch) + review queue + merge | ✔ delivered |
| REST API (15 endpoints) + OpenAPI/Swagger + Prometheus `/metrics.prom` | ✔ delivered |
| FHIR R5 Person | Partial — bundles + capability statement open (service T-3, T-4) |
| Event streaming | In-memory publisher only (service T-2) |
| gRPC | Stub (service T-6) |
| Authentication | ✘ none — API is open (service T-1 / entity E-1) |
| Tests | ~100 unit + 7 integration + 14 bridge + 3 benchmark suites |

Detail: [service spec §14](../person-service-with-loco/spec/14-implementation-status.md).

### 14.2 person-matcher-rust-crate

| Capability | Status |
|---|---|
| Deterministic + probabilistic engine, per-field breakdown, config presets | ✔ delivered (v0.3.0) |
| 42 national personal-identifier schemes + 9 passport-format validators | ✔ delivered |
| Normalisation: NFKD diacritics, postcode, E.164 phone (39 jurisdictions), nicknames | ✔ delivered |
| Purity guarantees: no IO, no `unsafe`, deterministic, `Send + Sync` | ✔ delivered |
| Open items | §22 OQ-7 (phonetic-bonus weighting documentation); roadmap research in `agents/roadmap-research.md` |

Detail: [matcher spec §23](../person-matcher-rust-crate/spec/23-tasks-and-acceptance-criteria.md)
and [agents/delivered-tasks.md](../person-matcher-rust-crate/agents/delivered-tasks.md).

### 14.3 person-front-end-with-svelte

| Capability | Status |
|---|---|
| Scaffold, types, ApiClient, form primitives | ✔ delivered |
| List + search grid, create + 409 surfacing, detail / edit / delete, audit, match, merge | ✔ delivered (MVP) |
| Unit (8) + e2e smoke (6) + integration golden-paths (9 written) | ✔ written; live run blocked (entity E-2 / front-end OQ-5) |
| Sub-record editing (identifiers / addresses / emergency contacts) | ✘ read-only on detail (front-end T-15) |
| Masked view, GDPR-export download, consent UI, dedup-scan UI | ✘ queued (front-end T-18–T-20; entity E-4, E-5) |
| Localization | ✘ English only (entity E-7) |
| `pnpm install` / `pnpm test` operator verification | Pending per front-end spec §14 |

Detail: [front-end spec §14](../person-front-end-with-svelte/spec/14-implementation-status.md).

### 14.4 Cross-subproject gaps

| Gap | Task |
|---|---|
| No authentication anywhere in the trio | E-1 |
| Seam-2 suite never run green against a live service | E-2 |
| Events not durable — downstream agencies cannot subscribe | E-3 |
| Data-subject-rights path incomplete in the UI | E-4, E-5 |
| Adapter scheme-routing coverage undocumented | E-8 |
| Broken repo-root relative links after entity nesting | E-9 |
