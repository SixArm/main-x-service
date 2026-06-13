## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (loco.rs controllers on Axum) | endpoints under `/api/courses/*` + `/api/courses/{id}/instances/*` + `/api/audit/*` + `/api/health`, registered as a loco `Routes` table with prefix `/api` |
| Ops (loco built-ins) | `GET /_health` (DB + queue readiness) and `GET /_ping` (liveness), from loco's default routes — for orchestration probes, outside `/api` |
| gRPC (Tonic) | Out of MVP scope. |
| Docs | Swagger UI at `/swagger-ui`, raw OpenAPI 3 JSON at `/api-docs/openapi.json` (utoipa). |

All `/api` endpoints return `{ "success": bool, "data": …, "error": … }`.
HTTP status codes follow REST conventions: `409` for duplicate
detection on create, `422` for validation failure, `501` only for
`GET /api/courses` (list-all-without-search, intentionally
unimplemented — clients should call `/api/courses/search` with an
empty `q` for the same effect).

`GET /api/health` is the service's own envelope-wrapped health
endpoint (kept for front-end and API-client parity); the loco
`/_health` / `/_ping` pair serves container orchestration.
