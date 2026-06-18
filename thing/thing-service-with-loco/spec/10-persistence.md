## 10. Persistence

PostgreSQL 18+ via SeaORM.

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

