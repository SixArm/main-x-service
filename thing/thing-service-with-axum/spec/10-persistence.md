## 10. Persistence

PostgreSQL 18+ via **sqlx**, with no ORM (since 2026-11-07; SeaORM
before). The rules, from the family's `spec/tech-stack/index.md` §5.2:

- **All SQL is in `src/db/`.** `mod.rs` (the thing repository, the pool,
  the migrator, the integrity-verification read), `audit.rs`,
  `outbox.rs` (including the relay's poll, ack and retention purge), and
  `review_queue.rs`. Nothing outside that directory runs a statement.
  `models.rs` holds plain row structs (`ThingRow`, `AuditLogRow`,
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
  `AssertSqlSafe` in this crate. The one optional filter, the review
  queue's `status`, is a bound parameter: `($1::text IS NULL OR status = $1)`.
- **Transactions.** A helper that runs several statements takes
  `&mut PgConnection` (or `axa::sqlx::Db`), so it runs inside the
  caller's transaction. Each write's child rows and its `event_outbox`
  row commit in the same transaction as the `things` row they describe;
  a merge's survivor update, child replacement, duplicate soft delete
  and both outbox rows commit together.
- **Timestamps** are `chrono::DateTime<Utc>` in the row structs, as in
  the domain model. axa enables sqlx's `chrono`, and enabling `time` as
  well makes sqlx 0.9.0's macros warn on every timestamp column. Every
  sqlx error is `Error::Database` carrying sqlx's message; a unique
  violation is not singled out, as it was not before.
- **Migrations** are plain SQL in sqlx's reversible flat layout,
  `migrations/<version>_<name>.up.sql` / `.down.sql`, embedded by
  `sqlx::migrate!` (`db::MIGRATOR`) and tracked in `_sqlx_migrations`.
  `thing-service db migrate | down [steps] | reset | status` and the
  boot auto-migrate run them through `axa::sqlx`. The SQL is the same
  SQL the SeaORM `migration/` wrapper crate ran; a `pg_dump
  --schema-only` of a database migrated each way is identical apart
  from the bookkeeping table (`seaql_migrations` → `_sqlx_migrations`).

### 10.1 Tables

`things`, `thing_identifiers`, `thing_alternate_names`,
`thing_images`, `thing_same_as`, `thing_links`, `thing_match_scores`,
`audit_log`.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`.

### 10.3 Bulk import / export

Thing adopts the family-wide bulk contract verbatim — async `bg_pg`
jobs, the `bulk_jobs` table, the five-endpoint API, JSONL/CSV/Parquet
codecs, idempotent upsert-by-key, the keyless-row → duplicate-detection
→ review-queue path, the per-row error report, and export
masking + audit. See
[`../../../agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md)
for the full design; only the Thing-specific bits are declared here.

**Stable key(s) (upsert key, §6 of the shared doc).** A Thing row
upserts in place when it carries one of the matcher's deterministic
short-circuit identifiers (a `ThingIdentifier` whose `property_id` is
`Doi`, `Isbn`, `Issn`, `Gtin`, `Mpn`, `SerialNumber`, or `Uuid`),
matched as the scheme-scoped pair `(property_id, value)` — the same key
the matcher uses for deterministic equality. The registry `pid` (UUID
`id`) is also accepted as a stable key for round-tripping a prior
export. Rows with only non-deterministic identifiers (`Sku`, `Uri`,
`Custom`) or none carry no stable key and run normal duplicate
detection.

**CSV column set + flattening (§5 of the shared doc).** Scalar
schema.org/Thing properties are one flat column each: `id`, `name`,
`description`, `disambiguating_description`, `additional_type`, `url`,
`main_entity_of_page`, `owner`, `subject_of`, `created_at`,
`updated_at`, `is_deleted`, `deleted_at`. Repeated and
array-of-object fields are single **JSON-encoded cells**:
`identifiers` (the `Vec<ThingIdentifier>`), `alternate_names`,
`images`, `same_as`, `potential_action`, and `links` (the
`thing_links` relationships). JSONL remains the lossless reference and
is preferred whenever nested fidelity matters.

**Export sensitivity (§8 of the shared doc).** A Thing is generally
non-personal, low-sensitivity reference data, so the default
`masking_profile` is light and full export needs no elevated
authorisation in the common case. The exception is a Thing that is
itself subject to a data-protection regime (e.g. a personally-owned
record carrying a `Consent`, §5.3) — those follow the standard masked
default and elevated-authorisation-for-full rule. `include_soft_deleted`
defaults `false` and is gated. Every export is audited regardless of
sensitivity, per the shared contract.

