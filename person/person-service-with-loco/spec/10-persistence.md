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
