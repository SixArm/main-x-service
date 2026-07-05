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

Key-set configuration (issuer/audience from `THING_TOKEN_ISSUER` /
`THING_TOKEN_AUDIENCE`, defaults `authentication-service` /
`main-x-service`):

- `THING_PASETO_KEYS_URL` set (non-blank) — the key-set JSON is
  fetched over HTTP **once at boot** (async, in `after_routes`, before
  the routers/middleware capture the verifier) from the auth service
  (normally `/.well-known/paseto-keys`). A successful fetch **wins**
  over `THING_PASETO_KEYS`; a failed fetch warn-logs and falls back to
  the env path. No refresh loop (rotation re-fetch is roadmap, §15).
- Unset/blank — the key set comes from the `THING_PASETO_KEYS` env
  var; absent/unparseable ⇒ an empty reject-all key set.

Either way the service **always boots**.

Blanket enforcement is implemented **default-off**: when
`THING_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
case-insensitive), every `/api/*` route requires a valid bearer token
except the public `/api/health`. Root-level `/_health`, `/_ping`,
`/api-docs/openapi.json`, `/swagger-ui*`, and `/metrics.prom` sit
outside the `/api` scope and stay public. The flag is read once at
construction — restart to change. Roles are the only open T-4 item
(boot-time HTTP key fetch landed 2026-07-04). Family contract:
[jwt-enforcement](../../../agents/share/jwt-enforcement.md).

