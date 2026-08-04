## 15. Roadmap

- **v0.1**: Scaffold (T-1).
- **v0.2** (shipped): T-2..T-14 — full CRUD + search + matching +
  merge + dedup + instance sub-resource + audit + streaming +
  privacy + bridge tests + integration tests + benches + OpenAPI.
- **v0.3** (shipped): T-16 — Prometheus metrics surface
  (`GET /metrics.prom`: process-wide registry, CRUD/merge counters,
  reserved labelled `http_requests_total`); plus Dockerfile non-root
  user rename, OpenAPI `info.version` sourced from `CARGO_PKG_VERSION`,
  and the doc-harmonization pass (test counts, status row, this
  roadmap, CHANGELOG version cut).
- **v0.4** (shipped): auth (T-15) — offline PASETO v4.public bearer
  verification + ABAC blanket guard (default-off via
  `COURSE_REQUIRE_AUTH`), coordinated with the family-wide auth
  rollout, plus key rotation / policy hot-reload without a restart
  (AU-2); the durable transactional-outbox bus + relay + retention
  (T-21/T-22) and its real-broker Fluvio adapter under the `fluvio`
  feature flag (T-23); the deliberately non-standard FHIR `Basic`
  surface (T-20); header-based API versioning (T-25); and row-level
  integrity digests + audit-log MAC verification, default off (T-24).
  **Still open**: the request-path middleware that increments the
  already-declared `http_requests_total{path,status}` counter (T-18).
- **v0.5** (next): bulk import / export (T-19, designed in §9.2, not
  yet built); syllabus-section sub-resource handlers + repository
  round-trip (currently the column ships JSONB, but no read/write
  API).
- **v0.6+**: LMS round-trip (LTI / xAPI import / export) — out of
  MVP, captured here so it stays on the radar.

