## 10. Persistence

PostgreSQL 18+ via **sqlx**, with no ORM (since 2026-11-07). The rules, from the family's `spec/tech-stack/index.md` §5.2:

- **All SQL is in `src/db/`.** `repositories.rs` (the event repository
  and the record-integrity read), `audit.rs` (the audit log, including
  the audit-integrity read), `outbox.rs` (including the relay's poll,
  ack and retention purge), and `mod.rs` (the pool and the migrator).
  Nothing outside that directory runs a statement. `models.rs` holds
  plain row structs (`EventRow`, one per child table, `AuditLogRow`,
  `OutboxRow`), one field per column, with no behaviour.
- **Every statement is a `sqlx::query!` / `query_as!` / `query_scalar!`
  literal**, checked at compile time against the schema. The check data
  is committed in `.sqlx/` and the build reads it offline
  (`SQLX_OFFLINE=true`); `scripts/ci-check.sh test-db` runs `cargo sqlx
  prepare --check` against a freshly migrated database, so a query that
  no longer matches the schema fails CI. Regenerate after any query or
  migration change with `cargo sqlx prepare -- --all-targets` against a
  migrated database.
- **No dynamic SQL.** There is no `QueryBuilder`, `raw_sql` or
  `AssertSqlSafe` in this crate, and no `ci/sql-allowlist.txt` entry.
- **Transactions.** A helper that runs several statements takes
  `&mut PgConnection`, so it runs inside the caller's transaction. A
  create's or update's child rows and its `event_outbox` row commit in
  the same transaction as the `events` row; a merge's survivor update,
  child replacement, duplicate soft delete and both outbox rows commit
  together. An update or soft delete that matches no `events` row is an
  error.
- **Timestamps** are `chrono::DateTime<Utc>` in the row structs, as in
  the domain model. axa enables sqlx's `chrono`, and enabling `time` as
  well makes sqlx 0.9.0's macros warn on every timestamp column. The
  audit endpoints serialize `audit_log.timestamp` in the `time` crate's
  form, which clients read. Every sqlx error is `Error::Database`
  carrying sqlx's message; a unique violation is not singled out.
- **Migrations** are plain SQL in sqlx's reversible flat layout,
  `migrations/<version>_<name>.up.sql` / `.down.sql`, embedded by
  `sqlx::migrate!` (`db::MIGRATOR`) and tracked in `_sqlx_migrations`.
  `event-service db migrate | down [steps] | reset | status` and the
  boot auto-migrate run them through `axa::sqlx`. The history keeps the
  legacy `create_organizations` and `drop_dead_organizations` pair. A
  database migrated before 2026-11-07 tracks its migrations in another
  table and must be recreated.

### 10.1 Tables

- `events` — scalar columns (name, description, time window,
  status / mode / type, capacities, audit cols) plus JSONB arrays for
  `alternate_names`, `image`, `same_as`, `keywords`, `in_language`.
- `event_identifiers`, `event_locations`, `event_parties`,
  `event_offers`, `event_links`, `event_sub_events`.
  - `event_locations.latitude` / `.longitude` are **`NUMERIC`**, not
    `DOUBLE PRECISION` (§5.2.1). No scale is declared: `NUMERIC(9,6)` is
    the usual geo choice but would silently round anything finer, and the
    previous `DOUBLE PRECISION` column accepted ~15 significant digits.
    An unconstrained `NUMERIC` keeps every value a client could
    previously send, and the service caps decimal places at validation
    time rather than letting the database truncate without saying so.
    The widening migration is exact (every double has a `NUMERIC` form);
    existing rows keep the float artefacts they were stored with, since
    back-filling a rounder number would invent precision the caller never
    sent.
- `organizations`, `organization_addresses`,
  `organization_contacts`, `organization_identifiers`.
- `audit_log`.
- `review_queue` (T-15, migration `2026111000000001_create_review_queue`):
  one row per candidate pair from the batch scan, `record_id_a <
  record_id_b` under `UNIQUE (record_id_a, record_id_b)`, so a re-scan
  upserts in place; `score_breakdown` and `evidence` are JSONB; `status`
  is the lowercase wire token (`pending` / `confirmed` / `rejected` /
  `automerged`), indexed. No foreign key to `events`: a merged-away
  event stays soft-deleted, and its row stays for the record. Every
  statement is in `src/db/review_queue.rs`.

### 10.2 Extensions

Required: `pgcrypto`, `pg_trgm`.
Optional: `citext`, `unaccent`, `btree_gist` (for "no overlapping
events per resource" exclusion constraints).

### 10.3 Bulk import / export — `bulk_jobs`

Async bulk operations (§9.1) add one table, `bulk_jobs`, per the
family-wide schema in
[bulk import/export §3](../../../agents/share/bulk-import-export.md) — the
canonical column list lives there and is **not** restated. It tracks each
import/export job (`kind`, `format`, `status`, `params`, the
`rows_total`/`rows_created`/`rows_upserted`/`rows_to_review`/`rows_errored`
counts, `actor`, artifact URLs, and `expires_at` TTL), with
`UNIQUE (entity, kind, idempotency_key)` so a retried submit maps to the
same job. Jobs run on the axa `bg_pg` worker; artifacts (uploaded source,
export output, error report) live in the config-driven artifact store
(S3-compatible in deployment, local fs in dev), referenced by short-lived
access-controlled URLs. Idempotency is anchored on the §9.1 stable keys
(scheme-scoped `event_ids` `(scheme, value)` pair or the event `id`):
re-submitting a file re-upserts the same rows to the same state.

