# RESTful API reference — Course Service

All endpoints mount under `/api` (per spec.md §9): `courses_routes()`
is a loco `Routes` table with prefix `/api`, registered in
`App::routes` alongside loco's default ops routes. The `Event`
service uses `/api/v1`; `course` does NOT — clients should call
`/api/courses/...` directly. The front-end's
[`CourseRepository`](../../course-front-end-with-svelte/src/lib/api/courses.ts)
assumes this base path.

## Library API

```rust
use course_service::app::App;                  // loco Hooks (boot, routes, after_routes)
use course_service::api::rest::{create_router, courses_routes, AppState};
use course_service::api::{ApiResponse, ApiError};
use course_service::models::{Course, CourseInstance, CourseIdentifier};
use course_service::matching::{CourseMatcher, MatchResult, MatchConfidence};
```

The binary boots through the loco CLI (`cargo loco start`);
`create_router` is retained for the `tower::oneshot` integration
tests.

## Health & ops

| Method | Path | Description |
|---|---|---|
| GET | `/api/health` | Service health check (envelope-wrapped `HealthResponse`) |
| GET | `/_health` | loco built-in readiness (DB + queue) — plain, no envelope |
| GET | `/_ping` | loco built-in liveness — plain, no envelope |

## Course CRUD

| Method | Path | Description |
|---|---|---|
| POST | `/api/courses` | Create. Returns `201 Course`. `409 MatchResult[]` on duplicate, `422` on validation. |
| GET | `/api/courses/{id}` | Get one course (includes nested `instances`). |
| PUT | `/api/courses/{id}` | Replace. Excludes `instances` — those have their own endpoints. |
| DELETE | `/api/courses/{id}` | Soft-delete (sets `deleted_at`). |

## Search

| Method | Path | Description |
|---|---|---|
| GET | `/api/courses/search` | Full-text search |

`SearchQuery` parameters:

| Field | Type | Notes |
|---|---|---|
| `q` | string | Free-text query against `name`, `alternate_names`, `keywords`, `teaches`, identifier values. Empty / whitespace falls back to `repository.list` (paginated). |
| `limit` | u64 | Default 20 |
| `offset` | u64 | Pagination |
| `fuzzy` | bool | Multi-token `FuzzyTermQuery` with edit distance 2 (one fuzzy clause per alphanumeric run). |
| `phonetic` | bool | Accepted for API parity with sibling services; currently a no-op in the search path. The matcher's Soundex bonus on `name_score` fires on `/match` and `/check-duplicates`. |
| `mask_sensitive` | bool | Accepted for API parity; masking is exposed at `/api/courses/{id}/masked` (FR-16). |

Response normalises to `{ items: Course[], total: usize }` per
spec.md FR-19.

## Matching & deduplication

| Method | Path | Description |
|---|---|---|
| POST | `/api/courses/match` | Find candidate matches for a request course |
| POST | `/api/courses/check-duplicates` | Real-time duplicate check |
| POST | `/api/courses/merge` | Merge two courses |
| POST | `/api/courses/deduplicate` | Batch dedup scan |

Blocking: by `name` (FuzzyTermQuery, multi-token) AND/OR by
`(provider_id, course_code)` when both present.

## CourseInstance sub-resource

| Method | Path | Description |
|---|---|---|
| GET | `/api/courses/{id}/instances` | List all instances (ordered `schedule.start_date DESC NULLS LAST`) |
| POST | `/api/courses/{id}/instances` | Create a new instance |
| GET | `/api/courses/{id}/instances/{instance_id}` | Get one instance |
| PUT | `/api/courses/{id}/instances/{instance_id}` | Replace |
| DELETE | `/api/courses/{id}/instances/{instance_id}` | Soft-delete |

## Privacy

| Method | Path | Description |
|---|---|---|
| GET | `/api/courses/{id}/export` | GDPR Article-15 export |
| GET | `/api/courses/{id}/masked` | Masked view |

## Audit

| Method | Path | Description |
|---|---|---|
| GET | `/api/courses/{id}/audit` | Audit log for one course (and its child instances + syllabus) |
| GET | `/api/audit/recent` | Recent audit activity |

## Response envelope

```json
{ "success": true, "data": {…}, "error": null }
```

Error envelope:

```json
{
  "success": false,
  "data": null,
  "error": { "code": "…", "message": "…", "details": null }
}
```

## HTTP status codes

| Code | Meaning |
|---|---|
| 200 | OK |
| 201 | Created |
| 204 | Deleted |
| 400 | Bad request |
| 404 | Not found |
| 409 | Duplicate detected (on create) |
| 422 | Validation error |
| 500 | Internal error |
| 501 | Endpoint not yet implemented (MVP scaffold) — only `GET /api/courses` (list-all-without-search) remains 501; the rest are wired. |

## OpenAPI

The full spec is rendered at runtime by [`utoipa`](https://docs.rs/utoipa):

- Interactive: `GET /swagger-ui`
- Raw JSON: `GET /api-docs/openapi.json`

## Source files

- `src/app.rs` — loco `Hooks`: registers `courses_routes()` in
  `App::routes`, builds `AppState` into the `AppContext` shared
  store and layers Swagger UI + CORS in `after_routes`
- `src/api/mod.rs` — `ApiResponse`, `ApiError`
- `src/api/rest/mod.rs` — `courses_routes()` (loco `Routes`, prefix
  `/api`), `create_router` (test-only Axum router), `ApiDoc`
  aggregator
- `src/api/rest/handlers.rs` — endpoint handlers (FR-1..FR-9 + FR-14..FR-18 wired)
- `src/api/rest/state.rs` — `AppState` + `FromRef<AppContext>` bridge
