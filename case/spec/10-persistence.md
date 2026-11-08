## 10. Persistence

PostgreSQL via sqlx with no ORM (since 2026-11-08), owned entirely by
the service crate: every statement is a `query!` literal in
[`src/models/`](../case-service-with-axum/src/models/), and the schema
is the plain SQL under
[`migrations/`](../case-service-with-axum/migrations/), run by
`sqlx::migrate!` (crate spec §10). Family conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md) and
[`spec/tech-stack/index.md`](../../spec/tech-stack/index.md) §5.

The tables below show each one as first created; later migrations add
the audit chain, disclosure and redaction columns to `audit_logs`, the
digest and MAC columns to `cases`, and the `event_outbox`,
`entity_links`, `review_queue` and `bulk_jobs` tables.

### 10.1 Tables

The `cases` table, created by migration
[`2022010100000001_cases`](../case-service-with-axum/migrations/2022010100000001_cases.up.sql):

| Column | Type | Notes |
|---|---|---|
| `id` | `bigint` identity | Internal row id |
| `pid` | `uuid`, unique | Public id |
| `title` | `character varying` | Denormalised `data.title` for listing |
| `data` | `jsonb` | Full `Case` payload |
| `active` | `boolean`, default `true` | Registry flag |
| `deleted_at` | `timestamptz`, nullable | Soft delete |

The `audit_logs` table, created by migration
[`2022010100000002_audit_logs`](../case-service-with-axum/migrations/2022010100000002_audit_logs.up.sql)
— one row per CRUD / merge action:

| Column | Type | Notes |
|---|---|---|
| `id` | `bigint` identity | Internal row id |
| `entity_pid` | `uuid` | The case the entry concerns |
| `action` | `character varying` | `created` / `updated` / `deleted` / `merged` |
| `actor` | `character varying`, nullable | Caller `sub` from the bearer token, else `NULL` |
| `snapshot` | `jsonb`, nullable | The record's `data` at the time of the action |

The `merge_records` table, created by migration
[`2022010100000003_merge_records`](../case-service-with-axum/migrations/2022010100000003_merge_records.up.sql)
— one row per record-merge:

| Column | Type | Notes |
|---|---|---|
| `id` | `bigint` identity | Internal row id |
| `main_pid` | `uuid` | The surviving case |
| `duplicate_pid` | `uuid` | The merged-away (now soft-deleted) case |
| `reason` | `character varying`, nullable | Optional operator-supplied reason |
| `actor` | `character varying`, nullable | Caller `sub` from the bearer token, else `NULL` |
| `transferred` | `jsonb`, nullable | Snapshot of the duplicate's payload at merge time |

Every table also carries `created_at` / `updated_at` (`timestamptz`,
default `CURRENT_TIMESTAMP`), except `review_queue` and `bulk_jobs`.

### 10.2 JSONB rationale

The payload is stored verbatim so that the matcher type remains the
single schema. Adding a field to `Case` is a matcher-crate change plus
a CHANGELOG entry — **no service migration** — because serde fills
missing fields with defaults on read. The trade-off: querying inside
the payload needs JSONB operators (or GIN indexes, roadmap), and the
only relational projections are `pid` and `title`.

### 10.3 Operations

- `auto_migrate` is on in development (`cargo run -- start` migrates).
- Soft delete is application-level (`deleted_at`); all queries filter
  `deleted_at IS NULL`.
- Audit rows are written best-effort (a failed audit write logs but
  never fails the originating request).
- Open question OQ-3: normalise `subjects` / `keywords` into side
  tables once search lands, to enable subject-level queries and
  candidate blocking.
- Privacy note: because the payload is personal data (§12), retention,
  encryption-at-rest, and a GDPR-erasure path on top of soft delete are
  deployment-side requirements; per-field masking + export are §13
  T-10.
