# PostgreSQL

Monorepo-wide reference for how the **Main X Index** family uses
PostgreSQL. This is the comprehensive spec; the short version lives at
[`agents/share/postgresql.md`](../../agents/share/postgresql.md). Each
service owns its own database — there is no shared database — but they all
follow the conventions below.

> Related: [data-modeling.md](../data-modeling.md) (SQL-first child-table
> rules), [architecture](../architecture/index.md),
> [auditability](../auditability/index.md), [search](../search/index.md),
> [event-streaming](../event-streaming/index.md).

## 1. Version & baseline

- **PostgreSQL 18** is the target. (A few older crate docs still say 14/15/17;
  18 is canonical — treat those as drift.)
- Access is always via **sqlx 0.9** (`postgres`, `runtime-tokio`,
  `tls-rustls-ring-webpki`), through literal `query!` statements checked at
  compile time against the committed `.sqlx/`
  ([tech-stack](../tech-stack/index.md) §5); no service issues hand-rolled
  libpq calls. axa's `sqlx` feature opens the pool and its `queue` feature
  runs the job queue.
- sqlx type features in use: `uuid`, `json` and `chrono`. `chrono` only,
  never `time` ([`rust-axum-stack.md`](../../agents/share/rust-axum-stack.md)).

## 2. One database per service

Every service is isolated in its own database (microservice-per-DB). There
is **no** cross-database access and no shared schema; services integrate
over HTTP (authenticated with cross-service PASETO v4.public tokens; the
prior RS256 JWT model is decommissioned), not the database.

- **Naming.** `<entity>_service_<env>` for development and test
  (`person_service_development`, `organization_service_test`, …);
  `case_folder_<env>` for the case-folder app. Production names come from
  `DB_URL`.
- **Roles (dev defaults).** controller-style services default to `loco`/`loco`;
  case-folder to `postgres`/`postgres`; the older Axum services to peer
  auth as `$USER` (`postgres://localhost/<db>`). All overridable via
  `DB_URL`.
- The full database list and a provisioning script are maintained with the
  service configs; each `config/{development,test}.yaml` carries the
  canonical default.

## 3. Connection & pool configuration

The controller-style services configure the connection in `config/*.yaml` under
`database:`:

| Key | Dev default | Notes |
|---|---|---|
| `uri` | `DB_URL` or the per-service default | Postgres connection string. |
| `enable_logging` | `false` | Logs every SQL statement when `true`. |
| `connect_timeout` | `500` (ms) | Acquire-connection timeout. |
| `idle_timeout` | `500` | Idle duration before a pooled connection closes. |
| `min_connections` | `1` | **Raise for production** (1 is a dev convenience). |
| `max_connections` | `1` | **Raise for production.** |
| `auto_migrate` | `true` (dev) | Run migrations on boot. **`false` in production** — migrate as a deploy step. |
| `dangerously_truncate` | `false` | Truncate all tables on boot — test/dev only. |
| `dangerously_recreate` | `false` | Drop + recreate schema on boot — test/dev only. |

The older Axum services read the same values from environment
(`DATABASE_MAX_CONNECTIONS`, etc.). Stateless service design plus pooling
is what enables horizontal scaling (see
[availability](../availability/index.md)).

## 4. Two persistence styles

The repo deliberately runs two schema styles; know which one a service uses
before touching its tables.

### 4.1 Normalized (the older Axum / MPI-style services: person, worker, place, thing, event)

Fully normalized SQL per [data-modeling.md](../data-modeling.md):

- A parent table (`persons`, `workers`, …) plus **child tables** for every
  repeating collection (names, identifiers, addresses, contacts, links),
  each with a foreign key `ON DELETE CASCADE` and an explicit `position`
  ordering column.
- **Enums are `VARCHAR` + `CHECK`** constraints, not native enum types.
- **Polymorphic unions / multi-role lists** use a discriminator column, not
  JSONB.
- Migrations are **raw SQL** `up.sql`/`down.sql` pairs under `migrations/`
  (timestamp-named, e.g. `2024122800000001_create_organizations/`), with a
  committed `schema.sql` snapshot. An `add_indexes_and_triggers` migration
  adds the secondary indexes and `updated_at` triggers.

### 4.2 Document-in-a-column (the controller-style services: organization, care-pathway, case; and authentication for its own tables)

- The matcher/DTO type is stored **verbatim as a JSONB `data` column**, with
  a denormalized scalar (`name` or `title`) lifted out for listing and
  search, plus `pid` (UUID), `active`, and soft-delete timestamps.
- This is the **deliberate exception** to the "JSONB only for opaque
  snapshots" rule (§6): the whole DTO is opaque-by-design so there is no
  model/adapter to drift from the matcher type. Structured querying is not a
  goal for these services (matching reads the payload into the matcher).
- Side tables are normal columns: `audit_logs`, `merge_records`, and (for
  authentication) `users`, `sessions`, `auth_events`, `auth_rate_limits`.
- Migrations are plain SQL, `migrations/<version>_<name>.up.sql` with a
  `.down.sql`, embedded by `sqlx::migrate!` and run by the service's
  `db migrate` command (§11).

## 5. Extensions

The canonical menu (declare what a deployment needs):

| Extension | Purpose | Used today |
|---|---|---|
| `pgcrypto` | UUIDs (`gen_random_uuid`), hashing/encryption | **Yes** (MPI schemas) |
| `pg_trgm` | Trigram GIN indexes for fuzzy / `ILIKE` acceleration | **Yes** (MPI `add_indexes_and_triggers`) |
| `uuid-ossp` | UUID v4 generation (alternative to pgcrypto) | Available |
| `citext` | Case-insensitive text columns for matching | Available |
| `unaccent` | Diacritic-insensitive text search | Available |
| `postgis` | Geographic types / queries (place geo-radius) | Roadmap (place) |
| `pg_vector` | Similarity search / RAG embeddings | Roadmap |
| `pg_stat_statements` | Execution-statistics / slow-query analysis | Ops |

`CREATE EXTENSION IF NOT EXISTS` belongs in the first migration of any
service that depends on it.

## 6. JSONB policy

From [data-modeling.md](../data-modeling.md): **JSONB is only for genuinely
opaque snapshots** — audit `old_values`/`new_values`, merge
`transferred`/`snapshot`, review-queue `score_breakdown` — not for
structured domain data, which uses columns and child tables.

The one sanctioned exception is the controller-style services' entity `data` column
(§4.2): the payload *is* the opaque matcher DTO, stored whole on purpose.

## 7. Identifiers, soft delete, audit

- **Public id (`pid`).** A UUID exposed in the API, distinct from the
  internal `BIGSERIAL`/`i32` primary key. Never expose the serial key.
- **Soft delete.** Records are never hard-deleted: an `active` boolean
  and/or a `deleted_at` timestamp tombstones them (GDPR erasure additionally
  anonymises columns). Reads filter to active rows.
- **Audit.** Every CRUD/merge writes an `audit_logs` row (action + JSON
  before/after snapshot + actor + timestamp). Audit rows **never** carry
  tokens or secrets. See [auditability](../auditability/index.md).

## 8. Search (ILIKE today, Tantivy planned)

- The controller-style services do **case-insensitive substring search** on the
  denormalized scalar: `Expr::col(Name).ilike("%{q}%")` over active rows,
  capped at 50, blank `q` → `400`.
- User input is wildcard-escaped (`escape_like` escapes `%`, `_`, `\`) so it
  matches literally — no injection of `LIKE` metacharacters.
- `pg_trgm` GIN indexes accelerate `ILIKE`/fuzzy lookups in the normalized
  services. Full-text **Tantivy** search is the planned upgrade across the
  family (see [search](../search/index.md)); Postgres `ILIKE` is the
  pragmatic interim.

## 9. Concurrency: transactions & advisory locks

- **Transactions** wrap multi-row writes: a normalized create folds parent +
  child rows atomically; merge transfers data + soft-deletes the duplicate +
  writes history in one unit.
- **Advisory locks.** The magic-link rate limiter takes a per-key
  transaction-scoped advisory lock —
  `pg_advisory_xact_lock(hashtext(email_key))` — so concurrent checks for the
  same email are exact while different emails never contend. See the
  `auth_rate_limits` table and [authentication](../authentication/index.md).
- **Transactional outbox.** The durable event bus (design:
  [event-streaming](../event-streaming/index.md)) writes an `event_outbox`
  row in the *same* transaction as the entity change, so an event exists iff
  its mutation committed.

## 10. Background jobs

Background jobs are **Postgres-backed** (the axa `worker` feature,
`queue.kind: Postgres`), sharing the app database — axa creates its own
queue tables there. Config lives under `queue:` in `config/*.yaml`. See
[`agents/share/axa.md`](../../agents/share/axa.md).

## 11. Migrations workflow

- **Every service:** plain-SQL `migrations/<version>_<name>.up.sql` and
  `.down.sql`, embedded by `sqlx::migrate!` and applied by
  `cargo run -- db migrate`, which records them in `_sqlx_migrations`. A new
  migration is a new file; sqlx checksums applied ones, so an applied file is
  never edited. After a schema change, regenerate `.sqlx/` with
  `cargo sqlx prepare -- --all-targets` (sqlx-cli reads `DATABASE_URL`).
- **Production:** `auto_migrate: false`; run migrations as an explicit,
  reviewed deploy step, never implicitly on boot.

## 12. Testing & CI

- DB-backed tests are `#[ignore]`d so a checkout without Postgres keeps
  `cargo test` green; run them with `cargo test -- --ignored` and a
  `DB_URL` (or `config/test.yaml`).
- CI provisions a `postgres` service container with the **per-service** test
  database (`<entity>_service_test`); `dangerously_truncate` resets state
  between tests. See [testing](../testing/index.md).

## 13. Operations

- **Pooling** (§3) + stateless services → horizontal scale; size
  `max_connections` to `(replicas × pool) < server max_connections`.
- **Health checks** verify DB connectivity for orchestration.
- **Backups:** `pg_dump` per database; restore into the matching
  per-service name.
- **Observability:** enable `pg_stat_statements`; sqlx logs statements on
  the `sqlx::query` tracing target (`RUST_LOG=sqlx::query=debug`); correlate with the tracing/OTLP stack
  ([observability](../observability/index.md)).

## 14. Compliance notes

PostgreSQL is the system of record for personal data (person, worker, case
subjects). Honour the family compliance posture
([compliance](../compliance/index.md)): encryption at rest/in transit,
least-privilege roles, the immutable audit trail, soft-delete + anonymise
for GDPR erasure, and no secrets in audit/log rows.
