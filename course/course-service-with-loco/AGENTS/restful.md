# RESTful API reference — Course Service

All endpoints mount under `/api` (per spec.md §9): `courses_routes()`
is a loco `Routes` table with prefix `/api`, registered in
`App::routes` alongside loco's default ops routes. The `Event`
service uses `/api`; `course` does NOT — clients should call
`/api/courses/...` directly. The front-end's
[`CourseRepository`](../../course-front-end-with-svelte/src/lib/api/courses.ts)
assumes this base path.

API URLs are version-free; select the version with the `Accepts-version`
header (default `1.0`), stamped back on the response — see
[`agents/share/api-versioning.md`](../../../agents/share/api-versioning.md).
Implemented by `version::require_version_mw` (`src/api/rest/version.rs`),
layered on both router surfaces (`create_router` + `App::after_routes`).

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
| GET | `/api/whoami` | Echoes the verified bearer's claims (T-15) — public when `COURSE_REQUIRE_AUTH` is off. |
| GET | `/_health` | loco built-in readiness (DB + queue) — plain, no envelope |
| GET | `/_ping` | loco built-in liveness — plain, no envelope |
| GET | `/metrics.prom` | Prometheus metrics (T-16) — mounted at the application **root** (not under `/api`), public, `text/plain; version=0.0.4`. NOT envelope-wrapped. |

`GET /metrics.prom` worked example:

```bash
curl -s http://localhost:8084/metrics.prom
```

```text
# HELP course_created_total Total course records created.
# TYPE course_created_total counter
course_created_total 12
# HELP course_merged_total Total course merges performed.
# TYPE course_merged_total counter
course_merged_total 3
```

Counters: `course_created_total`, `course_updated_total`,
`course_deleted_total`, `course_merged_total` (incremented one-per-success
in the create / update / delete / merge handlers) plus a labelled
`http_requests_total{path,status}` registered for a future request-path
middleware (emits no sample line until a label combination is observed).
Configure the scraper with `metrics_path: /metrics.prom`.

## Course CRUD

| Method | Path | Description |
|---|---|---|
| POST | `/api/courses` | Create. Returns `201 Course`. `409 MatchResult[]` on duplicate, `422` on validation. |
| GET | `/api/courses/{id}` | Get one course (includes nested `instances`). |
| PUT | `/api/courses/{id}` | Replace. Excludes `instances` — those have their own endpoints. |
| DELETE | `/api/courses/{id}` | Soft-delete (sets `deleted_at`). |

On a duplicate, `POST /api/courses` returns `409 Conflict` with the
ranked `ScoredCandidate[]` under `error.details` (a deterministic
short-circuit — shared `provider_id` + `course_code`, DOI, Wikidata, or
`same_as` URL — pins the score at `1.0`; FR-1 / FR-20):

```jsonc
// HTTP/1.1 409 Conflict
{
  "success": false,
  "data": null,
  "error": {
    "code": "DUPLICATE",
    "message": "Potential duplicate course(s) detected",
    "details": [
      { "course_id": "9c2b…", "name": "Intro to Computer Science",
        "course_code": "CS101", "score": 1.0, "quality": "certain" }
    ]
  }
}
```

`POST /api/courses/check-duplicates` returns the same `ScoredCandidate[]`
shape without writing.

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

## Compliance / integrity (T-24, default off)

| Method | Path | Description |
|---|---|---|
| GET | `/api/records/verify?limit=200` | Reassembles each `Course` record and recomputes its stored SHA-256 / SHA3-256 / MAC against the live pre-image; reports `verified` / `mismatch` / `mac_absent` / `unhashed` per row. |
| GET | `/api/audit/verify?limit=200` | Same, over `audit_log` rows. |

With no `COURSE_INTEGRITY_MAC_KEY` (or `..._KEY_FILE`) set, no MAC is
written and both endpoints report `mac_absent` rather than a mismatch.
See [`spec/12-compliance.md`](../spec/12-compliance.md) and
`src/compliance/`.

## FHIR (non-standard `Basic` surface)

No FHIR R5 resource models an educational course, so the crate serves
a **deliberately non-R5** best-effort surface wrapping a course as the
FHIR `Basic` resource (`code = course`), per
[`agents/share/fhir.md`](../../../agents/share/fhir.md) §3. Mounted by
`fhir_routes()` ([`src/api/rest/fhir.rs`](../src/api/rest/fhir.rs),
prefix `/fhir`): `GET /fhir/metadata` (CapabilityStatement, which
states the non-standard shape explicitly), `POST /fhir/Basic` (create),
`GET /fhir/Basic` (search), and `GET` / `PUT` / `DELETE
/fhir/Basic/{id}`. Responses are `application/fhir+json`; every
non-2xx body is a FHIR `OperationOutcome`.

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
