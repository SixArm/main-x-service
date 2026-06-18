## 14. Implementation Status

Honest snapshot per subproject. Aspirational items live in
[§15 Roadmap](15-roadmap.md), not here.

### 14.1 thing-service-with-loco

| Capability | Status |
|---|---|
| Domain model (schema.org/Thing + PropertyValue identifiers) | ✅ |
| REST API — 15 endpoints, OpenAPI / Swagger, CORS, envelopes | ✅ (loco.rs 0.16 / Axum 0.8) |
| In-service matching (5 components) + deterministic short-circuits | ✅ |
| Embedded thing-matcher 0.6.1 via `adapter.rs` + 15 bridge tests | ✅ |
| Tantivy search | ✅ |
| Validation / normalisation (`422`) | ✅ |
| Privacy: masking, GDPR export, consent model | ✅ |
| Audit log + audit query endpoints | ✅ |
| Event streaming | ⚠️ in-memory publisher only (Fluvio: service T-1) |
| gRPC | ❌ stub (service T-3) |
| AuthN / AuthZ | ❌ not enforced (service T-4; entity T-6) |
| Tests | ~100 unit + 6 `integration_*` suites + bridge tests + benchmarks |

### 14.2 thing-matcher-rust-crate

| Capability | Status |
|---|---|
| Deterministic match (identifier pair / sameAs URL / canonical URL) | ✅ |
| Probabilistic engine — 10 components, renormalised weighted sum | ✅ |
| Presets (default 0.80 / strict 0.95 / lenient 0.65) + 13 config knobs | ✅ |
| Batch `match_one_to_many` / `rank_one_to_many` | ✅ |
| Pure-library guarantees (no IO, no `unsafe`, deterministic) | ✅ |
| SemVer-stable public API (`#[non_exhaustive]`) | ✅ (shipped 0.6.1; spec banner stale — entity T-4) |

### 14.3 thing-front-end-with-svelte

| Capability | Status |
|---|---|
| Scaffold, types, ApiClient / ThingRepository | ✅ |
| List + search, create + 409 surfacing, detail / edit / delete | ✅ |
| Audit view, match check, merge UI | ✅ |
| Unit tests (8) + e2e smoke (6) | ✅ |
| `pnpm install` / `pnpm test` verified | ❌ (entity T-5) |
| Live integration walkthrough | ❌ (entity T-5) |
| check-duplicates / deduplicate / masked / export routes | ❌ (entity T-7) |

### 14.4 Entity-level

| Capability | Status |
|---|---|
| This umbrella spec + `AGENTS/` reference set | ✅ (inaugural) |
| Cross-subproject link integrity post-nesting | ❌ (entity T-1) |
| Doc drift (endpoint name, sibling prose, version banner) | ❌ (entity T-2–T-4) |
| Confidence-vocabulary mapping | ❌ (entity T-8) |
