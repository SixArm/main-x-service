## 9. API Consumption

The front-end binds 1:1 to the Thing Service REST surface (see [`thing-service-with-loco/agents/restful.md`](../../thing-service-with-loco/agents/restful.md)). Calls do not go straight to the service: the browser calls the same-origin BFF proxy (`/api/proxy/...`, `src/routes/api/proxy/[...path]/+server.ts`), which exchanges the session for a short-lived PASETO and forwards to `THING_API_URL` (§8 Architecture). `ThingRepository`/`ApiClient` are unaware of this — their configured base URL is just `/api/proxy`.

| Endpoint | Used by |
| --- | --- |
| `GET /api/health` | Dashboard |
| `GET /api/things/search` | `/things` list |
| `GET /api/things/{id}` | `/things/[id]`, `/things/[id]/edit`, merge preview |
| `POST /api/things` | `/things/new` |
| `PUT /api/things/{id}` | `/things/[id]/edit` |
| `DELETE /api/things/{id}` | Detail page (soft-delete button) |
| `POST /api/things/match` | `/things/match` |
| `POST /api/things/check-duplicates` | (available — not yet routed; T-17) |
| `POST /api/things/merge` | `/things/merge` |
| `POST /api/things/deduplicate` | `/review` (Run scan button; destructive-classed, never a page-load side effect) |
| `GET /api/things/review-queue` | `/review` (loads the stored queue on mount — a safe read) |
| `POST /api/things/review-queue/{id}/decision` | `/review` (drag a card into Confirmed / Rejected) |
| `GET /api/things/{id}/audit` | `/things/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/things/{id}/masked` | (available — not yet routed) |
| `GET /api/things/{id}/export` | (available — not yet routed) |

Auth (not against the Thing Service): `POST /api/auth/magic-link`, `GET /api/auth/magic-link/{token}`, `POST /api/auth/token`, `POST /api/auth/signout` against `AUTH_API_URL` — see `src/lib/server/auth.ts` and §8.

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `ThingRepository` return unwrapped `data`.

