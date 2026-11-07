## 10. Persistence

PostgreSQL via sqlx 0.9 with no ORM (controller-style; SeaORM until
2026-11-07).
Family-wide database conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md).

### 10.1 Tables

| Table | Migration | Columns |
|---|---|---|
| `organizations` | `2022010100000001_organizations` | `id` (auto PK), `pid` (UUID unique), `name` (string, denormalised), `data` (JSONB — full `Organization` payload), `active` (bool default true), `deleted_at` (timestamptz null) |
| `audit_logs` | `2022010100000002_audit_logs` | `id` (auto PK), `entity_pid` (UUID), `action` (string: created / updated / deleted), `actor` (string null), `snapshot` (JSONB null) |

### 10.2 JSONB persistence model

The entity stores the canonical DTO **verbatim** as the `data` column
— this is the integration contract, not an implementation detail:

- Whatever the API accepts is exactly what is stored and exactly what
  matching deserialises. One serde round-trip is the whole mapping,
  pinned by the service's
  [`tests/matching.rs`](../organization-service-with-axum/tests/matching.rs).
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
  [`migrations/`](../organization-service-with-axum/migrations/) as
  plain SQL, one `<version>_<name>.up.sql` / `.down.sql` pair each, run
  by `sqlx::migrate!` (`db migrate | down | reset | status`); new tables
  MUST be added the same way. Every statement the service runs is a
  `sqlx::query!` literal in `src/models/`, checked against the committed
  `.sqlx/` (service spec §10). They replaced the SeaORM `migration/`
  crate on 2026-11-07 with an identical schema.
- `auto_migrate: true` in `config/development.yaml`; production
  applies migrations explicitly.
- Extensions: none required by the current schema beyond stock
  PostgreSQL. `pg_trgm` (trigram index for the `ILIKE` search) and
  the family-standard set become relevant with §15 search work.
