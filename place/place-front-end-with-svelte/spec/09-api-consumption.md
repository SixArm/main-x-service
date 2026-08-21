## 9. API Consumption

The front-end binds 1:1 to the Place Service REST surface (see [`place-service-with-loco/agents/restful.md`](../../place-service-with-loco/agents/restful.md)). Since T-22 (BFF auth), the browser calls these paths through the same-origin `/api/proxy/[...path]` reverse proxy rather than the Place Service directly — the SvelteKit server exchanges the session for a short-lived PASETO and forwards with `Authorization: Bearer <paseto>` (see `AGENTS.md` "Authentication — the BFF pattern"). The table below still names the upstream Place Service path each route ultimately reaches.

| Endpoint | Used by |
| --- | --- |
| `GET /api/health` | Dashboard |
| `GET /api/places/search` | `/places` list |
| `GET /api/places/{id}` | `/places/[id]`, `/places/[id]/edit`, merge preview |
| `POST /api/places` | `/places/new` |
| `PUT /api/places/{id}` | `/places/[id]/edit` |
| `DELETE /api/places/{id}` | Detail page (soft-delete button) |
| `POST /api/places/match` | `/places/match` |
| `POST /api/places/check-duplicates` | (available — not yet routed; T-17) |
| `POST /api/places/merge` | `/places/merge` |
| `POST /api/places/deduplicate` | `/review` (scan button; destructive-classed, never a page-load side effect) |
| `GET /api/places/review-queue` | `/review` (loads the stored queue on mount) |
| `POST /api/places/review-queue/{id}/decision` | `/review` (drag-to-decide) |
| `GET /api/places/{id}/audit` | `/places/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/places/{id}/masked` | (available — not yet routed; T-19) |
| `GET /api/places/{id}/export` | (available — not yet routed; T-20) |

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `PlaceRepository` return unwrapped `data`.

