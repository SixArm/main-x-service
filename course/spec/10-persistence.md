## 10. Persistence

All durable state in the entity lives in the **service**. The matcher
is a pure in-memory library; the front-end is a stateless SPA (its
only configuration is `.env` / `PUBLIC_API_BASE_URL`, and it holds no
HTTP state in global stores by ground rule).

### 10.1 Service stores

| Store | Contents |
|---|---|
| PostgreSQL (SeaORM, migrations in the crate) | 16 tables: 9 core (`providers`, `courses`, `course_identifiers`, `course_links`, `course_instances`, `syllabus_sections`, `course_match_scores` (review queue), `course_merge_records`, `audit_log`) + 7 collection child tables from the `normalize_course_collections` migration (`course_text_values`, `course_credentials`, `course_instance_languages`, `course_instance_instructors`, `course_instance_sessions`, `course_syllabus_text_values`, `provider_text_values`) |
| Tantivy index (local directory) | Full-text / fuzzy search + blocking queries; rebuilt from the database; reader-reload after every commit |
| In-memory event bus | `CourseEvent` stream (MVP; durable bus is roadmap §15) |

Detail: service [spec §10](../course-service-rust-crate/spec/10-persistence.md).

### 10.2 Entity-level persistence rules

- The database is the system of record; the search index is a
  derived, rebuildable projection. No subproject other than the
  service may touch either.
- Soft delete only: `deleted_at` set, rows retained for the audit
  trail (§12 erasure handling).
- Merge never destroys data: the duplicate is soft-deleted and the
  transferred payload is snapshotted in `course_merge_records`.
- `syllabus_sections` currently ships as schema + JSONB without a
  read/write API (service roadmap v0.4) — tracked in §13.
- PostgreSQL conventions (extensions, version) follow
  [`agents/share/postgresql.md`](../../agents/share/postgresql.md).

### 10.3 Known artefact: `course-service-schema.sql`

[`../course-service-schema.sql`](../course-service-schema.sql) sits
at the entity directory level. What it is, on inspection:

- A **hand-written design document** for the fully-normalized
  target schema (JSONB collections refactored into child tables),
  authored in the same change that produced the service's
  `normalize_course_collections` migration. It is not a `pg_dump`
  output and there is **no regeneration story** — nothing produces
  or refreshes it from the migrations or from a live database.
- The migration implements the design but **deviates in two table
  names**: the live schema keeps the pre-existing
  `syllabus_sections` and `course_match_scores`, where the file
  says `course_syllabus_sections` and `course_review_queue`. The
  file has already drifted, exactly as OQ-4 predicted.
- The service crate's `migrations/` (run through the `migration/`
  loco Migrator crate) remain the executable source of truth; treat
  this file as a historical design snapshot until OQ-4 (delete /
  generate / formally adopt) is resolved.
