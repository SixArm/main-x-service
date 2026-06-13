## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/workers/*` + `/api/audit/*` + `/api/health` |
| FHIR R5 (Axum) | `Worker` CRUD + search under `/fhir/Worker` (handlers implemented and **mounted** via `fhir_routes()` in `App::routes`; pinned by `tests/api_integration_test.rs::test_fhir_worker_route_is_mounted`) |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../../spec/index.md) |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

