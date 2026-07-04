## 9. API Surface

Complete reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/things/*` + `/api/audit/*` + `/api/health` |
| Auth (Axum) | `GET /api/whoami` — echo the verified PASETO bearer-token claims (`401` without a valid token) |
| Metrics | `GET /metrics.prom` at the application **root** (not under `/api`) — Prometheus text-exposition (`text/plain; version=0.0.4`); scrape with `metrics_path: /metrics.prom` |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../../spec/index.md) |
| Docs | Swagger UI at `/swagger-ui` |

This crate does **not** expose a FHIR R5 surface — Things are not a
FHIR-resource concern.

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

Authentication is opt-in per handler: taking an `AuthUser` argument
requires a valid `Authorization: Bearer <paseto>` token, verified
offline (PASETO `v4.public`, Ed25519) against the auth-service
published key set (see §13 T-4; blanket enforcement is the open part
of T-4).

