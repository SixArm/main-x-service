## 9. API Surface

The world-facing API is **read-only**. There are no link-write
endpoints here — edge creation / withdrawal lives in the owning entity
service
([design §4.1](../../../agents/share/cross-service-linking.md#41-write-side--entity_links-per-participating-service)).
All state changes arrive via the bus consumers (§8), which are not an
HTTP surface.

| Tier | Surface |
|---|---|
| REST (loco.rs controllers on Axum) | Read endpoints under `/api/*`, registered as a loco `Routes` table. |
| Ops (loco built-ins) | `GET /_health` (DB + queue readiness) and `GET /_ping` (liveness), for orchestration probes, outside `/api`. |
| gRPC (Tonic) | Out of MVP scope. |
| Docs | Swagger UI at `/swagger-ui`, raw OpenAPI 3 JSON at `/api-docs/openapi.json` (utoipa). |
| Metrics | `GET /metrics.prom` — Prometheus text-exposition format, mounted at the application **root**, public. Gauges/counters: per-entity consumer lag, edge counts by `status`, reconciliation divergence, `linked` / `unlinked` / `merged` processed totals, `http_requests_total{path,status}`. |

All `/api` endpoints return `{ "success": bool, "data": …, "error": … }`.
Every **graph** response additionally carries an `as_of` watermark (§6
FR-17).

### 9.1 Read endpoints

```
GET /api/neighbors/{ref}?kind=&direction=out|in|both&depth=1
        Edges incident to {ref}. {ref} is the EntityRef URN
        (e.g. person:0c4f…), URL-encoded. `depth` is capped (§16).
        → { success, data: { ref, edges: [...], as_of }, error }

GET /api/edges?from=&to=&kind=&status=
        Filtered edge list (any subset of the four filters).
        → { success, data: { edges: [...], as_of }, error }

GET /api/single-view/{ref}
        Golden-record walk: same_identity unification + affiliations
        (person → worker → org employer derivation).
        → { success, data: { identity_refs: [...], affiliations: [...], as_of }, error }

GET /api/health/freshness
        Per-entity-topic last-consumed occurred_at + lag-versus-now.
        → { success, data: { topics: [{ entity, last_occurred_at, lag_seconds }], as_of }, error }
```

### 9.2 HTTP status conventions

- `200` — successful read.
- `400` — malformed `EntityRef` URN, unknown `kind`, or `depth` over
  the cap.
- `401` / `403` — unauthorised; specifically, requests that would
  surface a `case ↔ person` edge to a caller lacking case-read
  authorisation return `403` (or `404`-style concealment per §12) so
  the caller does not learn the edge exists.
- `404` — `{ref}` has no presence record **and** lazy verify-on-read
  resolves it as absent (distinguished from "present, no edges", which
  is `200` with an empty list).

### 9.3 Health

`GET /api/health/freshness` is the service's own envelope-wrapped
freshness endpoint (the eventual-consistency window made queryable);
the loco `/_health` / `/_ping` pair serves container orchestration.

### 9.4 Authorisation

- Affiliation edges (`same_identity`, `works_at`, `member_of`,
  `employed_by`) carry the default service posture (JWT verification
  per the family rollout; see §12 / §14).
- `case ↔ person` (`subject_of` / `about`) edges require case-read
  authorisation on every read path that could surface them, including
  `single-view` (§6 FR-18/20, §12).
