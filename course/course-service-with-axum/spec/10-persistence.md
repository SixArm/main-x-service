## 10. Persistence

PostgreSQL via **sqlx**, with no ORM (since 2026-11-07, T-31; SeaORM
before). The schema source of truth is the hand-written, numbered SQL
under `migrations/`, in sqlx's reversible flat layout
(`<version>_<name>.up.sql` / `.down.sql`), embedded by
`sqlx::migrate!` as `db::MIGRATOR` and tracked in `_sqlx_migrations`.
`course-service db migrate | down [steps] | reset | status` and the
boot auto-migrate (`auto_migrate`, on by default in development) run
them through `axa::sqlx`. The SQL is the same SQL the former SeaORM
`migration/` wrapper crate ran; a `pg_dump --schema-only` of a database
migrated each way is identical apart from the bookkeeping table
(`seaql_migrations` → `_sqlx_migrations`).

The rules, from the family's `spec/tech-stack/index.md` §5.2:

- **All SQL is in `src/db/`.** `mod.rs` (the course and instance
  repository, the review queue, the merge, the pool, the migrator, the
  integrity-verification read), `audit.rs`, and `outbox.rs` (including
  the relay's poll, ack and retention purge). Nothing outside that
  directory runs a statement. `models.rs` holds plain row structs
  (`CourseRow`, `CourseInstanceRow`, `CourseTextValueRow`,
  `CourseCredentialRow`, `ReviewQueueRow`, `AuditLogRow`, `OutboxRow`),
  one field per column, with no behaviour.
- **Every statement is a `sqlx::query!` / `query_as!` / `query_scalar!`
  literal**, checked at compile time against the schema. The check data
  is committed in `.sqlx/` and the build reads it offline
  (`SQLX_OFFLINE=true`); `scripts/ci-check.sh test-db` runs `cargo sqlx
  prepare --check` against a freshly migrated database, so a query that
  no longer matches the schema fails CI. Regenerate after any query or
  migration change with `cargo sqlx prepare -- --all-targets` against a
  migrated database.
- **No dynamic SQL.** There is no `QueryBuilder`, `raw_sql` or
  `AssertSqlSafe` in this crate. The one optional filter, the review
  queue's `status`, is a bound parameter: `($1::text IS NULL OR status = $1)`.
- **Transactions.** A helper that runs several statements takes
  `&mut PgConnection`, so it runs inside the caller's transaction. A
  course write's child rows and its `course_outbox` row commit in the
  same transaction as the `courses` row; an instance write's child rows
  commit with the instance row; a merge's survivor update, child
  replacement, duplicate soft delete and both outbox rows commit
  together.
- **Full-row updates stay full-row.** A course or instance update
  writes every column, `created_at` included, as the SeaORM full-row
  `ActiveModel` update did; a soft delete writes only its tombstone
  columns. An update or soft delete that matches no row is still an
  error (`Error::Database`, `500`).
- **Timestamps** are `chrono::DateTime<Utc>` in the row structs, as in
  the domain model, so the persistence boundary no longer converts to
  the `time` crate (only the outbox's pure envelope→row mapping still
  takes a `time::OffsetDateTime`). axa enables sqlx's `chrono`, and
  enabling `time` as well makes sqlx 0.9.0's macros warn on every
  timestamp column. Every sqlx error is `Error::Database` carrying
  sqlx's message; a unique violation is not singled out, as it was not
  before.

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
- `course_match_scores` — historical match scores / review queue. Its
  nullable `evidence` JSONB column (migration
  `2026110900000001_add_review_evidence`, §6.1) holds the matcher's
  evidence for the pair; older rows are `NULL`.
- `course_merge_records` — merge audit trail with transferred-data snapshot.
- `audit_log` — HIPAA / FERPA-style trail for who / what / when.

### Time types

Until T-31 this section recorded why `sea-orm` ran with `with-time`
rather than `with-chrono` (T-17). With SeaORM gone the question is
closed: the row structs use `chrono`, the same type as the domain
model, and sqlx's `time` feature stays off (see the timestamp rule
above). The `time` crate remains a direct dependency only for the
outbox mapping's `OffsetDateTime` argument.

### 10.5 Bulk import / export — `bulk_jobs`

Async bulk operations (§9.2) add one table, `bulk_jobs`, per the
family-wide schema in
[bulk import/export §3](../../../agents/share/bulk-import-export.md) — the
canonical column list lives there and is **not** restated. It tracks each
import/export job (`kind`, `format`, `status`, `params`, the
`rows_total`/`rows_created`/`rows_upserted`/`rows_to_review`/`rows_errored`
counts, `actor`, artifact URLs, and `expires_at` TTL), with
`UNIQUE (entity, kind, idempotency_key)` so a retried submit maps to the
same job. Jobs run on the axa `worker` feature (Postgres-backed
background jobs — renamed from `bg_pg` in the axa → 1.0.1
migration this crate already carries); artifacts (uploaded source,
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
