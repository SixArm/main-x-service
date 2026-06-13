## 10. Persistence

PostgreSQL via SeaORM 1.1 + `sea-orm-migration`, owned entirely by the
service crate. Family conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md).

### 10.1 Tables

The `cases` table, created by migration
[`m20220101_000001_cases`](../case-service-rust-crate/migration/src/m20220101_000001_cases.rs):

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `pid` | `UuidUniq` | Public id |
| `title` | `String` | Denormalised `data.title` for listing |
| `data` | `JsonBinary` (JSONB) | Full `Case` payload |
| `active` | `BooleanWithDefault(true)` | Registry flag |
| `deleted_at` | `TimestampWithTimeZoneNull` | Soft delete |

The `audit_logs` table, created by migration
[`m20220101_000002_audit_logs`](../case-service-rust-crate/migration/src/m20220101_000002_audit_logs.rs)
— one row per CRUD / merge action:

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `entity_pid` | `Uuid` | The case the entry concerns |
| `action` | `String` | `created` / `updated` / `deleted` / `merged` |
| `actor` | `StringNull` | Caller `sub` from the bearer token, else `NULL` |
| `snapshot` | `JsonBinaryNull` | The record's `data` at the time of the action |

The `merge_records` table, created by migration
[`m20220101_000003_merge_records`](../case-service-rust-crate/migration/src/m20220101_000003_merge_records.rs)
— one row per record-merge:

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `main_pid` | `Uuid` | The surviving case |
| `duplicate_pid` | `Uuid` | The merged-away (now soft-deleted) case |
| `reason` | `StringNull` | Optional operator-supplied reason |
| `actor` | `StringNull` | Caller `sub` from the bearer token, else `NULL` |
| `transferred` | `JsonBinaryNull` | Snapshot of the duplicate's payload at merge time |

### 10.2 JSONB rationale

The payload is stored verbatim so that the matcher type remains the
single schema. Adding a field to `Case` is a matcher-crate change plus
a CHANGELOG entry — **no service migration** — because serde fills
missing fields with defaults on read. The trade-off: querying inside
the payload needs JSONB operators (or GIN indexes, roadmap), and the
only relational projections are `pid` and `title`.

### 10.3 Operations

- `auto_migrate` is on in development (`cargo loco start` migrates).
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
