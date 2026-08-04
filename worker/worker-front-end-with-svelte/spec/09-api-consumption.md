## 9. API Consumption

The front-end binds 1:1 to the Worker Service REST surface (see [`worker-service-with-loco/AGENTS/restful.md`](../../worker-service-with-loco/AGENTS/restful.md)). Since T-22a (§8) every call is issued to the same-origin `/api/proxy/...` reverse proxy, which forwards to the Worker Service with a server-injected PASETO — the table below names the upstream Worker Service path each proxied call maps to (`/api/proxy/<path>` → `<path>` upstream):

| Endpoint | Used by |
| --- | --- |
| `GET /api/health` | Dashboard |
| `GET /api/workers/search` | `/workers` list |
| `GET /api/workers/{id}` | `/workers/[id]`, `/workers/[id]/edit`, merge preview |
| `POST /api/workers` | `/workers/new` |
| `PUT /api/workers/{id}` | `/workers/[id]/edit` |
| `DELETE /api/workers/{id}` | Detail page (soft-delete button) |
| `POST /api/workers/match` | `/workers/match` |
| `POST /api/workers/check-duplicates` | (available — not yet routed) |
| `POST /api/workers/merge` | `/workers/merge` |
| `POST /api/workers/deduplicate` | `/review` (the scan button; destructive-classed, never a page-load side effect) |
| `GET /api/workers/review-queue` | `/review` (loads the stored queue on mount) |
| `POST /api/workers/review-queue/{id}/decision` | `/review` (drag-to-decide) |
| `GET /api/workers/{id}/audit` | `/workers/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/workers/{id}/masked` | (available — not yet routed) |
| `GET /api/workers/{id}/export` | (available — not yet routed) |
| `GET /api/workers/{id}/links` | `/workers/[id]` (Cross-service links panel, T-23) |
| `POST /api/workers/{id}/links` | `/workers/[id]` (assert a link) |
| `DELETE /api/workers/{id}/links/{link_id}` | `/workers/[id]` (withdraw a link) |

The BFF also calls the **authentication-service** directly (not via this proxy, and not via `WorkerRepository`): `GET /api/auth/magic-link/{token}` (`/verify`), `POST /api/auth/magic-link` (`/signin`), `POST /api/auth/token` (session → PASETO exchange, on every proxied call and on sign-out), and `POST /api/auth/signout` (the layout's sign-out form action) — see `src/lib/server/auth.ts`.

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `WorkerRepository` return unwrapped `data`.

