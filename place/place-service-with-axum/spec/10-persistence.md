## 10. Persistence

PostgreSQL 18+ via **sqlx**, with no ORM (since 2026-11-07; SeaORM
before). The rules, from the family's `spec/tech-stack/index.md` §5.2:

- **All SQL is in `src/db/`.** `mod.rs` (the place repository, the pool,
  the migrator, integrity-verification reads), `audit.rs`, `outbox.rs`,
  and `review_queue.rs`. Nothing outside that directory runs a statement.
  `models.rs` holds plain row structs (`PlaceRow`, `AuditLogRow`,
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
  row commit in the same transaction as the `places` row they describe.
- **Timestamps** are `chrono::DateTime<Utc>` in the row structs, as in
  the domain model. axa enables sqlx's `chrono`, and enabling `time` as
  well makes sqlx 0.9.0's macros warn on every timestamp column. `NUMERIC`
  maps to `bigdecimal::BigDecimal`, as before. Every sqlx error is
  `Error::Database` carrying sqlx's message; a unique violation is not
  singled out, as it was not before.
- **Migrations** are plain SQL in sqlx's reversible flat layout,
  `migrations/<version>_<name>.up.sql` / `.down.sql`, embedded by
  `sqlx::migrate!` (`db::MIGRATOR`) and tracked in `_sqlx_migrations`.
  `place-service db migrate | down [steps] | reset | status` and the
  boot auto-migrate run them through `axa::sqlx`. The SQL is the same
  SQL the SeaORM `migration/` wrapper crate ran; a `pg_dump
  --schema-only` of a database migrated each way differs only in the
  bookkeeping table (`seaql_migrations` → `_sqlx_migrations`).

### 10.1 Tables (13)

`places`, `place_addresses`, `place_geo_coordinates`,
`place_identifiers`, `place_amenities`, `place_opening_hours`,
`place_same_as`, `place_hierarchy`, `place_links`,
`organizations`, `organization_addresses`,
`place_match_scores`, `audit_log`.

> **Drift, noted not fixed (2026-08-22):** the shipped schema does not
> have a separate `place_geo_coordinates` table — geo is flattened into
> `places` as `geo_latitude_as_decimal_degrees` /
> `geo_longitude_as_decimal_degrees` / `geo_elevation_as_decimal_metres`
> (renamed from `geo_latitude` / `geo_longitude` / `geo_elevation` by
> `2026082400000001_coordinate_columns_name_their_units`), with
> `idx_places_geo` over the first two. Recorded here because the geo
> columns were being edited anyway; reconciling the rest of this list
> against the migrations is its own task.

#### Geo columns

`places.geo_latitude_as_decimal_degrees` /
`.geo_longitude_as_decimal_degrees` / `.geo_elevation_as_decimal_metres`
are **`NUMERIC`**, not `DOUBLE PRECISION` (§5.2.1). No scale is declared:
`NUMERIC(9,6)` is the usual geo choice but would silently round anything
finer, and the previous `DOUBLE PRECISION` column accepted ~15
significant digits. An unconstrained `NUMERIC` keeps every value a client
could previously send, and the service caps decimal places at validation
time rather than letting the database truncate without saying so. The
widening migration is exact (every double has a `NUMERIC` form); existing
rows keep the float artefacts they were stored with, since back-filling a
rounder number would invent precision the caller never sent.
`idx_places_geo` is rebuilt by Postgres as part of the type change.

#### `review_queue.evidence`

`review_queue.evidence` is a nullable `JSONB` holding the matcher's
evidence for the pair (§6.2.1), added by
`2026110900000001_add_review_queue_evidence`. It is written on insert
and refreshed with the score columns on the re-scan upsert; `status`
still never changes on conflict, so a pair queued `pending` before its
evidence reached `proven` stays `pending`. Rows scanned before the
migration stay `NULL` and read back as `evidence: null`; a stored value
that does not parse as evidence also reads as `null` rather than failing
the list.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`, **`postgis`** (planned use: spatial indexing
and bounding-box pre-filter on geo-radius search).

### 10.3 Bulk import / export

The execution model, job API, file formats, import dedupe semantics,
per-row error contract, and export privacy/audit posture are the
uniform family contract in
[`../../../agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md).
This section declares only the Place-specific bindings (per shared §10).

**Stable key(s)** — drive idempotent upsert on re-import, in order:

1. A scheme-scoped `PlaceIdentifier` the matcher already short-circuits
   on — **GLN** (13-digit), **OSM ID**, **GNIS**, **FIPS**, or a
   `same_as` authoritative-source URL (Wikidata / GeoNames / OSM). The
   first such identifier present wins.
2. The record `pid` (UUID `id`) when present (re-export → re-import
   round-trip).

A row carrying neither runs normal duplicate detection (matcher +
geo proximity) and routes likely duplicates to the review queue with
`provenance = import` (shared §6).

**CSV column set + flattening** (shared §5; JSONL is the lossless
reference — prefer it when fidelity matters):

- **Scalars** → one column each: `id`, `name`, `alternate_name`,
  `description`, `place_type`, `telephone`, `fax_number`, `url`,
  `is_accessible_for_free`, `public_access`, `smoking_allowed`,
  `maximum_attendee_capacity`, `contained_in_place`, `is_deleted`,
  `deleted_at`, `created_at`, `updated_at`.
- **Single nested objects** → dotted columns:
  - address → `address.street_address`, `address.address_locality`,
    `address.address_region`, `address.address_country`,
    `address.postal_code`;
  - geo → `geo.latitude_as_decimal_degrees`,
    `geo.longitude_as_decimal_degrees`, `geo.elevation_as_decimal_metres`.
- **Arrays / arrays-of-objects** → one **JSON-encoded cell** each:
  `identifiers` (type + system + value, incl. GLN / FIPS / GNIS / OSM),
  `keywords`, `alternate_names`, `same_as`, `amenity_feature`,
  `opening_hours_specification`, `place_links`.

**Export sensitivity** — a Place is largely non-personal,
low-sensitivity reference data, so the default `masking_profile` is
**light** (most fields export in full; no elevated authorisation needed
for the common case). `telephone` / `fax_number` may carry incidental
personal contact data and follow the read API's masking
([`../../../agents/share/privacy.md`](../../../agents/share/privacy.md)).
`include_soft_deleted` still defaults `false` and is gated, and **every
export is audited** (actor, filter, format, row count, masking profile,
timestamp) per the shared contract, even for a zero-row export.

