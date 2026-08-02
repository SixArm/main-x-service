## 10. Persistence

PostgreSQL 18+ via SeaORM. Schema overview:
[`agents/share/postgresql.md`](../../../agents/share/postgresql.md).

### 10.1 Tables (12+)

`persons`, `person_names`, `person_identifiers`, `person_addresses`,
`person_contacts`, `person_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `person_match_scores`, `audit_log`.

> Migration history note: the early migrations created `patients*`
> tables, and a later migration
> (`*_rename_patient_tables_to_person`) renames them to `persons*`.
> The live schema therefore matches the person-named tables listed
> above, while the historical migration filenames (`*_create_patients`,
> `*_create_patient_related_tables`) are preserved immutably to keep
> the migration history intact.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`. Optional: `pg_vector`, `postgis`.

### 10.3 Connection pooling

Configurable min / max via env (`DATABASE_MIN_CONNECTIONS` /
`DATABASE_MAX_CONNECTIONS`). Soft delete is application-level (`active`
flag); audit triggers retain history.

### 10.4 Cross-service entity links — `entity_links`

The write side of cross-service links (§5.4, §8.6) adds one table,
**separate** from `person_links` (which is within-entity). Schema per
[cross-service linking §4.1](../../../agents/share/cross-service-linking.md):

```sql
CREATE TABLE entity_links (
    id           UUID PRIMARY KEY,
    from_pid     UUID NOT NULL,          -- local person (FK to persons)
    kind         TEXT NOT NULL,          -- same_identity | works_at | member_of
    to_ref       TEXT NOT NULL,          -- EntityRef URN of the far record
    role         TEXT,                   -- e.g. job title
    confidence   DOUBLE PRECISION,       -- 1.0 operator-asserted; <1 suggested
    provenance   TEXT NOT NULL,          -- operator | import | matcher_suggested
    valid_from   DATE,                   -- affiliation start (nullable)
    valid_to     DATE,                   -- affiliation end ("former …")
    created_at   TIMESTAMPTZ NOT NULL,
    deleted_at   TIMESTAMPTZ,            -- soft-delete (withdrawn edge)
    UNIQUE (from_pid, kind, to_ref, valid_from)   -- idempotent upsert key
);
```

This table holds **outbound** edges only; the inverse is the other
endpoint's concern and the aggregator stores both directions. It is
never read by the matcher (the partition rule, §5.1).

### 10.5 Bulk import / export — `bulk_jobs`

Async bulk operations (§9.2) add one table, `bulk_jobs`, per the
family-wide schema in
[bulk import/export §3](../../../agents/share/bulk-import-export.md) — the
canonical column list lives there and is **not** restated. It tracks each
import/export job (`kind`, `format`, `status`, `params`, the
`rows_total`/`rows_created`/`rows_upserted`/`rows_to_review`/`rows_errored`
counts, `actor`, artifact URLs, and `expires_at` TTL), with
`UNIQUE (entity, kind, idempotency_key)` so a retried submit maps to the
same job. Jobs run on the loco `bg_pg` worker; artifacts (uploaded source,
export output, error report) live in the config-driven artifact store
(S3-compatible in deployment, local fs in dev), referenced by short-lived
access-controlled URLs.

Person-specific notes:

- **Idempotency** is anchored on the §9.2 stable keys (scheme-scoped
  national/health identifier or `pid`): re-submitting a file re-upserts the
  same rows to the same state.
- Imported rows are **not** a matcher partition exception — `entity_links`
  cells in a CSV/JSONL row follow the same partition rule (§5.1): bulk
  import may populate `entity_links`, but they are never projected into the
  matcher input.

### 10.6 Bulk CSV column set (§5 flattening declaration)

The **JSONL** format is the lossless reference (the person wire type, one
object per line). The **CSV** format (`src/bulk/csv.rs`, BLK-1) flattens that
wire type with the family-wide
[§5 convention](../../../agents/share/bulk-import-export.md): scalars → one
column each; the primary name (a single nested object) → dotted columns;
arrays / arrays-of-objects → a single JSON-encoded cell. It round-trips
losslessly against JSONL (`decode(encode(p)) == p`), matches columns **by
header name** (operator-reordered / extra columns tolerated), and reports a
malformed row as a per-row error (§7).

Person's declared columns (export order; the export header):

| Column | Source field | Rendering |
|---|---|---|
| `id` | `id` | scalar (UUID) |
| `active` | `active` | `true`/`false` (blank ⇒ default `true`) |
| `name.family` | `name.family` | scalar |
| `name.use_type` | `name.use_type` | scalar (enum, blank ⇒ none) |
| `name.given` | `name.given` | JSON array |
| `name.prefix` | `name.prefix` | JSON array |
| `name.suffix` | `name.suffix` | JSON array |
| `gender` | `gender` | scalar (enum) |
| `birth_date` | `birth_date` | scalar (`YYYY-MM-DD`, blank ⇒ none) |
| `tax_id` | `tax_id` | scalar (blank ⇒ none) |
| `deceased` | `deceased` | `true`/`false` |
| `deceased_datetime` | `deceased_datetime` | scalar (RFC 3339, blank ⇒ none) |
| `marital_status` | `marital_status` | scalar (blank ⇒ none) |
| `multiple_birth` | `multiple_birth` | `true`/`false` (blank ⇒ none) |
| `managing_organization` | `managing_organization` | scalar (UUID, blank ⇒ none) |
| `created_at` / `updated_at` | same | scalar (RFC 3339) |
| `identifiers` | `identifiers` | JSON array-of-objects |
| `additional_names` | `additional_names` | JSON array-of-objects |
| `telecom` | `telecom` | JSON array-of-objects |
| `documents` | `documents` | JSON array-of-objects |
| `emergency_contacts` | `emergency_contacts` | JSON array-of-objects |
| `addresses` | `addresses` | JSON array-of-objects |
| `photo` | `photo` | JSON array-of-strings |
| `links` | `links` | JSON array-of-objects (within-entity person links) |

CSV is inherently lossy for deep nesting, so fidelity-sensitive loads should
prefer JSONL; the JSON-in-cell columns keep the CSV path lossless at the cost
of embedded JSON. **BLK-2 (done):** the codec is wired end-to-end — the
import/export handlers accept `format: "jsonl" | "csv"`, the `bg_pg` worker
dispatches on the job's stored `format` (`process_import_job`/
`process_export_job` take a [`BulkFormat`](../src/bulk/mod.rs)), and the
stored artifact filenames carry the matching extension
(`jobs/{id}/input.{jsonl,csv}`, `jobs/{id}/export.{jsonl,csv}`).

**Keyless-row → duplicate-detection → review-queue routing (BLK-2, done).**
A row with no strong identifier, no `tax_id`, and no explicit `id` of its
own (`stable_key::is_keyless`) cannot idempotently upsert — its
`resolve_stable_key` fallback is only a freshly-generated placeholder pid,
not a real find target. Such a row instead runs the same search-blocking +
matcher duplicate detection `POST /check-duplicates` uses. A likely
duplicate (score ≥ `IMPORT_REVIEW_THRESHOLD`, 0.7 — the same bar the
interactive check uses) still **creates** the row (a bulk load must never
silently withhold legitimate data) but also inserts a
`provenance = "import"` pair into the stored `review_queue`
(`detection_method = "import_duplicate_detection"`), so an operator sees it
flagged rather than discovering it only on a later batch scan. No candidate
clears the threshold ⇒ a plain create, the same path a keyed row with no
match takes. `review_queue.provenance` (`operator` | `import` |
`matcher_suggested` — the cross-service-linking vocabulary) distinguishes
these from the interactive/batch-scan-sourced rows; it is set once on
insert and never touched by a re-scan upsert.

### 10.7 Parquet export (BLK-3, done)

`format: "parquet"` — **export-only** (§12 lean: "export-only in v1,
import is roadmap"; `BulkFormat::is_export_only`, enforced at the import
handler regardless of the crate's build configuration) and
**feature-gated** behind this crate's own `parquet` Cargo feature (off by
default): `src/bulk/parquet_format.rs` and its `arrow`/`parquet`
dependencies only exist when the feature is on, so a deployment that
never needs it carries none of that weight. A build without the feature
still *recognises* `format: "parquet"` as a valid token
(`BulkFormat::parse` never depends on the feature) but
`process_export_job` returns a clean `422` rather than a silent JSONL
substitution — a caller must be told the capability is missing, not
handed a different format it did not ask for.

**Reuses §10.6's column set exactly** — the flattening declaration moved
to a shared `src/bulk/columns.rs` module so CSV and Parquet render one
column list rather than two that could drift. Per §5's "nested via
Parquet nested types or JSON-encoded columns", this crate takes the
JSON-encoded option (matching CSV's own choice): a `Scalar`/`Bool` column
becomes a **nullable** Arrow `Utf8`/`Boolean` column (a real null for an
absent field, not CSV's ambiguous empty string); a `Json` column (the
arrays / arrays-of-objects) becomes a **non-nullable** `Utf8` column
carrying the same compact JSON text CSV puts in its JSON-encoded cells.
One `RecordBatch`, one row group, written via `parquet::arrow::ArrowWriter`.

Verified by reading the encoded bytes back through `parquet`'s own Arrow
reader (both a DB-free unit-test suite in `parquet_format.rs` and a
DB-gated round-trip in `pipeline.rs`), asserting the row count, the
scalar `name.family` value, the JSON-encoded `identifiers` cell, and a
genuine Arrow null (not `""`) for an absent `tax_id`.

**Known gap:** the family CI (`scripts/ci-check.sh`) runs `cargo
test`/`cargo clippy` with this crate's *default* features, so the
`parquet` feature is not exercised by the standard pipeline — only by
running `cargo test --features parquet` / `cargo clippy --features
parquet` locally, as this task's own verification did. Wiring a
dedicated CI feature-matrix entry (mirroring how `cargo-fuzz`/`cargo
deny` already get their own opt-in stage) is a reasonable follow-up but
was judged out of scope for this (Small) task.

### 10.8 S3-compatible artifact store (BLK-4, done)

`src/bulk/store.rs`'s [`ArtifactStore`](../src/bulk/store.rs) trait
became **async** (`#[async_trait]`) and gained a second backend
alongside `LocalFsArtifactStore`, ported from the care-pathway service —
the family's reference implementation
([bulk import/export §12](../../../agents/share/bulk-import-export.md)).
Selected at runtime by `PERSON_BULK_ARTIFACT_BACKEND` (`local`, the
default, `fs`, `file`, or `s3`; an unrecognised value falls back to
`local` with a warning rather than failing to boot):

- **`LocalFsArtifactStore`** — unchanged behaviour, now async. Writes
  under `PERSON_BULK_ARTIFACT_DIR` and returns confined `file://`
  references (SEC-B4).
- **`S3ArtifactStore`** (behind this crate's own `s3` Cargo feature, off
  by default) — any S3-compatible object store (AWS S3, MinIO, Ceph RGW,
  Cloudflare R2) via `PERSON_BULK_S3_{BUCKET(required),ENDPOINT,REGION
  (default us-east-1),FORCE_PATH_STYLE(default on)}`. Credentials come
  from the standard AWS credential chain, never a bespoke variable, so a
  deployment's existing secret management applies unchanged. References
  are `s3://<bucket>/<key>` URLs; `presigned_get` issues a short-lived,
  access-controlled download URL (TTL clamped to `[1, 3600]` seconds) —
  the local backend cannot offer one (a `file://` path is not fetchable
  by a remote client) and says so with `None` rather than a URL-shaped
  reference that is not one. `split_reference` refuses a reference
  naming a different bucket than configured, so a `bulk_jobs` row edit
  cannot be turned into a read of an arbitrary bucket this service's
  credentials can reach.

Asking for `PERSON_BULK_ARTIFACT_BACKEND=s3` in a binary built without
the `s3` feature is a clean error, not a silent fallback to local
storage — that would lose data by writing a deployment's exports to an
ephemeral container disk while appearing to succeed. The AWS SDK is used
rather than hand-rolled SigV4 signing: request signing is
security-relevant code that cannot be verified in this repository's test
environment without a live endpoint or the published AWS test vectors,
and shipping an unverified signing implementation that looks finished is
the worse risk.

The async trait changed `AppState::new` (`src/api/rest/state.rs`) from a
synchronous constructor to `pub async fn new(..) ->
crate::Result<Self>`, since the S3 backend's client construction
resolves credentials asynchronously. Both call sites — the loco
`after_routes` boot hook and the integration-test harness
(`tests/common/mod.rs`) — were already `async fn`, so this is additive;
`bulk_store` is built once at boot (`crate::bulk::store::from_env()`)
and shared via the existing `Arc<dyn ArtifactStore>` field rather than
reconstructed per request.

Verified: `cargo build`/`test`/`clippy --all-targets -D warnings`/`fmt
--check` under default features, `--features s3`, `--features parquet`,
and `--features s3,parquet`; the DB-gated suite
(`scripts/ci-check.sh test-db`) against real Postgres, exercising the
real boot path through `AppState::new`. The live-endpoint S3 round-trip
test is `#[ignore]`d (documented `MinIO` invocation in its doc comment)
since CI has no object store to test against — everything else about
the S3 backend (key safety, reference parsing, path-style default, the
without-the-feature error) is unit-tested without one.
