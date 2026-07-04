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
published key set (see §13 T-4).

Blanket enforcement is implemented **default-off**: when
`THING_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
case-insensitive), every `/api/*` route requires a valid bearer token
except the public `/api/health`. Root-level `/_health`, `/_ping`,
`/api-docs/openapi.json`, `/swagger-ui*`, and `/metrics.prom` sit
outside the `/api` scope and stay public. The flag is read once at
construction — restart to change. Roles + fetching the published key
set over HTTP remain open (§13 T-4). Family contract:
[jwt-enforcement](../../../agents/share/jwt-enforcement.md).

