## 7. Non-Functional Requirements

Targets for a worldwide public governmental deployment with millions
of users. Figures marked *(current single-node)* are measured /
specified on today's single-instance deployments — the multi-region
path to meet them at population scale is roadmap (§15), not delivered.

| # | Attribute | Target |
|---|---|---|
| NFR-1 | Scale | Millions of worker records; thousands of organisations; population-scale national registries per [service §7](../worker-service-with-loco/spec/07-non-functional-requirements.md) |
| NFR-2 | Latency *(current single-node)* | Create ≤ 50 ms p50; read ≤ 5 ms p50; search ≤ 100 ms p50; match ≤ 500 ms p99 |
| NFR-3 | Throughput *(current single-node)* | ≥ 1 000 req/sec per service instance |
| NFR-4 | Pairwise match cost | Microseconds per comparison on commodity hardware ([matcher NFR-1](../worker-matcher-rust-crate/spec/07-non-functional-requirements.md)) — this is what makes batch dedup over millions of records tractable |
| NFR-5 | Availability | Stateless app tier, health-check endpoints for orchestration, graceful shutdown, connection pooling, horizontal scaling; PostgreSQL replication ([`agents/share/availability.md`](../../agents/share/availability.md)) |
| NFR-6 | Determinism | Matching MUST be reproducible: same inputs ⇒ same score, byte-for-byte (matcher NFR-5 / FR-11). Required for audit defensibility of merge decisions |
| NFR-7 | i18n / locales | Diacritic-correct name handling via NFKD (matcher NFR-11); identifier schemes spanning 40+ jurisdictions; E.164 phone normalisation for 39 countries; UI locale catalogue per [`agents/share/locales.md`](../../agents/share/locales.md) (front-end localisation is roadmap §15) |
| NFR-8 | Security | SSO via the [authentication entity](../../authentication/) — offline **PASETO v4.public** (Ed25519) bearer verification against the published key set (RS256/JWKS decommissioned); blanket `/api/*` enforcement plus **ABAC** authorization over the token's `attrs` claim (per [`authorization-attributes.md`](../../agents/share/authorization-attributes.md); supersedes the earlier RBAC roles sketch), default-off via `WORKER_REQUIRE_AUTH`. Delivered (service §13 T-1); activation is an operations decision |
| NFR-9 | Privacy by default | Sensitive workforce fields (SSN, tax ID, DEA, home address) masked on demand; matcher library MUST never log or persist worker data (pure, no-IO) |
| NFR-10 | Observability | OTLP traces / metrics / logs; `traceparent` per request; Prometheus exposition at `/metrics.prom` ([`agents/share/observability.md`](../../agents/share/observability.md)) |
| NFR-11 | Auditability | 100 % of mutations audited with user context; audit log append-only via soft-delete-only policy (§12) |
| NFR-12 | Background jobs | Loco `BackgroundQueue` backed by PostgreSQL (`bg_pg`) — no external broker |
| NFR-13 | Supply-chain hygiene | Matcher crate stays dependency-light, no `unsafe`, clippy-clean; service pins the matcher by SemVer (`worker-matcher = "0.6.1"`) |
| NFR-14 | Front-end quality | TypeScript strict + `noUncheckedIndexedAccess`; page shells must render even when the API is down (degraded-mode banner) |

### 7.1 Tension to manage

NFR-2/NFR-3 figures come from the service crate's single-node spec and
benchmarks. Governmental scale (NFR-1) multiplies them across regions
and replicas; the deltas — durable event bus, externalised search,
read replicas, multi-region failover — are tracked in §15, and MUST
NOT be claimed as delivered in §14.
