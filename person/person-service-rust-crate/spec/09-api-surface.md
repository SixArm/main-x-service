## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/persons/*` + `/api/audit/*` + `/api/health` |
| FHIR R5 (Axum) | Person CRUD + search under `/fhir/Person` |
| gRPC (Tonic) | Stubbed; not yet implemented |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

All REST endpoints return `{ "success": bool, "data": …, "error": … }`.
HTTP status codes follow REST conventions: `409` for duplicate
detection on create, `422` for validation failure.

