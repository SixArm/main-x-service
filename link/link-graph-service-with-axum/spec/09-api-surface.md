## 9. API Surface

The world-facing API is **read-only**. There are no link-write
endpoints here — edge creation / withdrawal lives in the owning entity
service
([design §4.1](../../../agents/share/cross-service-linking.md#41-write-side--entity_links-per-participating-service)).
All state changes arrive via the bus consumers (§8), which are not an
HTTP surface.

The LNK-4 cross-service `same_identity` suggestion job (§6.8) does not
change this: it adds **no new route on this service**. It is a
periodic Tokio task that makes outbound calls as an HTTP *client*
(`GET` person + worker, `POST` to person's own link-write endpoint) —
the inverse direction of this table, and no different in kind from the
reconciliation worker's existing outbound `GET`s (§8.1). This table
remains the complete inbound surface.

| Tier | Surface |
|---|---|
| REST (axa controllers on Axum) | Read endpoints under `/api/*`, registered as an axa `Routes` table. |
| Ops (axa built-ins) | `GET /_health/live` (liveness), `GET /_health/ready` (DB + queue readiness) and `GET /_health/startup`, for orchestration probes, outside `/api`. |
| gRPC (Tonic) | Out of MVP scope. |
| Docs | Swagger UI at `/swagger-ui`, raw OpenAPI 3 JSON at `/api-docs/openapi.json` (utoipa). |
| Metrics | `GET /metrics.prom` — Prometheus text-exposition format, mounted at the application **root**, public. Gauges/counters: per-entity consumer lag, edge counts by `status`, reconciliation divergence, `linked` / `unlinked` / `merged` processed totals, `http_requests_total{path,status}`. |

All `/api` endpoints return bare JSON on success and an RFC 9457 problem
(`application/problem+json`, `code` `bad_request` on a `400`) on failure
([`spec/restful`](../../../spec/restful/index.md) §1.3). Every **graph**
response carries an `as_of` watermark (§6
FR-17).

### 9.1 Read endpoints

```
GET /api/neighbors/{ref}?kind=&direction=out|in|both&depth=1
        Edges incident to {ref}. {ref} is the EntityRef URN
        (e.g. person:0c4f…), URL-encoded. `depth` is capped (§16).
        → { ref, edges: [...], as_of }

GET /api/edges?from=&to=&kind=&status=
        Filtered edge list (any subset of the four filters).
        → { edges: [...], as_of }

GET /api/single-view/{ref}
        Golden-record walk: same_identity unification + affiliations
        (person → worker → org employer derivation).
        → { identity_refs: [...], affiliations: [...], as_of }

GET /api/health/freshness
        Per-entity-topic last-consumed occurred_at + lag-versus-now.
        → { topics: [{ entity, last_occurred_at, lag_seconds }], as_of }
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

`GET /api/health/freshness` is the service's own freshness endpoint (the eventual-consistency window made queryable);
the axa `/_health/live`, `/_health/ready`, `/_health/startup` pair serves container orchestration.

### 9.4 Authorisation

- Affiliation edges (`same_identity`, `works_at`, `member_of`,
  `employed_by`) carry the default service posture (JWT verification
  per the family rollout; see §12 / §14).
- `case ↔ person` (`subject_of` / `about`) edges require case-read
  authorisation on every read path that could surface them, including
  `single-view` (§6 FR-18/20, §12).
