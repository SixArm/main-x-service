## 10. Persistence

One subproject persists; the other two are deliberately stateless.

| Subproject | Persistence |
|---|---|
| service | PostgreSQL via SeaORM (below) + local Tantivy search index |
| matcher | None — pure library, no IO by specification |
| front-end | None — browser state only; every page fetches fresh from the REST API |

### 10.1 Service tables

Per [service spec §10](../thing-service-rust-crate/spec/10-persistence.md):
`things`, `thing_identifiers`, `thing_alternate_names`,
`thing_images`, `thing_same_as`, `thing_links`,
`thing_match_scores`, `audit_log`.

Required extensions: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`,
`pg_trgm`, `citext`, `unaccent`. Shared PostgreSQL guidance:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md).

### 10.2 Entity-level reference schema

[`../thing-service-schema.sql`](../thing-service-schema.sql) is the
fully-normalised relational reference schema for the entity: every
repeating domain collection (alternate names, identifiers, images,
same-as) is its own child table with a foreign key back to `things`
and an explicit ordering column. JSONB is retained only for genuinely
opaque payloads — audit before/after snapshots and the merge
transferred-data snapshot. Soft delete via `is_deleted` /
`deleted_at`; rows are never hard-deleted.

**Provenance and regeneration (honest status).** The file is
hand-maintained — there is no generator script, build step, or
`pg_dump` pipeline anywhere in the entity that produces or verifies
it, so it can drift from the service's actual migrations silently.
Current deltas against the service's two raw-SQL migrations
([`migrations/`](../thing-service-rust-crate/migrations/), executed
verbatim by the SeaORM wrappers in
[`migration/src/`](../thing-service-rust-crate/migration/src/)):

- The migrations create 7 tables (`things`,
  `thing_alternate_names`, `thing_identifiers`, `thing_images`,
  `thing_same_as`, `audit_log`, `thing_merge_records`); the reference
  schema additionally defines `thing_consents`, which no migration
  creates (the service has a `Consent` model but does not yet persist
  it).
- Neither the migrations nor the reference schema define the
  `thing_links` / `thing_match_scores` tables listed in §10.1 — those
  are spec-ahead-of-code.
- The migration DDL omits the reference schema's `CHECK` constraints
  and uses `IF NOT EXISTS` guards; column shapes otherwise agree.

Until OQ-4 (§16) settles ownership, treat the file as **descriptive
documentation**, not an executable source of truth — the migrations
are what the service actually runs.

### 10.3 Cross-subproject persistence invariants

- The database is reachable **only** through the service. The
  front-end and matcher never see a connection string.
- Audit rows and merge snapshots are append-only.
- The Tantivy index is derived state — rebuildable from PostgreSQL;
  index writes follow every database write (see
  [`agents/share/dataflow.md`](../../agents/share/dataflow.md)).
- Background jobs use Loco `BackgroundQueue` on the same PostgreSQL
  (`bg_pg`) — no external broker.
