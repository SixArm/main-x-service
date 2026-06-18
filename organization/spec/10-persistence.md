## 10. Persistence

PostgreSQL via SeaORM 1.1 + `sea-orm-migration` (loco-idiomatic).
Family-wide database conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md).

### 10.1 Tables

| Table | Migration | Columns |
|---|---|---|
| `organizations` | `m20220101_000001_organizations` | `id` (auto PK), `pid` (UUID unique), `name` (string, denormalised), `data` (JSONB — full `Organization` payload), `active` (bool default true), `deleted_at` (timestamptz null) |
| `audit_logs` | `m20220101_000002_audit_logs` | `id` (auto PK), `entity_pid` (UUID), `action` (string: created / updated / deleted), `actor` (string null), `snapshot` (JSONB null) |

### 10.2 JSONB persistence model

The entity stores the canonical DTO **verbatim** as the `data` column
— this is the integration contract, not an implementation detail:

- Whatever the API accepts is exactly what is stored and exactly what
  matching deserialises. One serde round-trip is the whole mapping,
  pinned by the service's
  [`tests/matching.rs`](../organization-service-with-loco/tests/matching.rs).
- `name` is the only denormalised column, maintained on create /
  update, serving listing and `ILIKE` search.
- Trade-off: no per-field SQL indexing of identifiers / addresses
  yet. Whether to normalise identifiers and addresses into their own
  tables once search and register-feed ingestion land is an open
  question (§16, also the service crate's §16).

### 10.3 Soft delete

Soft delete stamps `deleted_at`; all read paths (`find_by_pid`,
`list`, `search`) filter `deleted_at IS NULL`. Rows are never
removed — this is the entity's GDPR-balancing retention mechanism
(§12) and matches the family-wide rule.

### 10.4 Migrations and environments

- Migrations live in
  [`migration/src/`](../organization-service-with-loco/migration/);
  new tables MUST be added as `sea-orm-migration` migrations.
- `auto_migrate: true` in `config/development.yaml`; production
  applies migrations explicitly.
- Extensions: none required by the current schema beyond stock
  PostgreSQL. `pg_trgm` (trigram index for the `ILIKE` search) and
  the family-standard set become relevant with §15 search work.
