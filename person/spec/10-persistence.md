## 10. Persistence

All durable state in the person entity lives in **one place: the
service crate**. This is an architectural invariant, not an accident.

### 10.1 Ownership

| Subproject | Persistent state |
|---|---|
| person-service | PostgreSQL (SeaORM, 12+ tables) + Tantivy search index on disk |
| person-matcher | **None** — pure library; no IO, no files, no global state |
| person-front-end | **None durable** — in-memory page state only; no localStorage of person data |

### 10.2 Service persistence (summary)

PostgreSQL 18+ via SeaORM with migrations. Tables: `persons`,
`person_names`, `person_identifiers`, `person_addresses`,
`person_contacts`, `person_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `person_match_scores`, `audit_log`.
Extensions, pooling, and soft-delete mechanics:
[service spec §10](../person-service-with-loco/spec/10-persistence.md)
and [agents/share/postgresql.md](../../agents/share/postgresql.md).

#### Schema artifact: `person-service-schema.sql`

A reference DDL file sits at the entity root:
[`person-service-schema.sql`](../person-service-schema.sql) (17
tables). It is **hand-written, not a `pg_dump`**: it states the
fully-normalized target schema — former JSONB child collections
(documents, emergency contacts, photos) as dedicated child tables,
JSONB retained only for audit snapshots and match-score breakdowns.

Relationship to the migrations: the service crate's
`migrations/*/up.sql` files (wrapped by its SeaORM `migration/`
crate) are the **operative** DDL. The migration chain (create
`patient_*` → rename to `person_*` → 2026-06 child-table additions)
converges on the same shape for 16 of the 17 tables. Known
divergences as of 2026-06-13:

- `person_consents` exists only in the schema file; no migration
  creates it (consent is model-level only in the service today).
- The file's header targets "PostgreSQL 15+" while the family
  standard is PostgreSQL 18.

There is **no regeneration story**: no script produces this file from
the migrations or a live database, and no CI check detects drift. It
is a hand-maintained design snapshot and can silently go stale —
tracked as task E-10 (§13).

### 10.3 Entity-level invariants

- Soft delete only: `active = false`; no subproject may hard-delete a
  person row (§5.5).
- The Tantivy index is a **derived** store — it MUST be rebuildable
  from PostgreSQL (bulk re-index is supported); it is never the
  system of record.
- The front-end MUST NOT cache person data beyond the page lifetime;
  every view re-fetches through the REST API. This keeps masking,
  consent, and soft-delete decisions server-side.
- Audit rows are append-only.

### 10.4 Scale note

Today's Tantivy index is per-instance local disk, which constrains
horizontal scaling of search-heavy traffic; externalizing it is a
roadmap item (§15), as is cross-region PostgreSQL replication.
