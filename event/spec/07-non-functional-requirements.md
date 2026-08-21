## 7. Non-Functional Requirements

Targets for a worldwide public governmental deployment with millions
of users. Figures sourced from the service crate's spec are
**single-node** measurements; multi-region targets are roadmap (§15)
and marked as such.

| # | Attribute | Target |
|---|---|---|
| NFR-1 | Scale | Millions of Event records; population-scale party references into the person / worker / organization entities |
| NFR-2 | Create latency | ≤ 50 ms p50 (single node, service spec §7) |
| NFR-3 | Read latency | ≤ 5 ms p50 (single node) |
| NFR-4 | Search latency | ≤ 100 ms p50 (single node) |
| NFR-5 | Match latency | ≤ 500 ms p99 (single node) |
| NFR-6 | Throughput | ≥ 1 000 req/sec per service instance; horizontal scaling via stateless app tier |
| NFR-7 | Availability | HADR; stateless app tier; PostgreSQL replication; health checks for orchestration; graceful shutdown. Multi-region active-active is roadmap (§15) |
| NFR-8 | i18n / locales | Multi-locale per [`agents/share/locales.md`](../../agents/share/locales.md) (46 languages); `in_language` on every Event; UTC storage + IANA `time_zone` for display. Time-zone-aware *fuzzy matching* is an open gap (service T-2) |
| NFR-9 | Security | SSO via the [authentication entity](../../authentication/) (passwordless magic-link, RS256 JWT + JWKS for offline verification). JWT enforcement on `/api/*` is **not yet implemented** (service T-8, entity ET-5) — until then the trio MUST NOT face the public internet unfronted |
| NFR-10 | Auditability | 100% of CRUD / merge / link operations audited (FR-12) and streamed (FR-13); match decisions explainable per component (FR-17) |
| NFR-11 | Privacy | Party / attendee data treated as personal data; masking on by default for public-facing reads; GDPR export ≤ 30 days statutory, target immediate via API (§12) |
| NFR-12 | Observability | OTLP traces / metrics / logs; `traceparent` per request; Prometheus text exposition at `/metrics.prom` |
| NFR-13 | Determinism | Matcher: same inputs ⇒ same outputs, byte-for-byte; no unsafe code |
| NFR-14 | Front-end resilience | Page shells render with the API down (banner shown, layout mounts) — pinned by Playwright smoke tests |
| NFR-15 | Background jobs | Loco `BackgroundQueue` backed by PostgreSQL (`bg_pg`) — no external broker |

### 7.1 Benchmarks

Single-node Criterion benches exist in the service
([`agents/testing.md`](../event-service-with-loco/agents/testing.md)):
matching (incl. probabilistic match against 50 candidates), search
(full-text + fuzzy over 500 docs), validation. The matcher crate
carries its own `benches/`. No load test at realistic governmental
event volumes has been run yet — service roadmap item; tracked here
as part of ET-9.
