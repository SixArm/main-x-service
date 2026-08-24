## 10. Persistence

PostgreSQL 18+ via SeaORM.

### 10.1 Tables (13)

`places`, `place_addresses`, `place_geo_coordinates`,
`place_identifiers`, `place_amenities`, `place_opening_hours`,
`place_same_as`, `place_hierarchy`, `place_links`,
`organizations`, `organization_addresses`,
`place_match_scores`, `audit_log`.

> **Drift, noted not fixed (2026-08-22):** the shipped schema does not
> have a separate `place_geo_coordinates` table — geo is flattened into
> `places` as `geo_latitude` / `geo_longitude` / `geo_elevation`, with
> `idx_places_geo` over the first two. Recorded here because the geo
> columns were being edited anyway; reconciling the rest of this list
> against the migrations is its own task.

#### Geo columns

`places.geo_latitude` / `.geo_longitude` / `.geo_elevation` are
**`NUMERIC`**, not `DOUBLE PRECISION` (§5.2.1). No scale is declared:
`NUMERIC(9,6)` is the usual geo choice but would silently round anything
finer, and the previous `DOUBLE PRECISION` column accepted ~15
significant digits. An unconstrained `NUMERIC` keeps every value a client
could previously send, and the service caps decimal places at validation
time rather than letting the database truncate without saying so. The
widening migration is exact (every double has a `NUMERIC` form); existing
rows keep the float artefacts they were stored with, since back-filling a
rounder number would invent precision the caller never sent.
`idx_places_geo` is rebuilt by Postgres as part of the type change.

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
  - geo → `geo.latitude`, `geo.longitude`, `geo.elevation`.
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

