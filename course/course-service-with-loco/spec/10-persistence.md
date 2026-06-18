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

### SeaORM time-type feature

`sea-orm` is configured with the `with-time` feature (not
`with-chrono`). This is the older-service convention carried over from
the person-service copy-adapt and is consistent across the
first-converted loco services (event / person / thing / place / worker /
course). The crate also pulls `chrono` directly for its own
domain-model timestamps (`DateTime<Utc>` on `ReviewQueueItem`, audit
entries, etc.), so both crates are present in the tree. The shared
stack note (`rust-loco-stack.md`) prescribes `with-chrono` as the
loco-service default; reconciling these older services onto
`with-chrono` is tracked as a cross-crate task (spec §13 T-17) rather
than flipped piecemeal here, since it must land uniformly across the
six services to avoid SeaORM time-type drift between siblings.

### 10.5 Bulk import / export — `bulk_jobs`

Async bulk operations (§9.2) add one table, `bulk_jobs`, per the
family-wide schema in
[bulk import/export §3](../../../agents/share/bulk-import-export.md) — the
canonical column list lives there and is **not** restated. It tracks each
import/export job (`kind`, `format`, `status`, `params`, the
`rows_total`/`rows_created`/`rows_upserted`/`rows_to_review`/`rows_errored`
counts, `actor`, artifact URLs, and `expires_at` TTL), with
`UNIQUE (entity, kind, idempotency_key)` so a retried submit maps to the
same job. Jobs run on the loco `bg_pg` worker; artifacts (uploaded source,
export output, error report) live in the config-driven artifact store
(S3-compatible in deployment, local fs in dev), referenced by short-lived
access-controlled URLs.

Course-specific notes:

- **Idempotency** is anchored on the §9.2 stable keys (a deterministic
  `CourseIdentifier` scheme — DOI / Wikidata / LOM / OER / URI / UUID — the
  provider-scoped `(provider_id, course_code)` pair, or `pid`): re-submitting
  a file re-upserts the same rows to the same state.
- **CourseInstance** rows have no independent stable key; they round-trip as
  the nested `instances` array of their parent course (§9.2) and upsert
  transactionally with the parent — never as standalone `course_instances`
  rows from a bulk job.

A related design artefact, `../../course-service-schema.sql`, lives
at the entity-directory level; see the entity spec
[§10.3](../../spec/10-persistence.md) for what it is (and is not).
Ownership is tracked as entity OQ-4.
