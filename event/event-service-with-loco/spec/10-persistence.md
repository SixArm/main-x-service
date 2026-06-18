## 10. Persistence

PostgreSQL 18+ via SeaORM.

### 10.1 Tables

- `events` — scalar columns (name, description, time window,
  status / mode / type, capacities, audit cols) plus JSONB arrays for
  `alternate_names`, `image`, `same_as`, `keywords`, `in_language`.
- `event_identifiers`, `event_locations`, `event_parties`,
  `event_offers`, `event_links`, `event_sub_events`.
- `organizations`, `organization_addresses`,
  `organization_contacts`, `organization_identifiers`.
- `audit_log`.

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
same job. Jobs run on the loco `bg_pg` worker; artifacts (uploaded source,
export output, error report) live in the config-driven artifact store
(S3-compatible in deployment, local fs in dev), referenced by short-lived
access-controlled URLs. Idempotency is anchored on the §9.1 stable keys
(scheme-scoped `event_ids` `(scheme, value)` pair or the event `id`):
re-submitting a file re-upserts the same rows to the same state.

