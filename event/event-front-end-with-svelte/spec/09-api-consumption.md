## 9. API Consumption

The front-end binds 1:1 to the Event Service REST surface (see [`event-service-with-loco/agents/restful.md`](../../event-service-with-loco/agents/restful.md)). URLs are version-free (`/api/events`, not `/api/v1/events` — Event is the family's [API-versioning reference](../../../agents/share/api-versioning.md)); the BFF proxy (`src/routes/api/proxy/[...path]/+server.ts`) sends `Accepts-version: 1.0` on every forwarded request instead.

| Endpoint | Used by |
| --- | --- |
| `GET /api/health` | Dashboard |
| `GET /api/events/search` | `/events` list |
| `GET /api/events/{id}` | `/events/[id]`, `/events/[id]/edit`, merge preview |
| `POST /api/events` | `/events/new` |
| `PUT /api/events/{id}` | `/events/[id]/edit` |
| `DELETE /api/events/{id}` | Detail page (soft-delete button) |
| `POST /api/events/match` | `/events/match` |
| `POST /api/events/check-duplicates` | (available — not yet routed) |
| `POST /api/events/merge` | `/events/merge` |
| `POST /api/events/deduplicate` | (available — not yet routed; deferred to roadmap) |
| `GET /api/events/{id}/audit` | `/events/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/events/{id}/masked` | (available — not yet routed) |
| `GET /api/events/{id}/export` | (available — not yet routed) |

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `EventRepository` return unwrapped `data`.

