## 7. Non-Functional Requirements

Targets for a worldwide public governmental deployment with millions
of users. Figures sourced from crate specs are **single-node
measurements**; multi-region targets are roadmap (§15) until the
infrastructure lands.

| # | Attribute | Target |
|---|---|---|
| NFR-1 | Scale | Millions of Thing records; thousands of data sources; population-scale read traffic |
| NFR-2 | Service latency (single node, per [service §7](../thing-service-with-loco/spec/07-non-functional-requirements.md)) | Create ≤ 50 ms p50; read ≤ 5 ms p50; search ≤ 100 ms p50; match ≤ 500 ms p99; ≥ 1 000 req/sec per instance |
| NFR-3 | Matcher throughput (single node, per [matcher §7.1](../thing-matcher-rust-crate/spec/07-quality-attributes-and-tuning.md)) | `match_things` < 50 µs per call on typical records; `rank_one_to_many` linear in candidates; `Send + Sync` so callers may parallelise |
| NFR-4 | Availability | Stateless app tier, horizontal scaling, health checks, graceful shutdown ([agents/share/availability.md](../../agents/share/availability.md)); PostgreSQL replication; multi-region active/active is **roadmap** |
| NFR-5 | Internationalisation | Operator UI and API error text localisable to the full locale set in [agents/share/locales.md](../../agents/share/locales.md); Unicode-correct matching (diacritics round-trip through matcher normalisation) — UI localisation is **roadmap** |
| NFR-6 | Security | SSO via the central [authentication entity](../../authentication/) — RS256 JWT verified offline via JWKS; **not yet enforced** (service spec §13 T-4); non-root containers; env-based secrets; no PII in logs |
| NFR-7 | Auditability | 100 % of mutations audited (FR-10); audit rows retained per records-management policy; event stream durable once the production bus lands (§15) |
| NFR-8 | Observability | OTLP traces / metrics / logs; `traceparent` per request; Prometheus exposition at `/metrics.prom` ([agents/share/observability.md](../../agents/share/observability.md)) |
| NFR-9 | Determinism / explainability | Same match inputs ⇒ same outputs byte-for-byte; every score carries a per-field breakdown — the audit trail for automated decisions (ISO/IEC 42001 relevance, §12) |
| NFR-10 | Front-end performance | List page ≤ 250 kB gzipped JS; first interaction < 1 s warm; WAI-ARIA conformance ([front-end §7](../thing-front-end-with-svelte/spec/07-non-functional-requirements.md)) |
| NFR-11 | Background jobs | Loco `BackgroundQueue` on PostgreSQL (`bg_pg`) — no external broker |
| NFR-12 | Data durability | Soft delete only; merges keep a full transferred-data snapshot; backup / DR runbook is **roadmap** |
