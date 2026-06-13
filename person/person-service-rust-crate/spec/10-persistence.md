## 10. Persistence

PostgreSQL 18+ via SeaORM. Schema overview:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md).

### 10.1 Tables (12+)

`persons`, `person_names`, `person_identifiers`, `person_addresses`,
`person_contacts`, `person_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `person_match_scores`, `audit_log`.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`. Optional: `pg_vector`, `postgis`.

### 10.3 Connection pooling

Configurable min / max via env (`DATABASE_MIN_CONNECTIONS` /
`DATABASE_MAX_CONNECTIONS`). Soft delete is application-level (`active`
flag); audit triggers retain history.

