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
| FR-9 | `POST /api/courses/deduplicate` MUST scan the index in batch, queue uncertain matches, and auto-merge a pair only when its score is at or above `auto_merge_threshold` **and** its evidence is `proven` (§6.1); a `consistent` or `contradicted` pair MUST be queued as `Pending` whatever its score. |
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
| FR-20 | A deterministic identifier match (DOI, Wikidata, LOM, OER id, URI, UUID — `IdentifierType::is_deterministic`; **not** LMS course-id or a bare course code, both provider-scoped) MUST short-circuit scoring to `1.0` with `confidence = High`. |
| FR-21 | `name` is required, non-empty after trim. |
| FR-22 | `course_code`, when present, MUST be 1-100 chars. |
| FR-23 | `number_of_credits`, when present, MUST be a non-negative integer. |
| FR-24 | `in_language` entries MUST be valid BCP-47 codes (length check; full validation deferred). |
| FR-25 | `url`, `image[*]`, `same_as[*]`, identifier `url`s MUST start with `http://` or `https://`. |
| FR-26 | `CourseInstance.schedule.end_date` MUST be ≥ `schedule.start_date` when both set. |
| FR-27 | `CourseInstance.enrollment_closes` MUST be ≥ `enrollment_opens` when both set. |
| FR-28 | `CourseInstance.maximum_attendee_capacity` MUST be ≥ `enrolled_count` when both set. |

### 6.1 Match evidence

Per [`match-evidence.md`](../../../agents/share/match-evidence.md)
(§10 step 4; spec §13 T-33):

- **Every scored pair carries the matcher's `evidence`**
  (`weight_bits`, `fields_compared`, `contradictions`, `strength`,
  `components`): each `ScoredCandidate` from FR-1's `409` body, FR-6
  and FR-7, and each FR-9 review item. `score` is unchanged. Evidence
  holds component and identifier-scheme names and bits only, never a
  field value, so it is returned unmasked on every path.
- **Auto-merge only on `proven`.** FR-9 is the only automatic merge in
  this service, and it is gated as FR-9 states. `POST
  /api/courses/merge` (FR-8) is an operator's explicit request and is
  not gated. Nothing about FR-1's `409` changes.
- **Persisted.** FR-9 stores the evidence on every review-queue row it
  writes and refreshes it on a re-scan; `GET /api/courses/review-queue`
  returns it. Rows written before the column existed return
  `evidence: null`. A pair FR-9 merges is also stored, as `AutoMerged`,
  and returned in the scan's `auto_merged_items`.
- **Value frequencies** (match-evidence.md §7). The matcher's one
  frequency component is the course `name`. The service counts each
  active course's name in the matcher's `frequency_key` form over at
  most 200 000 records (`FREQUENCY_SCAN_CAP`; above it the table is a
  sample of the newest records) and divides by the population. Below
  `MIN_FREQUENCY_POPULATION` (1 000) records it supplies no
  frequencies, because a share from a small population is noise. FR-9
  counts in a first pass over the same repository pages it then scans.
  FR-1, FR-6 and FR-7 use a table cached in `AppState`, rebuilt lazily
  once older than ten minutes: a stale table keeps serving while one
  background task rebuilds it, and only the first request after boot
  waits for one bounded scan.
