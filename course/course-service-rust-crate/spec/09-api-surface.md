## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | endpoints under `/api/courses/*` + `/api/courses/{id}/instances/*` + `/api/audit/*` + `/api/health` |
| gRPC (Tonic) | Out of MVP scope. |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa). |

All REST endpoints return `{ "success": bool, "data": …, "error": … }`.
HTTP status codes follow REST conventions: `409` for duplicate
detection on create, `422` for validation failure, `501` only for
`GET /api/courses` (list-all-without-search, intentionally
unimplemented — clients should call `/api/courses/search` with an
empty `q` for the same effect).

