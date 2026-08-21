## 14. Implementation Status

Honest per-subproject status. Worldwide-scale capabilities that do
not exist yet live in §15, not here.

### 14.1 worker-service-with-loco (v0.5.0)

| Capability | Status |
|---|---|
| CRUD + soft delete + validation (`422`) | ✅ |
| Matching (in-service probabilistic + deterministic) | ✅ |
| Canonical-matcher embed (`worker-matcher 0.6.1` + adapter + 13 bridge tests) | ✅ |
| Search (Tantivy, 11 fields, fuzzy + phonetic) | ✅ — local-disk index |
| Duplicate detection (real-time + explicit + batch + review queue) | ✅ |
| Merge (transfer / alias / link / snapshot / event) | ✅ |
| Privacy (masking, GDPR export, consent model) | ✅ — consent not enforced |
| Audit log + query API | ✅ |
| Event streaming | ⚠️ in-memory only; no durable bus, no consumers |
| REST + OpenAPI/Swagger + Prometheus `/metrics.prom` | ✅ |
| FHIR R5 Practitioner | ⚠️ partial — no capability statement / bundles; path discrepancy (§13 T-1) |
| gRPC | ❌ stub |
| AuthN/AuthZ | ❌ none — `loco-rs` `auth_jwt` feature compiled in, not enforced (service §13 T-1) |
| Tests | 99 unit + 7 integration + 13 bridge + 3 bench suites |

Detail: [service §14](../worker-service-with-loco/spec/14-implementation-status.md).

### 14.2 worker-matcher-rust-crate (v0.6.1, published to crates.io)

| Capability | Status |
|---|---|
| Deterministic + probabilistic engine with per-field breakdown | ✅ |
| 42 national-identifier scheme parsers, scheme-local | ✅ |
| 9 passport-format validators + `PassportBook` matching | ✅ |
| Normalisation (NFKD names, postcodes, E.164 phones × 39 countries, nicknames) | ✅ |
| Config presets (`strict` / `default` / `lenient`), `strict_mode` | ✅ |
| Pure-library guarantees (no IO / unsafe / async; deterministic) | ✅ |
| Spec banner version | ⚠️ stale — says 0.3.0 (§13 T-2) |

Detail: matcher [§23](../worker-matcher-rust-crate/spec/23-tasks-and-acceptance-criteria.md)
and delivered-task archives in its `agents/`.

### 14.3 worker-front-end-with-svelte (MVP)

| Capability | Status |
|---|---|
| All 8 MVP routes (dashboard … merge) | ✅ |
| Types / client / repository mirroring the service | ✅ |
| 409-duplicate inline surfacing on create | ✅ |
| Unit (8) + e2e smoke (6) tests | ✅ |
| Privacy UI (masked view, export download, consent) | ❌ (§13 T-5) |
| Sub-record editing (identifiers, addresses, emergency contacts) | ❌ read-only (front-end T-15) |
| Install / test / live-walkthrough verification | ❌ pending (§13 T-8) |
| Localisation | ❌ en only |

Detail: [front-end §14](../worker-front-end-with-svelte/spec/14-implementation-status.md).

### 14.4 Entity-level seams

| Seam | Status |
|---|---|
| Service↔matcher DTO contract + bridge tests | ✅ (§5.3, §11.2) |
| Front-end↔service live contract test | ❌ (§13 T-3) |
| SSO across the trio | ❌ (§13 T-4) |
| Entity-level spec (this document set) | ✅ first edition |
