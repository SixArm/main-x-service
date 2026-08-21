## 14. Implementation Status

Honest per-subproject status as of 2026-06. Aspirational items live
in [§15 Roadmap](15-roadmap.md), not here.

### 14.1 event-service-with-loco

| Capability | Status |
|---|---|
| CRUD + soft delete + validation (`422`) | ✅ |
| Matching (in-service probabilistic + deterministic) | ✅ |
| Embedded matcher bridge (`adapter.rs` + 16 bridge tests) | ✅ |
| Search (Tantivy; fuzzy, facets, date range, name+date blocking) | ✅ |
| Duplicate detection (real-time 409 + explicit + batch) + review queue | ✅ |
| Merge (transfer + alias + link + snapshot + event) | ✅ |
| Privacy (masking, GDPR export, consent model) | ✅ |
| Audit log + query endpoints | ✅ |
| Event streaming | ⚠️ In-memory publisher only (ET-6) |
| REST `/api` + OpenAPI/Swagger + Prometheus | ✅ |
| FHIR R5 | ❌ `501` stub (service T-1, OQ-1) |
| gRPC | ❌ Tonic stub (service T-6) |
| AuthN/AuthZ | ❌ None (ET-5 / service T-8) |
| Tests | ✅ 62+ unit, 16 bridge, 3 integration; Criterion benches; CI |

### 14.2 event-matcher-rust-crate

| Capability | Status |
|---|---|
| Probabilistic engine (weight-renormalised, per-field breakdown) | ✅ 0.5.0 |
| Deterministic rule (shared event ID, or name + start instant) | ✅ |
| Normalisation, Soundex, Gaussian temporal decay, Haversine geo | ✅ |
| Config presets (default / strict / lenient), serde-loadable | ✅ |
| Purity guarantees (no IO, no unsafe, deterministic) | ✅ |
| Living spec | ⚠️ Partially superseded — §1/§3/§5/§7 + examples still describe the 0.4.x place matcher (ET-1) |

### 14.3 event-front-end-with-svelte

| Capability | Status |
|---|---|
| Scaffold, types, ApiClient + EventRepository | ✅ |
| List + search grid, create + 409 surfacing, detail/edit/delete | ✅ |
| Audit view, match check, merge UI | ✅ |
| Unit tests (8) + e2e smoke (6) | ✅ |
| `pnpm install` / `pnpm test` verified | ❌ Manual step pending (ET-7) |
| Live integration against a running service | ❌ Pending operator walkthrough (ET-7) |
| check-duplicates / dedup-scan / masked / export UI | ❌ Endpoints available, not routed (front-end T-17…T-20) |
| Copy drift from person entity (README, T-15 wording) | ❌ ET-2 |

### 14.4 Entity level

| Capability | Status |
|---|---|
| This spec (§1–§18) + `agents/` reference set | ✅ Inaugural |
| Seam tests (bridge + typed client) | ✅ |
| Cross-entity link integrity after monorepo nesting | ❌ ET-3 |
| Multi-region, durable bus, SSO, load-tested scale | ❌ Roadmap (§15) |
