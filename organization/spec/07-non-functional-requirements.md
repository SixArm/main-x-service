## 7. Non-Functional Requirements

Targets are set for the worldwide-governmental-system goal; the
**Today** column records honestly where the MVP stands. Gaps drive
§13 tasks and §15 roadmap items.

| # | Attribute | Target | Today |
|---|---|---|---|
| NFR-1 | Scale | Millions of organization records (national company-register / GLEIF-LEI order of magnitude); millions of operator and machine users across the index | Single Postgres; list capped 100; `check-duplicates` scans ≤ `CHECK_DUPLICATES_SCAN_CAP` (1 000) rows in-process and logs a `WARN` on cap saturation so truncation is observable — still needs blocking/candidate search (T-7) before register scale |
| NFR-2 | Availability | HADR; stateless app tier behind a load balancer; PostgreSQL replication; health-checked orchestration (see [`agents/share/availability.md`](../../agents/share/availability.md)) | loco `/_health` + `/_ping`; single instance; in-memory event buffer is per-process (not HA-safe) |
| NFR-3 | Performance | Read ≤ 5 ms p50; search ≤ 100 ms p50; duplicate check ≤ 500 ms p99 at full register volume | Unmeasured; no benchmarks in the service crate (matcher is pure CPU and fast by construction) |
| NFR-4 | i18n / locales | Legal-suffix handling across jurisdictions; diacritic-correct matching; operator UI localized per [`agents/share/locales.md`](../../agents/share/locales.md) | Matcher: legal-suffix list (const), NFKC fold, diacritics preserved, ISO 3166 jurisdictions. UI: English only; suffix list not yet configurable |
| NFR-5 | Security | SSO via the [authentication entity](../../authentication/) — service verifies short-lived PASETO v4.public tokens offline against the published Ed25519 key (see [`authentication-sessions.md`](../../agents/share/authentication-sessions.md), superseding RS256-JWT + JWKS); TLS at the edge; least-privilege DB roles | Unauthenticated (MVP); auth middleware queued (§13); audit `actor` is always `null` until auth lands |
| NFR-6 | Auditability | Every state change attributable (who / what / when) with a snapshot; durable event stream for downstream consumers (see [`agents/share/auditability.md`](../../agents/share/auditability.md)) | Audit rows with action + snapshot ✔; `actor` pending auth; events in-memory only |
| NFR-7 | Observability | OTLP traces / metrics / logs; `traceparent` per request; JSON logs in production | loco `tracing` + `tracing-subscriber` (env-filter, JSON feature); no OTLP exporter yet |
| NFR-8 | Matcher quality | Total functions (no `unwrap` / `expect` / `panic!`), no `unsafe`, deterministic (no clocks / RNG / env), explainable per-component breakdown, diacritic-correct | Met — enforced by the matcher's golden rules and clippy `-D warnings` |
| NFR-9 | Data durability | Soft delete only; point-in-time recovery; migration discipline via `sea-orm-migration` | Soft delete ✔; migrations ✔ (`auto_migrate` in development); backup/DR is deployment-side |
