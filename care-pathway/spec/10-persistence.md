## 10. Persistence

PostgreSQL via SeaORM 1.1 + `sea-orm-migration`, owned entirely by
the service crate. Family conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md).

### 10.1 Tables

The `care_pathways` table, created by migration
[`m20220101_000001_care_pathways`](../care-pathway-service-with-loco/migration/src/m20220101_000001_care_pathways.rs):

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `pid` | `UuidUniq` | Public id |
| `name` | `String` | Denormalised `data.name` for listing |
| `data` | `JsonBinary` (JSONB) | Full `CarePathway` payload |
| `active` | `BooleanWithDefault(true)` | Registry flag |
| `deleted_at` | `TimestampWithTimeZoneNull` | Soft delete |

The `audit_logs` table, created by migration
[`m20220101_000002_audit_logs`](../care-pathway-service-with-loco/migration/src/m20220101_000002_audit_logs.rs)
— one row per CRUD action (T-3):

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `entity_pid` | `Uuid` | The care pathway the entry concerns |
| `action` | `String` | `created` / `updated` / `deleted` |
| `actor` | `StringNull` | User / system id — `NULL` until token auth (T-7) |
| `snapshot` | `JsonBinaryNull` | The record's `data` at the time of the action |

The `merge_records` table, created by migration
[`m20220101_000003_merge_records`](../care-pathway-service-with-loco/migration/src/m20220101_000003_merge_records.rs)
— one row per record-merge (T-8):

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `main_pid` | `Uuid` | The surviving pathway |
| `duplicate_pid` | `Uuid` | The merged-away (now soft-deleted) pathway |
| `reason` | `StringNull` | Optional operator-supplied reason |
| `actor` | `StringNull` | Caller `sub` from the bearer token, else `NULL` |
| `transferred` | `JsonBinaryNull` | Snapshot of the duplicate's payload at merge time |

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

### 10.4 Bulk import / export — `bulk_jobs`

Async bulk operations (§9.4) add one table, `bulk_jobs`, per the
family-wide schema in
[bulk import/export §3](../../agents/share/bulk-import-export.md) — the
canonical column list lives there and is **not** restated. It tracks each
import/export job (`kind`, `format`, `status`, `params`, the
`rows_total`/`rows_created`/`rows_upserted`/`rows_to_review`/`rows_errored`
counts, `actor`, artifact URLs, and `expires_at` TTL), with
`UNIQUE (entity, kind, idempotency_key)` so a retried submit maps to the
same job. Jobs run on the loco `bg_pg` worker; artifacts (uploaded source,
export output, error report) live in the config-driven artifact store
(`src/bulk/store.rs`), selected by `CARE_PATHWAY_BULK_ARTIFACT_BACKEND`:
`local` (the default — `file://` references confined to
`CARE_PATHWAY_BULK_ARTIFACT_DIR`) or `s3` (any S3-compatible store, behind
the optional `s3` cargo feature, issuing presigned download URLs capped at
one hour). Requesting `s3` from a binary built without the feature is an
error rather than a silent fall back to local disk. See
[`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md)
§12 for the full contract and the reasoning.

**Honest limit.** The S3 backend's key-safety and reference-parsing rules
are unit-tested, but a `put`/`get` round trip through a signed request
needs a live endpoint, which CI does not have. That round trip is covered
by an `#[ignore]`d opt-in test (`s3_round_trip_against_a_live_endpoint`)
runnable against `MinIO`; until someone runs it against a real store, the
S3 path is verified by the compiler and the parsing tests, not by
end-to-end evidence.

Care-pathway-specific notes:

- **Idempotency** is anchored on the §9.4 stable keys (a deterministic
  scheme-scoped identifier, the provider-scoped `(provider_id,
  pathway_code)`, or `pid`): re-submitting a file re-upserts the same rows
  to the same state.
- The matcher's **single schema** holds: the JSONB `data` column is the
  full `CarePathway` payload, so JSONL round-trips losslessly and CSV
  flattens per §9.4 (every repeated / nested field is a JSON-encoded cell);
  no bulk path bypasses the JSONB round-trip invariant (§5.5).
