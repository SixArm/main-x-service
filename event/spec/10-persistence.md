## 10. Persistence

One subproject persists; two do not. Detail:
[service spec §10](../event-service-with-loco/spec/10-persistence.md).

### 10.1 Service — PostgreSQL 18+ via SeaORM

| Table | Holds |
|---|---|
| `events` | Scalar columns + JSONB arrays (`alternate_names`, `image`, `same_as`, `keywords`, `in_language`) |
| `event_identifiers` | Typed, system-qualified external IDs |
| `event_locations` | `Location` union rows |
| `event_parties` | Organizers / performers / attendees / sponsors / funders / contributors |
| `event_offers` | Pricing tiers |
| `event_links`, `event_sub_events` | Merge links + hierarchy |
| `organizations` (+ addresses / contacts / identifiers) | Embedded organization records |
| `audit_log` | Old/new JSON, user ID, IP, user agent, timestamp |

Extensions — required: `pgcrypto`, `pg_trgm`; optional: `citext`,
`unaccent`, `btree_gist` (no-overlap exclusion constraints).

An entity-level schema artifact exists; see §10.5 — the migrations in
the service crate are authoritative, not the artifact.

### 10.2 Search index

Tantivy, owned by the service, on the local filesystem
(`SEARCH_INDEX_PATH`, default `./data/search_index`). Synchronised
with database writes; rebuilt per instance. Externalising it is a
roadmap item (§15).

### 10.3 Matcher — none

Pure library: no IO, no storage, no state. Configuration
(`MatchConfig`) is `Serialize + Deserialize` so tunings can live in a
caller-owned file — the service owns where that file lives.

### 10.4 Front-end — none

Stateless SPA. No browser persistence of registry data; the only
configuration is `PUBLIC_API_BASE_URL` in `.env`. No tokens are
stored today (no auth — ET-5); when SSO lands, token handling becomes
a front-end persistence decision recorded in its own spec.

### 10.5 Entity-level schema artifact — `event-service-schema.sql`

[`../event-service-schema.sql`](../event-service-schema.sql) (313
lines, 18 `CREATE TABLE` statements) is a **hand-written, fully
normalized reference design** for the Event Service schema — JSONB
string-list columns refactored into a tagged `event_text_values`
child table, the polymorphic `Location` union as one table with a
`kind` discriminator, and party roles as one table tagged by `role`.
It targets PostgreSQL 15+.

It is **not a generated export of the live migrations**, and no
regeneration script exists. Known divergences from the service
crate's `migrations/` (audited 2026-06-13):

- The snapshot carries tables no migration creates:
  `event_references`, `event_consents`, `event_merge_records`,
  `event_review_queue`, `organization_text_values` — designed-ahead
  persistence for workflows the service currently keeps outside
  dedicated tables.
- The migrations create a partitioned `audit_log`
  (`audit_log_y2024m12 PARTITION OF audit_log`); the snapshot's
  `audit_log` is unpartitioned.
- The `event_text_values` normalization **does** match: migration
  `2026060800000001_normalize_event_text_values` implemented the
  snapshot's design for the five string-list fields.

Treat the migrations as authoritative for what runs; treat the
snapshot as the normalization roadmap. If the remaining snapshot
tables get implemented, do it as migrations first and then either
regenerate this file from a live database (e.g.
`pg_dump --schema-only`) or retire it.
