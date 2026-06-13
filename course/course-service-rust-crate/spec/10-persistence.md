## 10. Persistence

PostgreSQL via SeaORM. The schema source of truth is the
hand-written, numbered SQL under `migrations/` (`up.sql` /
`down.sql` per step). The sibling `migration/` crate is the loco
SeaORM `Migrator`: each Rust migration wraps the corresponding SQL
pair via `include_str!`, so loco can run the same SQL through
`auto_migrate: true` (on by default in development config) or
explicitly via `cargo loco db migrate`.

Tables (9 core + 7 collection child tables from the
`normalize_course_collections` migration):

- `providers` — issuing organisations.
- `provider_text_values` — provider string collections (alternate names, same-as).
- `courses` — Course template (scalar fields).
- `course_text_values` — tagged single-table store for the Course
  string collections (alternate_names, keywords, teaches, …).
- `course_identifiers` — typed external identifiers.
- `course_links` — course-to-course cross-references.
- `course_credentials` — educational / occupational credentials awarded.
- `course_instances` — specific offerings (FK to courses; schedule flattened onto the row).
- `course_instance_languages` / `course_instance_instructors` /
  `course_instance_sessions` — instance collections.
- `syllabus_sections` — hierarchical (parent_id self-FK).
- `course_syllabus_text_values` — per-section teaches / resource strings.
- `course_match_scores` — historical match scores / review queue.
- `course_merge_records` — merge audit trail with transferred-data snapshot.
- `audit_log` — HIPAA / FERPA-style trail for who / what / when.

A related design artefact, `../../course-service-schema.sql`, lives
at the entity-directory level; see the entity spec
[§10.3](../../spec/10-persistence.md) for what it is (and is not).
Ownership is tracked as entity OQ-4.
