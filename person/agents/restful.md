# API Surface — Entity-Level Orientation

The entity's API is the **service's HTTP surface**; the front-end is
its first-party consumer. Full endpoint reference (payloads,
parameters, status codes, response envelope):
[service agents/restful.md](../person-service-with-loco/agents/restful.md).
Normative summary: entity [spec §9](../spec/09-api-surface.md).

## Shape of the surface

| Tier | Mount | Notes |
|---|---|---|
| REST | `/api/persons/*`, `/api/audit/*`, `/api/health` | 15 endpoints: CRUD, search, match, check-duplicates, merge, deduplicate, export, masked, audit |
| FHIR R5 | `/fhir/Person` | Bidirectional Person resource + search params; bundles/capability statement queued |
| Docs | `/swagger-ui` | OpenAPI 3.0 via utoipa |
| Metrics | `/metrics.prom` | Prometheus text exposition |
| gRPC | — | Tonic stub (service §13 T-6) |

Envelope everywhere: `{ "success": bool, "data": …, "error": … }`.
Status idioms: `409` = duplicate candidates on create, `422` =
validation failure, `401` = (future) missing/invalid session or
PASETO token. See
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
for the cookie-session + offline PASETO model.

## Front-end consumption

| Route | Calls |
|---|---|
| `/` | health + recent audit |
| `/persons` | search |
| `/persons/new` | create (renders 409 candidates inline) |
| `/persons/[id]` (+ `/edit`, `/audit`) | get / put / delete / audit |
| `/persons/match` | match |
| `/persons/merge` | merge |

Client stack: `src/lib/api/types.ts` (wire types) →
`client.ts` (envelope-aware `ApiClient`) → `persons.ts`
(`PersonRepository`). See
[front-end spec §9](../person-front-end-with-svelte/spec/09-api-consumption.md).

## Contract rules

- The wire format is owned by the service; front-end types mirror it
  and MUST be fixed in the same change cycle when it moves.
- The front-end consumes only `/api/*` — never FHIR, never the DB
  (entity [spec FR-19](../spec/06-functional-requirements.md)).
- Endpoints not yet consumed by the UI (export, masked,
  check-duplicates, deduplicate) are queued as front-end tasks —
  see entity [spec §13 E-4](../spec/13-tasks.md) and front-end §13.
- Adding/changing an endpoint → service spec §9 + `agents/restful.md`
  + integration test; if the UI consumes it, also entity spec §9.2.
