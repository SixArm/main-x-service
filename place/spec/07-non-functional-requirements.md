## 7. Non-Functional Requirements

Targets for a worldwide public governmental deployment with millions
of users. Figures sourced from crate specs are **single-node**
measurements; multi-region targets are roadmap ([§15](15-roadmap.md)).

| # | Attribute | Target |
|---|---|---|
| NFR-1 | Scale | Millions of place records (national gazetteers run 1–10 M+ entries per country), thousands of data sources, millions of operator + machine users across the index |
| NFR-2 | Latency (single node, service [spec §7](../place-service-rust-crate/spec/07-non-functional-requirements.md)) | Create ≤ 50 ms p50; read ≤ 5 ms p50; search ≤ 100 ms p50; geo-radius ≤ 200 ms p50; match ≤ 500 ms p99 |
| NFR-3 | Throughput | ≥ 1 000 req/sec per service instance; horizontal scale via stateless app tier |
| NFR-4 | Availability | HADR: stateless app tier, PostgreSQL replication, health checks, graceful shutdown; multi-region active/passive is roadmap |
| NFR-5 | Matching determinism | Same inputs ⇒ same outputs, byte-for-byte; no clocks, RNGs, or env reads in the matcher (matcher [spec §8](../place-matcher-rust-crate/spec/08-determinism-and-safety.md)) |
| NFR-6 | i18n / locales | Operator-facing locales per [`agents/share/locales.md`](../../agents/share/locales.md); Unicode-diacritic-correct normalisation end to end; international address formats — note the matcher expands **English-only** street vocabularies today (matcher spec §1.2), a worldwide-deployment gap tracked in [§13](13-tasks.md) |
| NFR-7 | Security | SSO via the [authentication entity](../../authentication/) (RS256 JWT + JWKS offline verification); JWT enforcement on `/api/*` is not yet implemented — service spec §13 T-8, entity [§13](13-tasks.md) E-5 |
| NFR-8 | Privacy by default | No PII in logs (all three subprojects); masked views available wherever full records are; coordinate rounding for residence-linked places |
| NFR-9 | Auditability | 100 % of mutating operations produce an audit row + event; audit trail queryable over REST |
| NFR-10 | Observability | OTLP traces / metrics / logs, `traceparent` per request, Prometheus scrape at `/metrics.prom` |
| NFR-11 | Background jobs | Loco `BackgroundQueue` backed by PostgreSQL (`bg_pg`) — no external broker |
| NFR-12 | Front-end performance | List-page JS ≤ 250 kB gzipped; time-to-first-interaction < 1 s warm; WAI-ARIA conformance (front-end [spec §7](../place-front-end-with-svelte/spec/07-non-functional-requirements.md)) |
| NFR-13 | Library hygiene (matcher) | Pure library: no IO, no `unsafe`, no async runtime, no logging; SemVer-stable public API (matcher [spec §9](../place-matcher-rust-crate/spec/09-public-api-contract-semver.md)) |

### 7.1 Scale notes

- The matcher is pairwise and in-memory; population-scale candidate
  **blocking** (Tantivy pre-filter, geo bounding box) is the service's
  job and must keep candidate sets small enough to hold the match
  p99 (NFR-2) as record counts grow.
- Geo-radius search currently computes Haversine app-side with a
  bounding-box pre-filter; PostGIS `ST_DWithin` + GiST indexing
  (service spec §13 T-1) is required to hold NFR-2 at ≥ 1 M places.
