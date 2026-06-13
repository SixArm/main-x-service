## 9. API Consumption

The front-end binds 1:1 to the Event Service REST surface (see [`event-service-rust-crate/AGENTS/restful.md`](../../event-service-rust-crate/AGENTS/restful.md)):

| Endpoint | Used by |
| --- | --- |
| `GET /api/v1/health` | Dashboard |
| `GET /api/v1/events/search` | `/events` list |
| `GET /api/v1/events/{id}` | `/events/[id]`, `/events/[id]/edit`, merge preview |
| `POST /api/v1/events` | `/events/new` |
| `PUT /api/v1/events/{id}` | `/events/[id]/edit` |
| `DELETE /api/v1/events/{id}` | Detail page (soft-delete button) |
| `POST /api/v1/events/match` | `/events/match` |
| `POST /api/v1/events/check-duplicates` | (available — not yet routed) |
| `POST /api/v1/events/merge` | `/events/merge` |
| `POST /api/v1/events/deduplicate` | (available — not yet routed; deferred to roadmap) |
| `GET /api/v1/events/{id}/audit` | `/events/[id]/audit` |
| `GET /api/v1/audit/recent` | Dashboard |
| `GET /api/v1/events/{id}/masked` | (available — not yet routed) |
| `GET /api/v1/events/{id}/export` | (available — not yet routed) |

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `EventRepository` return unwrapped `data`.

