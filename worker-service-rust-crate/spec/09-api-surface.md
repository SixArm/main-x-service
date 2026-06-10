## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/workers/*` + `/api/audit/*` + `/api/health` |
| FHIR R5 (Axum) | Practitioner CRUD + search under `/fhir/Practitioner` |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../../spec/index.md) |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

