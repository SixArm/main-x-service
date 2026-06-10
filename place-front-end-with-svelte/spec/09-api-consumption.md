## 9. API Consumption

The front-end binds 1:1 to the Place Service REST surface (see [`place-service-rust-crate/AGENTS/restful.md`](../../place-service-rust-crate/AGENTS/restful.md)):

| Endpoint | Used by |
| --- | --- |
| `GET /api/health` | Dashboard |
| `GET /api/places/search` | `/places` list |
| `GET /api/places/{id}` | `/places/[id]`, `/places/[id]/edit`, merge preview |
| `POST /api/places` | `/places/new` |
| `PUT /api/places/{id}` | `/places/[id]/edit` |
| `DELETE /api/places/{id}` | Detail page (soft-delete button) |
| `POST /api/places/match` | `/places/match` |
| `POST /api/places/check-duplicates` | (available — not yet routed) |
| `POST /api/places/merge` | `/places/merge` |
| `POST /api/places/deduplicate` | (available — not yet routed; deferred to roadmap) |
| `GET /api/places/{id}/audit` | `/places/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/places/{id}/masked` | (available — not yet routed) |
| `GET /api/places/{id}/export` | (available — not yet routed) |

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `PlaceRepository` return unwrapped `data`.

