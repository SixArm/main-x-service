## 15. Roadmap

- **Authentication & authorisation** — blanket PASETO enforcement on
  `/api/*` (the T-8 remainder; peer offline PASETO v4.public
  verification via the `authentication-verifier` crate landed
  2026-07-04, per
  [authentication-sessions](../../../agents/share/authentication-sessions.md)),
  RBAC for editor / curator / read-only / service roles, rate
  limiting, security headers.
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

