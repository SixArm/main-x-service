## 9. API Consumption

The front-end binds 1:1 to the Thing Service REST surface (see [`thing-service-rust-crate/AGENTS/restful.md`](../../thing-service-rust-crate/AGENTS/restful.md)):

| Endpoint | Used by |
| --- | --- |
| `GET /api/health` | Dashboard |
| `GET /api/things/search` | `/things` list |
| `GET /api/things/{id}` | `/things/[id]`, `/things/[id]/edit`, merge preview |
| `POST /api/things` | `/things/new` |
| `PUT /api/things/{id}` | `/things/[id]/edit` |
| `DELETE /api/things/{id}` | Detail page (soft-delete button) |
| `POST /api/things/match` | `/things/match` |
| `POST /api/things/check-duplicates` | (available — not yet routed) |
| `POST /api/things/merge` | `/things/merge` |
| `POST /api/things/deduplicate` | (available — not yet routed; deferred to roadmap) |
| `GET /api/things/{id}/audit` | `/things/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/things/{id}/masked` | (available — not yet routed) |
| `GET /api/things/{id}/export` | (available — not yet routed) |

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `ThingRepository` return unwrapped `data`.

