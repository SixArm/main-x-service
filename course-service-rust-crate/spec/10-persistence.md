## 10. Persistence

PostgreSQL via SeaORM. Migrations under `migrations/` (numbered SQL
`up.sql` / `down.sql`). Tables:

- `providers` — issuing organisations.
- `courses` — Course template (scalar fields + JSONB collections).
- `course_identifiers` — typed external identifiers.
- `course_links` — course-to-course cross-references.
- `course_instances` — specific offerings (FK to courses).
- `syllabus_sections` — hierarchical (parent_id self-FK).
- `course_match_scores` — historical match scores / review queue.
- `course_merge_records` — merge audit trail with transferred-data snapshot.
- `audit_log` — HIPAA / FERPA-style trail for who / what / when.

