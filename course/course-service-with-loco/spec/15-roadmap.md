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
- **v0.4** (next): ~~auth (T-15)~~ **done** — offline PASETO v4.public
  bearer verification + ABAC blanket guard (default-off via
  `COURSE_REQUIRE_AUTH`), coordinated with the family-wide auth rollout;
  Fluvio streaming adapter under a feature flag; and a request-path
  middleware that increments the already-declared
  `http_requests_total{path,status}` counter.
- **v0.5**: Syllabus-section sub-resource handlers + repository
  round-trip (currently the column ships JSONB, but no read/write
  API).
- **v0.6+**: LMS round-trip (LTI / xAPI import / export) — out of
  MVP, captured here so it stays on the radar.

