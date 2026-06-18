## 6. Functional Requirements

| ID | Requirement |
|---|---|
| FR-1 | `POST /api/courses` MUST validate (FR-21..FR-25), MUST short-circuit duplicate-detection on a deterministic identifier match, MUST return `201` + `Course` on success, `409` + `MatchResult[]` on probable duplicate, `422` + field errors on validation failure. |
| FR-2 | `GET /api/courses/{id}` MUST return the full Course including its `instances` collection. |
| FR-3 | `PUT /api/courses/{id}` MUST replace the Course (excluding `instances`, which have their own endpoints). |
| FR-4 | `DELETE /api/courses/{id}` MUST soft-delete (sets `deleted_at`, hides from search). |
| FR-5 | `GET /api/courses/search` MUST support `q`, `limit`, `offset`, `fuzzy`, `phonetic`, `educational_level`, `language`, `provider_id`, `mask_sensitive`. |
| FR-6 | `POST /api/courses/match` MUST score the request against blocked candidates and return ranked `MatchResult[]`. |
| FR-7 | `POST /api/courses/check-duplicates` MUST run the same blocker + scorer as FR-1 but never write. |
| FR-8 | `POST /api/courses/merge` MUST fold the duplicate into the main course (transfer identifiers, instances, syllabus, links), then soft-delete the duplicate, record a `MergeRecord`. |
| FR-9 | `POST /api/courses/deduplicate` MUST scan the index in batch, queue uncertain matches, and auto-merge above `auto_merge_threshold`. |
| FR-10 | `GET /api/courses/{id}/instances` MUST list all instances ordered by `schedule.start_date DESC NULLS LAST`. |
| FR-11 | `POST /api/courses/{id}/instances` MUST create a new instance, validate FR-26..FR-28. |
| FR-12 | `PUT /api/courses/{id}/instances/{instance_id}` MUST replace the instance. |
| FR-13 | `DELETE /api/courses/{id}/instances/{instance_id}` MUST soft-delete. |
| FR-14 | `GET /api/courses/{id}/audit` MUST return audit log entries (newest first) for the Course AND its child instances/syllabus. |
| FR-15 | `GET /api/courses/{id}/export` MUST return the GDPR Article-15 portability JSON (full record). |
| FR-16 | `GET /api/courses/{id}/masked` MUST return the Course with `provider_id`, `instructor_ids`, `instructor_names`, and any `Personal Data` identifier values masked. |
| FR-17 | Every CRUD operation MUST emit an audit-log entry. |
| FR-18 | Every CRUD operation MUST emit a Course event (`CourseCreated`, `CourseUpdated`, `CourseDeleted`, `CourseMerged`, `CourseInstanceCreated`, `CourseInstanceUpdated`, `CourseInstanceDeleted`) on the streaming bus. |
| FR-19 | Search responses MUST normalise to `{ items, total }` (the front-end expects the `items` key). |
| FR-20 | A deterministic identifier match (DOI, Wikidata, LMS id, OER id, URI, UUID) MUST short-circuit scoring to `1.0` with `confidence = High`. |
| FR-21 | `name` is required, non-empty after trim. |
| FR-22 | `course_code`, when present, MUST be 1-100 chars. |
| FR-23 | `number_of_credits`, when present, MUST be a non-negative integer. |
| FR-24 | `in_language` entries MUST be valid BCP-47 codes (length check; full validation deferred). |
| FR-25 | `url`, `image[*]`, `same_as[*]`, identifier `url`s MUST start with `http://` or `https://`. |
| FR-26 | `CourseInstance.schedule.end_date` MUST be ≥ `schedule.start_date` when both set. |
| FR-27 | `CourseInstance.enrollment_closes` MUST be ≥ `enrollment_opens` when both set. |
| FR-28 | `CourseInstance.maximum_attendee_capacity` MUST be ≥ `enrolled_count` when both set. |

