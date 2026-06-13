## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/v1/events/*` + `/api/v1/audit/*` + `/api/v1/health` |
| FHIR R5 (Axum) | `501 Not Implemented` stub (see §6.8) |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../../spec/index.md) |
| Docs | Swagger UI at `/swagger-ui` |

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

