## 10. Persistence

PostgreSQL via SeaORM 1.1 + `sea-orm-migration`, owned entirely by
the service crate. Family conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md).

### 10.1 Tables

The `care_pathways` table, created by migration
[`m20220101_000001_care_pathways`](../care-pathway-service-rust-crate/migration/src/m20220101_000001_care_pathways.rs):

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `pid` | `UuidUniq` | Public id |
| `name` | `String` | Denormalised `data.name` for listing |
| `data` | `JsonBinary` (JSONB) | Full `CarePathway` payload |
| `active` | `BooleanWithDefault(true)` | Registry flag |
| `deleted_at` | `TimestampWithTimeZoneNull` | Soft delete |

The `audit_logs` table, created by migration
[`m20220101_000002_audit_logs`](../care-pathway-service-rust-crate/migration/src/m20220101_000002_audit_logs.rs)
— one row per CRUD action (T-3):

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `entity_pid` | `Uuid` | The care pathway the entry concerns |
| `action` | `String` | `created` / `updated` / `deleted` |
| `actor` | `StringNull` | User / system id — `NULL` until JWT auth (T-7) |
| `snapshot` | `JsonBinaryNull` | The record's `data` at the time of the action |

### 10.2 JSONB rationale

The payload is stored verbatim so that the matcher type remains the
single schema. Adding a field to `CarePathway` is a matcher-crate
change plus a CHANGELOG entry — **no service migration** — because
serde fills missing fields with defaults on read. The trade-off:
querying inside the payload needs JSONB operators (or GIN indexes,
roadmap), and the only relational projections are `pid` and `name`.

### 10.3 Operations

- `auto_migrate` is on in development (`cargo loco start` migrates).
- Soft delete is application-level (`deleted_at`); all queries filter
  `deleted_at IS NULL`.
- Open question OQ-3: normalise condition codes / interventions into
  side tables once search lands, to enable code-level queries and
  candidate blocking.
