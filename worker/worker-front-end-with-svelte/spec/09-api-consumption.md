## 9. API Consumption

The front-end binds 1:1 to the Worker Service REST surface (see [`worker-service-rust-crate/AGENTS/restful.md`](../../worker-service-rust-crate/AGENTS/restful.md)):

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
| `POST /api/workers/deduplicate` | (available — not yet routed; deferred to roadmap) |
| `GET /api/workers/{id}/audit` | `/workers/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/workers/{id}/masked` | (available — not yet routed) |
| `GET /api/workers/{id}/export` | (available — not yet routed) |

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `WorkerRepository` return unwrapped `data`.

