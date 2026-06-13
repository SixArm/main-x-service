# RESTful API — Event Entity

One HTTP surface, owned by the service, versioned under **`/api/v1`**,
consumed by the front-end (and any agency integrator). Full endpoint
reference with request/response shapes:
[`event-service-rust-crate/AGENTS/restful.md`](../event-service-rust-crate/AGENTS/restful.md).
Front-end endpoint-to-route map:
[front-end spec §9](../event-front-end-with-svelte/spec/09-api-consumption.md).

## Shape of the surface

| Group | Endpoints |
|---|---|
| Health | `GET /health` |
| CRUD | `POST /events` (409 on duplicates, 422 on validation), `GET/PUT/DELETE /events/{id}` |
| Search | `GET /events/search` (`q`, `fuzzy`, `date_from`/`date_to`, `event_status`, `event_type`, `mask_sensitive`, `limit`/`offset`) |
| Match & dedup | `POST /events/match`, `/events/check-duplicates`, `/events/merge`, `/events/deduplicate` |
| Privacy | `GET /events/{id}/masked`, `GET /events/{id}/export` |
| Audit | `GET /events/{id}/audit`, `GET /audit/recent`, `GET /audit/user` |
| Meta | Swagger UI `/swagger-ui`, Prometheus `/metrics.prom` |
| Stubs | `/fhir/Event/*` → `501`; gRPC stub |

Envelope: `{ "success": bool, "data": …, "error": { code, message, details } }`.
Status codes: 200/201/204/400/404/409/422/500/501 per the service
reference.

## Entity-level conventions

- **Versioned surface.** Breaking wire changes require `/api/v1` →
  `/api/v2`; that decision is entity-level
  ([spec §8.3](../spec/08-architecture.md)).
- **The front-end binds 1:1** to these endpoints via `ApiClient` +
  `EventRepository` — no other transport, no direct DB access.
- **No auth yet** (entity ET-5): every endpoint is currently
  unauthenticated; deploy only behind trusted networks. When SSO
  lands, RS256 JWTs from the
  [authentication entity](../../authentication/) will be verified
  against its JWKS.
- **409 is a feature.** Duplicate-detected create returns the
  candidate list; the operator UI surfaces it inline and offers the
  merge route. Treat 409 handling as part of the contract, not an
  error path to suppress.
- Shared REST conventions: [`agents/share/restful.md`](../../agents/share/restful.md).
