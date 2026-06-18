## 9. API Consumption

The front-end binds 1:1 to the Person Service REST surface (see [`person-service-with-loco/AGENTS/restful.md`](../../person-service-with-loco/AGENTS/restful.md)):

| Endpoint | Used by |
| --- | --- |
| `GET /api/health` | Dashboard |
| `GET /api/persons/search` | `/persons` list |
| `GET /api/persons/{id}` | `/persons/[id]`, `/persons/[id]/edit`, merge preview |
| `POST /api/persons` | `/persons/new` |
| `PUT /api/persons/{id}` | `/persons/[id]/edit` |
| `DELETE /api/persons/{id}` | Detail page (soft-delete button) |
| `POST /api/persons/match` | `/persons/match` |
| `POST /api/persons/check-duplicates` | (available — not yet routed) |
| `POST /api/persons/merge` | `/persons/merge` |
| `POST /api/persons/deduplicate` | (available — not yet routed; deferred to roadmap) |
| `GET /api/persons/{id}/audit` | `/persons/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/persons/{id}/masked` | (available — not yet routed) |
| `GET /api/persons/{id}/export` | (available — not yet routed) |

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `PersonRepository` return unwrapped `data`.

