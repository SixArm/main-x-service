## 15. Roadmap

- **Authentication & authorisation** — T-8 is complete: peer offline
  PASETO v4.public verification, the default-off `PLACE_REQUIRE_AUTH`
  blanket `/api/*` enforcement middleware, the boot-time
  `PLACE_PASETO_KEYS_URL` HTTP key-set fetch (all 2026-07-04), and
  **ABAC authorization** (2026-07-05, per
  [authorization-attributes](../../../agents/share/authorization-attributes.md)
  — the shared policy engine over the token's `attrs` claim;
  supersedes the earlier RBAC roles sketch). Remaining: activation is
  an operations decision; richer deployment policies are
  configuration (`PLACE_ABAC_POLICY*`), not code; record-level
  resource attributes are a shared-design open question. Also: a
  periodic key-set **re-fetch/refresh loop** for key rotation (today
  the fetch happens once at boot), rate limiting, security headers.
- **Observability** — Prometheus alongside OTLP, complete OTLP trace
  exporter, custom metrics (`place_created`,
  `geo_search_radius_km_histogram`), Grafana dashboards + alerting.
- **Performance** — PostGIS spatial indexing, recursive CTEs for
  hierarchy depth, batch fixes.
- **Infrastructure as code** — OpenTofu modules, multi-cloud, secrets,
  backup + DR.
- **Kubernetes** — Helm chart, HPA, PVCs for the search index,
  ingress, probes.
- **Production readiness** — security audit + pen test, GDPR
  validation, DR runbook, backup / restore, CI/CD pipeline.
- **Feature enhancements** — complete gRPC; Fluvio production +
  consumers; **OSM import pipeline**; **GeoJSON export**;
  **map-tile clustering for high-density searches**;
  **reverse-geocoding endpoint**.

