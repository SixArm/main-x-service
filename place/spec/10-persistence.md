## 10. Persistence

Persistence is owned entirely by the **service**; the matcher is
stateless by contract (no IO — matcher
[spec §8](../place-matcher-rust-crate/spec/08-determinism-and-safety.md)),
and the front-end is a stateless SPA (no client-side persistence
beyond transient page state; front-end
[spec §10](../place-front-end-with-svelte/spec/10-persistence.md)).

### 10.1 Service: PostgreSQL

PostgreSQL 18+ via SeaORM. 13 tables (service
[spec §10](../place-service-rust-crate/spec/10-persistence.md)):
`places`, `place_addresses`, `place_geo_coordinates`,
`place_identifiers`, `place_amenities`, `place_opening_hours`,
`place_same_as`, `place_hierarchy`, `place_links`, `organizations`,
`organization_addresses`, `place_match_scores`, `audit_log`.

Extensions (shared baseline:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md)):
`pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`, `citext`,
`unaccent`, and **`postgis`** — required by the schema, with spatial
indexing (GiST + `ST_DWithin`) planned for geo-radius search (service
spec §13 T-1).

#### `place-service-schema.sql` (entity-root reference artifact)

[`../place-service-schema.sql`](../place-service-schema.sql) is a
hand-maintained, fully-normalized reference schema (8 tables:
`places` with PostalAddress/GeoCoordinates flattened on, plus
`place_keywords`, `place_identifiers`, `place_amenity_features`,
`place_opening_hours`, `place_consents`, `place_merge_records`,
`audit_log`). It follows the same flattened design as the SQL the
service actually applies — the SeaORM migration crate
(`place-service-rust-crate/migration/`) `include_str!`s the raw SQL
in `place-service-rust-crate/migrations/*/up.sql` — but it is **not
generated from** the migrations and has drifted: the live migrations
create 7 tables (no `place_consents`; consents are an in-memory
model only), omit the artifact's lat/lon CHECK constraints, and
carry 14 of its 16 indexes. Note neither set matches the 13-table
list in §10.1 (which follows service spec §10) — reconciling the
three is open.

There is **no regeneration story**: no script derives this file from
the migrations or from a live database. Treat the migrations as
authoritative for the deployed schema and this artifact as design
documentation; a generation step (e.g. `pg_dump --schema-only` after
`cargo run -- db migrate`) would close the gap.

### 10.2 Service: Tantivy index

Full-text index over `name`, `alternate_name`, `identifiers`, address
components, `place_type`. Synchronised with database writes; stored on
local disk (`SEARCH_INDEX_PATH`). At governmental scale the index
needs a persistent volume per replica and a rebuild story —
[§15](15-roadmap.md).

### 10.3 Event stream

`InMemoryEventPublisher` today — events do not survive a restart and
cannot be consumed by peers. Durable Fluvio publisher is service spec
§13 T-3 / entity [§13](13-tasks.md) E-6.

### 10.4 Entity-level invariant

The database is reachable **only** through the service. Neither the
front-end nor any peer entity may query the place tables directly;
integration is REST (and, later, the event stream). This keeps audit
logging (FR-15) airtight.
