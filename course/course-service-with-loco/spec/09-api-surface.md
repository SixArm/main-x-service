## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (loco.rs controllers on Axum) | endpoints under `/api/courses/*` + `/api/courses/{id}/instances/*` + `/api/audit/*` + `/api/health`, registered as a loco `Routes` table with prefix `/api` |
| Ops (loco built-ins) | `GET /_health` (DB + queue readiness) and `GET /_ping` (liveness), from loco's default routes — for orchestration probes, outside `/api` |
| gRPC (Tonic) | Out of MVP scope. |
| Docs | Swagger UI at `/swagger-ui`, raw OpenAPI 3 JSON at `/api-docs/openapi.json` (utoipa). |
| Metrics | `GET /metrics.prom` — Prometheus text-exposition format (`text/plain; version=0.0.4`), mounted at the application **root** (not under `/api`), public (no bearer token needed). Counters: `course_created_total`, `course_updated_total`, `course_deleted_total`, `course_merged_total`, plus a labelled `http_requests_total{path,status}`. |

**Record ids on create.** `POST /api/courses` mints the `id`: omit the
field (serde default) **or** send the all-zeros UUID, which the handler
treats as "you pick" and replaces. Sending nil used to be stored
verbatim, so the first such create claimed the nil id and every later one
failed on the primary key with a `500` — the same nil-sentinel handling
the event service already had. A **non-nil** `id` is still honoured, so a
caller may supply its own.

All `/api` endpoints return `{ "success": bool, "data": …, "error": … }`.
HTTP status codes follow REST conventions: `409` for duplicate
detection on create, `422` for validation failure, `501` only for
`GET /api/courses` (list-all-without-search, intentionally
unimplemented — clients should call `/api/courses/search` with an
empty `q` for the same effect).

`GET /api/health` is the service's own envelope-wrapped health
endpoint (kept for front-end and API-client parity); the loco
`/_health` / `/_ping` pair serves container orchestration.

### 9.2 Bulk import / export

The async, job-based bulk contract is fixed family-wide in
[bulk import/export](../../../agents/share/bulk-import-export.md) (execution
model on `bg_pg`, the five endpoints, JSONL/CSV/Parquet codecs,
upsert-by-stable-key + dedupe-to-review, the per-row error report, and
export masking + audit). This section declares only the **course-specific**
bits; the shared doc is the source of truth for everything else.

The five endpoints (shared doc §4) mount under the course resource:

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/courses/import` | `202 {job_id}` — body: `format`, `dedupe_mode`, `dry_run`; file upload |
| `GET` | `/api/courses/import/{id}` | Job status + counts + `errors_url` + `review_url` |
| `POST` | `/api/courses/export` | `202 {job_id}` — body: `format`, `filter`, `fields`, `include_soft_deleted`, `masking_profile` |
| `GET` | `/api/courses/export/{id}` | Job status + `download_url` |
| `GET` | `/api/courses/bulk-jobs` | List (filter by `kind`/`status`); `GET .../{id}` for one |

**Stable key(s) for upsert** (shared doc §6, §10). A row upserts in place
when it carries either:

- a **deterministic, scheme-scoped `CourseIdentifier`** — the same schemes
  the matcher short-circuits on via R-0 (`IdentifierScheme::is_deterministic`,
  §5): **DOI, Wikidata, LOM, OER, URI, UUID** — keyed by `(property_id,
  value)`; or
- the **provider-scoped course code** — the matcher's R-1 short-circuit pair
  `(provider_id, course_code)` (a bare `CourseCode` / `LmsCourseId` /
  `PlatformSlug` / `ISCED` / `ROR` is provider-scoped, so it is only a stable
  key when paired with its provider); or
- the record **`pid`** (the course UUID) when present in the row.

A row with neither runs the normal duplicate detection (§9.1 `check-duplicates`
/ create path, review queue), routing likely duplicates to the review queue
with `provenance = import`.

**CourseInstance sub-resource.** `Course` is the template; `CourseInstance` is
its sub-resource (offerings under `/api/courses/{id}/instances`). Bulk carries
instances **nested**: in JSONL each line is the full `Course` wire type with
its `instances` array inline (lossless, round-trips with schedule / mode /
instructors / capacity / enrollment window); in CSV the `instances` array is a
single **JSON-encoded cell** (shared doc §5), like the other arrays. Instances
have no independent stable key — they upsert as part of their parent course
row (matched on the parent's stable key), never as standalone records; a
separate instance-only import is not offered in v1.

**CSV column set + flattening** (shared doc §5). CSV is the operator /
spreadsheet format and is lossy for deep nesting — steer fidelity-sensitive
loads to **JSONL** (the lossless reference). Flat columns:

- **scalar** (one column each): `pid`, `name`, `course_code`,
  `educational_level`, `learning_resource_type`, `number_of_credits`,
  `course_prerequisites`, `financial_aid_eligible`,
  `educational_credential_awarded`, `total_historical_enrollment`, `active`;
- **single nested object** → dotted columns: the issuing provider
  (`provider.id`, `provider.name`);
- **arrays / arrays-of-objects** → a single **JSON-encoded cell** each:
  `identifiers` (`CourseIdentifier` PropertyValues), `alternate_names`,
  `keywords`, `teaches`, `available_language`, `syllabus_sections` (the
  hierarchical TOC, with nested `sub_sections`), `credentials`, `instances`
  (the CourseInstance sub-resource, see above), and `links` (course-to-course
  cross-references).

**Export sensitivity** (shared doc §8). Course rows are generally
**low-sensitivity** (schema.org/Course templates are largely public catalog
data), so default masking is **light** — the existing `mask_course` profile
(clears `provider_id`, instance `instructor_ids`, masks `instructor_names`,
§5/FR-16) is the masked default; full / unmasked output requires a
`masking_profile` selecting elevated authorisation and must never reveal more
than the caller could read one record at a time. `include_soft_deleted`
defaults `false` and is gated. **Every export is still audited** per the shared
contract (actor, filter, format, row count, masking profile, timestamp —
written even for a zero-row export). The existing single-record GDPR export
(FR-15) is the single-subject special case of this machinery (filter = one
`pid`).
