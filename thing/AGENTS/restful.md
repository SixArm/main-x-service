# RESTful API — Thing Entity orientation

The service exposes 15 REST endpoints; the front-end consumes them
1:1. Full endpoint reference with payloads and library API:
[service AGENTS/restful.md](../thing-service-rust-crate/AGENTS/restful.md).
Endpoint-by-route consumption map:
[front-end spec §9](../thing-front-end-with-svelte/spec/09-api-consumption.md).

## Surface shape

| Group | Endpoints |
|---|---|
| Health | `GET /api/health` |
| CRUD | `POST /api/things`, `GET/PUT/DELETE /api/things/{id}` |
| Search | `GET /api/things/search` (`q`, `limit`, `offset`, `fuzzy`, `mask_sensitive`) |
| Duplicate workflow | `POST /api/things/match`, `POST /api/things/check-duplicates`, `POST /api/things/merge`, `POST /api/things/deduplicate` |
| Privacy | `GET /api/things/{id}/export` (GDPR), `GET /api/things/{id}/masked` |
| Audit | `GET /api/things/{id}/audit`, `GET /api/audit/recent`, `GET /api/audit/user` |

Conventions (shared:
[agents/share/restful.md](../../agents/share/restful.md)):

- Standard JSON response envelope; per-endpoint methods in the
  front-end's `ThingRepository` return unwrapped `data`.
- `409 Conflict` with candidate matches when create detects a
  duplicate; `422` with field details on validation failure.
- OpenAPI via utoipa; Swagger UI at `/swagger-ui`; Prometheus
  text exposition at `/metrics.prom`.
- No FHIR surface for this entity; gRPC is a Tonic stub (service §13
  T-3); no auth enforcement yet (entity §13 T-6).

## Known doc drift

Code and OpenAPI use `POST /api/things/check-duplicates`; the service
`AGENTS/restful.md` endpoint table says `POST /api/things/duplicates`.
Trust the code / OpenAPI until entity spec
[§13 T-2](../spec/13-tasks.md) lands.

## Front-end consumption

Routes `/`, `/things`, `/things/new`, `/things/[id]`,
`/things/[id]/edit`, `/things/[id]/audit`, `/things/match`,
`/things/merge` — see entity spec
[§9.2](../spec/09-api-surface.md) for the route → endpoint table.
`check-duplicates`, `deduplicate`, `masked`, and `export` are served
by the API but not yet routed in the UI (entity §13 T-7).

Envelope handling lives in
[`src/lib/api/client.ts`](../thing-front-end-with-svelte/src/lib/api/client.ts);
endpoint methods in
[`src/lib/api/things.ts`](../thing-front-end-with-svelte/src/lib/api/things.ts).
