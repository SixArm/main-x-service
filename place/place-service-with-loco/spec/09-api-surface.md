## 9. API Surface

Complete reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 14 endpoints under `/api/places/*` + `/api/audit/recent` + `/api/health` |
| Auth (Axum) | `GET /api/whoami` — echo the verified PASETO bearer-token claims (`401` without a valid token) |
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

Authentication is opt-in per handler by default: taking an `AuthUser`
argument requires a valid `Authorization: Bearer <paseto>` token,
verified offline (PASETO `v4.public`, Ed25519) against the
auth-service published key set (see §13 T-8).

Key-set configuration (issuer/audience from `PLACE_TOKEN_ISSUER` /
`PLACE_TOKEN_AUDIENCE`, defaults `authentication-service` /
`main-x-service`):

- `PLACE_PASETO_KEYS_URL` set (non-blank) — the key-set JSON is
  fetched over HTTP **once at boot** (async, in `after_routes`, before
  the routers/middleware capture the verifier) from the auth service
  (normally `/.well-known/paseto-keys`). A successful fetch **wins**
  over `PLACE_PASETO_KEYS`; a failed fetch warn-logs and falls back to
  the env path. No refresh loop (rotation re-fetch is roadmap, §15).
- Unset/blank — the key set comes from the `PLACE_PASETO_KEYS` env
  var; absent/unparseable ⇒ an empty reject-all key set.

Either way the service **always boots**.

Blanket enforcement: when the default-off `PLACE_REQUIRE_AUTH` env
flag is truthy (`1`/`true`/`yes`/`on`, case-insensitive; read at
router construction — restart to change), an Axum middleware on both
router surfaces requires a valid bearer token on **every** route
except the public allow-list — `/api/health`, `/_health`, `/_ping`,
`/api-docs/openapi.json`, `/swagger-ui*`, `/metrics.prom` (constants
`auth::PUBLIC_PATHS` / `PUBLIC_PATH_PREFIXES`). Unauthenticated
requests to any other path get `401`.

Authorization (ABAC, inside the same guard — so only when
`PLACE_REQUIRE_AUTH` is on): the request's action is derived from the
HTTP method plus this crate's destructive named POSTs
(`auth::DESTRUCTIVE_POST_SUFFIXES`: `/merge`, `/deduplicate`,
`/import`), and the shared engine in `authentication-verifier` 0.3
evaluates the configured policy (`PLACE_ABAC_POLICY` inline JSON /
`PLACE_ABAC_POLICY_FILE`; unset/unparsable ⇒ built-in default: any
authenticated subject reads, `access=write` writes, `access=admin`
adds delete/merge/deduplicate, `svc=true` does everything) over the
token's `attrs` claim. A valid token the policy denies gets `403`
with the deciding rule; see
[authorization-attributes](../../../agents/share/authorization-attributes.md).

