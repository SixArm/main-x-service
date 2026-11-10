# Patient Flow service — edition spec

Stack-specific specification for the Axum edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth** for the domain (model, state machines,
journeys, views, auth posture); this file adds only what is specific
to this edition. It will grow topic files (routes, api-contract,
database, examples) as implementation phases land — each PF-T* task
adds its spec detail here in the same PR as the code.

## Stack

Per [architecture](../../spec/architecture.md) and the family
[rust-loco-stack](../../../agents/share/rust-axum-stack.md):
Rust 2024, axa (Axum + sqlx), PostgreSQL 18, plain-SQL crate-root
`migrations/`, controller-style `src/controllers/` layout, pure
`src/flow/` core, stub-first upstream clients, offline PASETO +
ABAC, OpenAPI/Swagger, `Accepts-version` header versioning,
tracing + OTLP, Podman.

## Edition-specific decisions (so far)

- **Config**: the environment only — there are no config files (axa
  reads none). `APP_ENV`, `PORT`, `DB_URL` and the rest of
  `axa::config`, over the defaults in `src/app.rs` (`defaults()`). In
  every environment the upstream clients default to `stub`
  (`PATIENT_FLOW_UPSTREAM_MODE`) and events to
  `PATIENT_FLOW_EVENT_TRANSPORT=memory`.
- **Env vars**: `PATIENT_FLOW_REQUIRE_AUTH`,
  `PATIENT_FLOW_PASETO_KEYS[_URL]`, `PATIENT_FLOW_ABAC_POLICY[_FILE]`,
  `PATIENT_FLOW_EVENT_TRANSPORT`, upstream base URLs
  (`PATIENT_FLOW_PERSON_SERVICE_URL`, …), DTOC grace
  (`PATIENT_FLOW_DTOC_GRACE`, default `midnight`).
- **Identifiers**: public UUID `pid` on every owned record; EntityRef
  URNs for all upstream references.

## Edition-specific implementation notes

- **Layout**: `src/{app,auth,clients,client_limits,metrics,openapi,
  streaming,validation}.rs`, `src/flow/` (pure core), `src/models/`
  (one module per table), `src/controllers/{topology,stays,
  bed_requests,boards,audits,docs,metrics}.rs`, `src/tasks/seed.rs`,
  crate-root `migrations/` and `.sqlx/`.
- **Persistence (sqlx, 2026-11-08)**: every SQL statement the service
  runs is a `sqlx::query!` / `query_as!` / `query_scalar!` literal in
  `src/models/`, the crate's one SQL area (repo
  `spec/tech-stack/index.md` §5). Each module owns one table: a plain
  `Model` row struct, serialized as-is by the read endpoints, and the
  statements against it. Helpers take an `axa::sqlx::Db`, so a handler
  passes its open transaction (`&mut *txn`) and the row change, its
  audit row and its outbox row still commit together (PF-D9). The
  compile-time query data is committed in `.sqlx/` and builds run with
  `SQLX_OFFLINE=true`. There is no `QueryBuilder`, `raw_sql` or
  `AssertSqlSafe`: the whiteboard's `IN` lists are `= ANY($1)`, and
  the partial `PUT` updates are `COALESCE($n, col)`, which leaves an
  absent field as it was, the way the `SeaORM` `ActiveModel` did.
- **Migrations**: five plain-SQL pairs under `migrations/`
  (`<version>_<name>.up.sql` / `.down.sql`), run by `sqlx::migrate!`
  through `axa::sqlx` for `db migrate | down | reset | status` and the
  boot auto-migrate. They were written from a `pg_dump --schema-only`
  of the database the old `SeaORM` schema-builder migrations produced,
  and a dump of the new schema is identical apart from the bookkeeping
  table (`_sqlx_migrations` for `seaql_migrations`), so a database the
  old build migrated must be recreated. `event_outbox` keeps its
  `SERIAL` (`integer`) id; every other table has a `bigint` identity.
- **`updated_at` is stamped in the SQL** (PF-U1): every `UPDATE` that
  changes a row — partial `PUT`s, transitions, soft deletes, the relay
  ack — sets `updated_at = now()` itself, and the `Red2Green` upsert
  does the same on its conflict branch. There is no trigger, so a
  statement that leaves it out leaves it unchanged; under `SeaORM` (and
  the first sqlx cut) none set it and it always equalled `created_at`.
- **`Red2Green` is one upsert** (PF-R1): `POST /api/stays/{pid}/red-green`
  writes `INSERT … ON CONFLICT (stay_pid, day) DO UPDATE` on the
  `red_green_stay_day` key, inside the handler's transaction with its
  audit and outbox rows, and answers the day's row as before. Two
  concurrent same-day POSTs both succeed and leave one row, holding the
  later write; the old read-then-insert let the second one break the
  unique index and answer `409`.
- **Outbox relay poll** (PF-O1): `event_outbox::Model::unpublished`
  (oldest unpublished rows, `FOR UPDATE SKIP LOCKED`) and
  `mark_published` are the poll and ack of the relay
  [`../../spec/integrations.md`](../../spec/integrations.md) and
  [`../../spec/audit.md`](../../spec/audit.md) specify (outbox → Fluvio,
  Phase 3). No relay runs in this crate yet; they are kept for it, and a
  DB test pins that a second relay does not see rows the first holds.
- **Clients**: one `src/clients.rs` module (mode-switched `stub` /
  `http` via `PATIENT_FLOW_UPSTREAM_MODE`, default `stub`) rather
  than per-service trait modules — patient-flow needs only
  display-name resolution.
- **`/events/recent`** reads the in-memory ring under the `memory`
  transport and the `event_outbox` table under `outbox`.

## Delivery

The queue is [../../spec/tasks.md](../../spec/tasks.md): PF-T1–T14
and PF-T17 (ETag conditional board reads + the DB-gated request /
enforcement suites — run with `cargo test -- --ignored` against
Postgres) **delivered 2026-07-18**. Tests per
[../../spec/testing.md](../../spec/testing.md).
