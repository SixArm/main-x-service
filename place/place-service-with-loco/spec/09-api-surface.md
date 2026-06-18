## 9. API Surface

Complete reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 14 endpoints under `/api/places/*` + `/api/audit/recent` + `/api/health` |
| Observability | `GET /metrics.prom` (root path, Prometheus text-exposition `text/plain; version=0.0.4`) |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../../spec/index.md) |
| Docs | Swagger UI at `/swagger-ui` |

The 14 REST endpoints are: `GET /api/health`; `POST /api/places`;
`GET`/`PUT`/`DELETE /api/places/{id}`; `GET /api/places/search`;
`POST /api/places/match`; `POST /api/places/check-duplicates`;
`POST /api/places/merge`; `POST /api/places/deduplicate`;
`GET /api/places/{id}/export`; `GET /api/places/{id}/masked`;
`GET /api/places/{id}/audit`; `GET /api/audit/recent`.

Search query parameters are `q`, `limit`, `fuzzy`, `mask_sensitive`.
Geo-radius search (`nearby`), an `/api/audit/user` route, and search
`offset` pagination are **not yet delivered** — see §13 T-9.

This crate does **not** expose a FHIR R5 surface — Places are not a
FHIR-resource concern.

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

