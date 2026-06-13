## 15. Roadmap

The path from today's single-region MVP trio to a worldwide public
governmental registry. Ordered roughly by dependency; per-crate
roadmaps ([service §15](../event-service-rust-crate/spec/15-roadmap.md),
[front-end §15](../event-front-end-with-svelte/spec/15-roadmap.md))
carry the implementation detail.

1. **Security baseline — SSO enforcement (ET-5).** JWT middleware on
   `/api/v1/*` verifying RS256 tokens against the
   [authentication entity](../../authentication/)'s JWKS; roles for
   scheduler / admin / read-only / service; front-end magic-link
   sign-in. Rate limiting and security headers ride along. Nothing
   else on this list ships to the public before this does.
2. **Durable event bus (ET-6).** Replace `InMemoryEventPublisher`
   with a durable, replayable stream; add consumers. Prerequisite
   for cross-region convergence and for downstream notification /
   transparency systems.
3. **Multi-region replication.** Regional PostgreSQL clusters with
   cross-region replication; region-local Tantivy indexes hydrated
   from the event bus; latency-based routing. Today everything is
   single-region (§8.5).
4. **Externalised search.** Move the search index out of the app
   instance (shared index service or rebuild-from-stream) so the
   stateless tier scales without per-node reindex cost.
5. **Recurring events — RFC 5545 RRULE** (service T-3). Governmental
   programmes are heavily recurring (weekly clinics, monthly
   hearings). Add `recurrence_rule`, expansion for search + dedup,
   and an entity-level decision on whether occurrences are
   materialised `sub_events` (EOQ-2).
6. **Localization / time-zone maturity.** Time-zone-aware fuzzy
   matching via `chrono-tz` (service T-2); locale-negotiated
   front-end UI strings across the
   [`agents/share/locales.md`](../../agents/share/locales.md) set;
   localized date/time rendering from UTC + IANA `time_zone`;
   iCalendar import/export (service T-7) for citizen-facing
   interchange.
7. **Operator-UI completion.** Wire the dormant endpoints —
   check-duplicates preview, batch dedup-scan results, masked-view
   toggle, GDPR-export download (front-end T-17…T-20); Lily
   component adoption (T-14).
8. **Scale proof (ET-9).** Load test at millions of records;
   btree_gist exclusion constraints; time-range query caching;
   Kubernetes (Helm, HPA, probes), OpenTofu modules, backup + DR
   runbook.
9. **Protocol completion.** gRPC for high-throughput agency
   integrators (service T-6); FHIR R5 once the Encounter /
   Appointment mapping is decided (service T-1, OQ-1).
10. **Assurance.** Security audit + pen test, GDPR validation, ISO
    27001 control evidence; any ML-assisted scoring enters only via
    an ISO 42001 impact assessment (§12.2).
