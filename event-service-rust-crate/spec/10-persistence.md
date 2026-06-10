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

