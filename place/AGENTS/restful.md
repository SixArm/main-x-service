# RESTful API — Place entity

The service owns the REST surface; the front-end is its reference
consumer. This page is the entity-level orientation — full endpoint
reference with request/response examples:
[service AGENTS/restful.md](../place-service-rust-crate/AGENTS/restful.md);
inventory: service [spec §9](../place-service-rust-crate/spec/09-api-surface.md).

## Shape of the surface

- **15 endpoints** under `/api/places/*`, `/api/audit/*`,
  `/api/health`: CRUD, search, geo-radius (`/api/places/nearby`),
  match, duplicate check, merge, batch deduplicate, GDPR export,
  masked view, audit queries.
- **Envelope:** every response wraps in the shared success/error
  envelope; the front-end unwraps it in `src/lib/api/client.ts`.
- **Status conventions:** `409` = duplicate candidates on create
  (payload carries the matches + breakdowns); `422` = validation
  failure (field-level details); REST-conventional otherwise.
- **Docs:** OpenAPI 3.0 + Swagger UI at `/swagger-ui`.
- **Metrics:** Prometheus at `/metrics.prom` (HTML dashboard at
  `/metrics`).
- **gRPC:** Tonic stub only (service spec §13 T-4).
- **Auth:** none yet — JWT via the authentication entity is queued
  (service spec §13 T-8; entity [spec §13](../spec/13-tasks.md) E-5).

## Front-end consumption

The SPA calls the service from the browser using
`PUBLIC_API_BASE_URL` (default `http://localhost:8080`). Per-route
endpoint usage table: entity [spec §9.2](../spec/09-api-surface.md);
client layering (types → client → repository):
[front-end AGENTS.md](../place-front-end-with-svelte/AGENTS.md).

## Entity-contract rules

- The **wire format is the contract**: a change to a request/response
  body is a service-spec change *and* a front-end types change in the
  same cycle (entity FR-20).
- Match responses MUST carry the per-component breakdown end to end
  (entity FR-21) — don't flatten it in a handler or drop it in the UI.
- Known drift (partially resolved 2026-06-13): the code serves
  `POST /api/places/check-duplicates` and the service docs
  (AGENTS/restful.md, spec §6.4) now agree. The **front-end client
  still calls `/api/places/duplicates`**
  (`src/lib/api/places.ts`) and will 404 against the live service —
  entity [spec §13](../spec/13-tasks.md) E-1.
