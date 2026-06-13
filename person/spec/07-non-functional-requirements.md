## 7. Non-Functional Requirements

Targets for the worldwide public governmental deployment. Where a
figure is a **current single-node benchmark** from a crate spec it is
marked as such; the population-scale targets that require new
infrastructure are roadmap items (§15), not claims about today.

| ID | Attribute | Target |
|---|---|---|
| NFR-1 | Scale | Hundreds of millions of person records; millions of operator + machine users across agencies. Current service spec targets "millions of persons" single-node — the gap is §15 roadmap. |
| NFR-2 | Latency (current single-node figures, service crate) | Create ≤ 50 ms p50 (incl. dup-check + index + audit); read ≤ 5 ms p50; search ≤ 100 ms p50; match ≤ 500 ms p99 |
| NFR-3 | Throughput (current single-node figure) | ≥ 1 000 req/sec per service instance |
| NFR-4 | Matcher performance | Single pairwise match in microseconds; no IO; deterministic — same inputs ⇒ same outputs byte-for-byte ([matcher §7](../person-matcher-rust-crate/spec/07-non-functional-requirements.md)) |
| NFR-5 | Availability | Stateless service tier behind health checks (`/api/health`); horizontal scaling; PostgreSQL replication; graceful shutdown. See [agents/share/availability.md](../../agents/share/availability.md). Multi-region active-active is roadmap (§15). |
| NFR-6 | Internationalisation | Matcher: NFKD diacritic-correct normalisation, 42 national-identifier schemes, E.164 phone across 39 jurisdictions. Front-end: target the locale set in [agents/share/locales.md](../../agents/share/locales.md) (today: en only — §15). |
| NFR-7 | Security | SSO via the [authentication entity](../../authentication/): service verifies RS256 JWTs offline against the provider's JWKS. Not yet wired (service §13 T-1; entity §13 E-1). TLS at the edge; non-root containers; no secrets in code. |
| NFR-8 | Observability | Tracing + OpenTelemetry OTLP (traces / metrics / logs), `traceparent` per request, Prometheus exposition at `/metrics.prom`. See [agents/share/observability.md](../../agents/share/observability.md). |
| NFR-9 | Auditability | 100% of mutating operations produce an audit row and an event (FR-14/FR-15); audit history queryable by person, time, and user |
| NFR-10 | Explainability | No match decision without a per-component breakdown, end to end: matcher output → REST response → operator UI |
| NFR-11 | Data durability | PostgreSQL with migrations; soft delete only; backup / DR runbook is roadmap (§15) |
| NFR-12 | Accessibility | Operator UI targets WCAG-conformant primitives (Lily Headless for focus trap / listbox / dialog); full audit is roadmap (§15) |
| NFR-13 | Code quality gates | Rust: `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test` green; matcher additionally forbids `unsafe` and IO. Front-end: `svelte-check` strict + vitest + playwright green. |

### 7.1 Entity-level composition constraints

- The front-end MUST degrade gracefully when the service is down
  (page shells render with an error banner — pinned by its e2e smoke
  suite).
- The matcher MUST NOT log or `Debug`-format person data; the service
  MUST NOT write PII into traces or event payload metadata beyond
  what the audit contract requires.
- Configuration thresholds (match threshold, auto-merge threshold)
  are service-side and tunable per deployment; the front-end MUST NOT
  hard-code them.
