## 10. Persistence

PostgreSQL 18+ via SeaORM. Schema overview:
[`agents/share/postgresql.md`](../../../agents/share/postgresql.md).

### 10.1 Tables (12+)

`persons`, `person_names`, `person_identifiers`, `person_addresses`,
`person_contacts`, `person_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `person_match_scores`, `audit_log`.

> Migration history note: the early migrations created `patients*`
> tables, and a later migration
> (`*_rename_patient_tables_to_person`) renames them to `persons*`.
> The live schema therefore matches the person-named tables listed
> above, while the historical migration filenames (`*_create_patients`,
> `*_create_patient_related_tables`) are preserved immutably to keep
> the migration history intact.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`. Optional: `pg_vector`, `postgis`.

### 10.3 Connection pooling

Configurable min / max via env (`DATABASE_MIN_CONNECTIONS` /
`DATABASE_MAX_CONNECTIONS`). Soft delete is application-level (`active`
flag); audit triggers retain history.

### 10.4 Cross-service entity links — `entity_links`

The write side of cross-service links (§5.4, §8.6) adds one table,
**separate** from `person_links` (which is within-entity). Schema per
[cross-service linking §4.1](../../../agents/share/cross-service-linking.md):

```sql
CREATE TABLE entity_links (
    id           UUID PRIMARY KEY,
    from_pid     UUID NOT NULL,          -- local person (FK to persons)
    kind         TEXT NOT NULL,          -- same_identity | works_at | member_of
    to_ref       TEXT NOT NULL,          -- EntityRef URN of the far record
    role         TEXT,                   -- e.g. job title
    confidence   DOUBLE PRECISION,       -- 1.0 operator-asserted; <1 suggested
    provenance   TEXT NOT NULL,          -- operator | import | matcher_suggested
    valid_from   DATE,                   -- affiliation start (nullable)
    valid_to     DATE,                   -- affiliation end ("former …")
    created_at   TIMESTAMPTZ NOT NULL,
    deleted_at   TIMESTAMPTZ,            -- soft-delete (withdrawn edge)
    UNIQUE (from_pid, kind, to_ref, valid_from)   -- idempotent upsert key
);
```

This table holds **outbound** edges only; the inverse is the other
endpoint's concern and the aggregator stores both directions. It is
never read by the matcher (the partition rule, §5.1).

### 10.5 Bulk import / export — `bulk_jobs`

Async bulk operations (§9.2) add one table, `bulk_jobs`, per the
family-wide schema in
[bulk import/export §3](../../../agents/share/bulk-import-export.md) — the
canonical column list lives there and is **not** restated. It tracks each
import/export job (`kind`, `format`, `status`, `params`, the
`rows_total`/`rows_created`/`rows_upserted`/`rows_to_review`/`rows_errored`
counts, `actor`, artifact URLs, and `expires_at` TTL), with
`UNIQUE (entity, kind, idempotency_key)` so a retried submit maps to the
same job. Jobs run on the loco `bg_pg` worker; artifacts (uploaded source,
export output, error report) live in the config-driven artifact store
(S3-compatible in deployment, local fs in dev), referenced by short-lived
access-controlled URLs.

Person-specific notes:

- **Idempotency** is anchored on the §9.2 stable keys (scheme-scoped
  national/health identifier or `pid`): re-submitting a file re-upserts the
  same rows to the same state.
- Imported rows are **not** a matcher partition exception — `entity_links`
  cells in a CSV/JSONL row follow the same partition rule (§5.1): bulk
  import may populate `entity_links`, but they are never projected into the
  matcher input.

