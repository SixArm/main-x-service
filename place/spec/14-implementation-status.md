## 14. Implementation Status

Honest snapshot per subproject. Aspirational items live in
[§15 Roadmap](15-roadmap.md), not here.

### 14.1 place-service-rust-crate (v0.5.0)

Delivered (service [spec §14](../place-service-rust-crate/spec/14-implementation-status.md)):
full schema.org/Place domain model; 13-table SeaORM schema; 15 REST
endpoints + Swagger; Tantivy search + app-side geo-radius; matching
(in-service scorer + embedded canonical matcher via adapter);
real-time / explicit / batch duplicate detection with review queue;
merge with snapshot + event; validation + normalisation; masking +
GDPR export + consent; audit log + audit API; Prometheus +
OTLP-ready observability; 104 unit + 67 integration + 14 bridge tests
+ 16 benchmarks.

Open gaps (tracked in service spec §13): PostGIS spatial queries
(T-1), hierarchy recursive CTEs (T-2), durable Fluvio publisher (T-3),
gRPC promotion from stub (T-4), OSM import (T-5), reverse geocoding
(T-6), GeoJSON export (T-7), authentication (T-8).

### 14.2 place-matcher-rust-crate (v0.6.1)

Delivered: authoritative §1–§13 spec; pure deterministic library
(`#![forbid(unsafe_code)]`, no IO); deterministic rule (shared
`(scheme, value)` place-id OR normalised name + postcode);
probabilistic weighted renormalised scoring over name / coordinates /
address / category / country code / place-ids / phone / email with
per-field breakdown; Soundex gating; `strict` / `default` / `lenient`
presets; builders, serde round-trip, examples, benchmarks, property
tests. Doc drift: spec-banner version mismatch resolved 2026-06-13
([§13](13-tasks.md) E-3 — banner now `0.6.1`).

### 14.3 place-front-end-with-svelte (MVP scaffold)

| Area | Status |
|---|---|
| Scaffold, types, API client, forms | ✅ |
| List + search, create + 409 surfacing, detail / edit / delete | ✅ |
| Match check, merge UI, audit view | ✅ |
| Unit (8) + e2e smoke (6) tests written | ✅ |
| `pnpm install` / `pnpm test` verified | ❌ — E-8 |
| Live integration against the service | ❌ — E-9 |
| Masked view, GDPR export, dedup-scan UI, localization | ❌ — front-end §13 T-18–T-20, entity E-10 |
| Person-entity copy artifacts (HumanName, emergency contacts) purged | ❌ — E-2 |

### 14.4 Entity-level

| Contract | Status |
|---|---|
| Service ↔ matcher adapter + bridge suite (14 tests) | ✅ |
| Front-end ↔ service wire types + envelope tests | ✅ (mocked only) |
| Entity spec (this document set) | ✅ inaugural |
| Live trio verification | ❌ — E-9 |
| SSO across the trio | ❌ — E-5 |
