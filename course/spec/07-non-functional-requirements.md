## 7. Non-Functional Requirements

Entity-level targets for deployment inside a worldwide public
governmental system. Where a figure is **measured today** it comes
from the service crate's spec / benches and is single-node; where it
is a **scale target** it is marked roadmap (§15).

| ID | Requirement | Status |
|---|---|---|
| NFR-1 | **Capacity (measured baseline):** ≤ 256 MB resident for 1M courses + 5M instances indexed, on a single node | Single-node, [service §7](../course-service-with-loco/spec/07-non-functional-requirements.md) |
| NFR-2 | **Capacity (scale target):** millions of course templates + tens of millions of instances across national catalogues, behind horizontally scaled stateless service instances | Roadmap §15 |
| NFR-3 | **Throughput:** ≥ 1000 req/s sustained on a single 4-core host for `GET /api/courses/{id}` | Single-node, [service §7](../course-service-with-loco/spec/07-non-functional-requirements.md) |
| NFR-4 | **Latency p95 (single-node):** get ≤ 25 ms; search ≤ 100 ms; match ≤ 500 ms. Criterion benches cover matching, search, validation | [service §7](../course-service-with-loco/spec/07-non-functional-requirements.md), `benches/` |
| NFR-5 | **Search consistency:** a create MUST be observable via search on subsequent requests (`SearchEngine::reload()` after every commit) | Implemented |
| NFR-6 | **Availability:** health checks for orchestration (loco `/_health`, `/_ping`, service `/api/health`), graceful shutdown, connection pooling, stateless horizontal scaling per [`agents/share/availability.md`](../../agents/share/availability.md); multi-region replication is roadmap | Partial — single-region today |
| NFR-7 | **Internationalisation:** course records carry BCP-47 `in_language` / `available_language`; matching is diacritic-correct (matcher golden rule); operator UI localized to the [`agents/share/locales.md`](../../agents/share/locales.md) set | Data model done; UI localization roadmap §15 |
| NFR-8 | **Security:** SSO via the [authentication entity](../../authentication/) — service verifies RS256 JWT against JWKS offline; front-end carries the token. Not yet enforced (service T-15) | Roadmap §15 |
| NFR-9 | **Auditability:** every state change leaves an audit-log row + stream event; audit query API per [`agents/share/auditability.md`](../../agents/share/auditability.md) | Implemented (in-memory bus) |
| NFR-10 | **Observability:** structured tracing + OpenTelemetry OTLP export per [`agents/share/observability.md`](../../agents/share/observability.md) | Implemented |
| NFR-11 | **Determinism / explainability (matcher):** same inputs ⇒ same outputs byte-for-byte; no IO, clocks, RNG, or `unsafe`; every score ships a breakdown | Implemented, matcher golden rules |
| NFR-12 | **Front-end resilience:** page shells render even when the API is down (banner + mounted layout; pinned by Playwright smoke tests) | Implemented |
| NFR-13 | **Binary footprint:** service binary < 30 MB stripped | [service §7](../course-service-with-loco/spec/07-non-functional-requirements.md) |

The measured figures are **not** yet validated at governmental scale;
NFR-2, NFR-6 (multi-region), NFR-7 (UI locales), and NFR-8 are the
gap between today's MVP and the §15 roadmap.
