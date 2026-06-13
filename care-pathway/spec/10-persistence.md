## 10. Persistence

PostgreSQL via SeaORM 1.1 + `sea-orm-migration`, owned entirely by
the service crate. Family conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md).

### 10.1 Tables

One table, `care_pathways`, created by migration
[`m20220101_000001_care_pathways`](../care-pathway-service-rust-crate/migration/src/m20220101_000001_care_pathways.rs):

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `pid` | `UuidUniq` | Public id |
| `name` | `String` | Denormalised `data.name` for listing |
| `data` | `JsonBinary` (JSONB) | Full `CarePathway` payload |
| `active` | `BooleanWithDefault(true)` | Registry flag |
| `deleted_at` | `TimestampWithTimeZoneNull` | Soft delete |

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
