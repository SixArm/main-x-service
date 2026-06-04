# RESTful API reference — Course Service

All endpoints mount under `/api` (per spec.md §9). The `Event`
service uses `/api/v1`; `course` does NOT — clients should call
`/api/courses/...` directly. The front-end's
[`CourseRepository`](../../course-front-end-with-svelte/src/lib/api/courses.ts)
assumes this base path.

## Library API

```rust
use course_service::api::rest::{create_router, serve, AppState};
use course_service::api::{ApiResponse, ApiError};
use course_service::models::{Course, CourseInstance, CourseIdentifier};
use course_service::matching::{CourseMatcher, MatchResult, MatchConfidence};
```

## Health

| Method | Path | Description |
|---|---|---|
| GET | `/api/health` | Health check (returns `HealthResponse`) |

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
| `q` | string | Free-text query against `name`, `alternate_names`, `description`, `keywords`, `teaches`, `course_code`, `identifier_value` |
| `limit` | usize | Default 10, capped at 100 |
| `offset` | usize | Pagination |
| `fuzzy` | bool | Multi-token `FuzzyTermQuery` with edit-distance 2 (mirrors person-service post-fix) |
| `phonetic` | bool | Soundex-augmented |
| `educational_level` | string | Filter to one EducationalLevel |
| `language` | string | BCP-47 filter |
| `provider_id` | uuid | Filter to one provider |
| `mask_sensitive` | bool | Mask instructor / provider personal identifiers |

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
| 501 | Endpoint not yet implemented (MVP scaffold) |

## Source files

- `src/api/mod.rs` — `ApiResponse`, `ApiError`
- `src/api/rest/mod.rs` — router setup, `serve`
- `src/api/rest/handlers.rs` — endpoint handlers (stubs in MVP)
- `src/api/rest/state.rs` — `AppState`
