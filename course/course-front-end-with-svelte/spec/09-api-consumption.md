## 9. API Consumption

The front-end binds 1:1 to the Course Service REST surface (see [`course-service-with-axum/agents/restful.md`](../../course-service-with-axum/agents/restful.md)):

| Endpoint | Used by |
| --- | --- |
| `GET /api/health` | Dashboard |
| `GET /api/courses/search` | `/courses` list |
| `GET /api/courses/{id}` | `/courses/[id]`, `/courses/[id]/edit`, merge preview |
| `POST /api/courses` | `/courses/new` |
| `PUT /api/courses/{id}` | `/courses/[id]/edit` |
| `DELETE /api/courses/{id}` | Detail page (soft-delete button) |
| `POST /api/courses/match` | `/courses/match` |
| `POST /api/courses/check-duplicates` | (available — not yet routed) |
| `POST /api/courses/merge` | `/courses/merge` |
| `POST /api/courses/deduplicate` | (available — not yet routed; deferred to roadmap) |
| `GET /api/courses/{id}/audit` | `/courses/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/courses/{id}/masked` | (available — not yet routed) |
| `GET /api/courses/{id}/export` | (available — not yet routed) |

Response handling is centralised in `ApiClient`: a success is the bare JSON body, and an RFC 9457 problem becomes an `ApiError` (`detail` → message, `code`, `errors` → `details`). Per-endpoint methods on `CourseRepository` return the body.

