# Bulk import / export — design

How the Main X Index family loads and extracts records **in bulk** — initial
migration, periodic syncs, analytics extracts, and bulk GDPR export. This is
a design document: it fixes the execution model, the API surface, the file
formats, the import dedupe semantics, the error contract, and the export
privacy/audit posture, so every entity service adopts one uniform contract
without re-litigating. Only the per-entity **stable key** and **CSV column
mapping** differ (§10). It reuses the matcher + duplicate-review queue
([match-search-merge.md](match-search-merge.md)), the event bus
([event-bus.md](event-bus.md)), the audit trail
([auditability.md](auditability.md)), and the privacy/masking rules
([privacy.md](privacy.md)).

## 1. Why change

Today each service offers single-record CRUD and (per entity) a single-record
GDPR export. There is no way to load a million rows from a legacy system, or
extract a filtered set for a data platform. Bulk import/export is listed as a
common capability every crate should provide ([overview.md](overview.md)); this
fixes its shape once.

## 2. Goals & non-goals

**Goals**

- **Async, job-based** bulk import and export that survives restarts and has
  no request-timeout ceiling.
- **JSONL, CSV, Parquet** formats, with JSONL as the lossless reference.
- **Idempotent import**: re-running the same file does not duplicate — rows
  with a stable key upsert in place; keyless rows run the normal duplicate
  detection and route likely duplicates to the **review queue** (§6).
- **Per-row best-effort**: valid rows commit, invalid rows land in a
  downloadable error report (§7).
- **Export honours read authorisation, field masking, and audit** (§8).
- One uniform contract across every entity service (§10).

**Non-goals**

- Cross-entity bulk in one call — import/export is per entity service.
- Streaming / real-time ingest — this is batch; the event bus is the
  real-time path.
- ETL transformation — records load as-is against the entity's validators;
  reshaping is the caller's job.
- A bulk backdoor around events or audit — every imported row emits its
  normal event and audit record (§6, §9).

## 3. Execution model — async jobs on `bg_pg`

Bulk operations run as loco **Postgres-backed background jobs**
(`queue.kind: Postgres`, [loco.md](loco.md)); no external broker.

```
POST /api/v1/<plural>/import   ──202──▶ { job_id }
                                          │  enqueue bulk_job (queued)
                                          ▼
                        bg_pg worker drains: queued → running
                          per-row pipeline (§6/§7), progress updates
                          → completed | completed_with_errors | failed
GET  /api/v1/<plural>/import/{job_id} ──▶ status, counts, errors_url, review_url
```

### `bulk_jobs` table (per service)

```sql
CREATE TABLE bulk_jobs (
    id            UUID PRIMARY KEY,
    kind          TEXT NOT NULL,        -- import | export
    entity        TEXT NOT NULL,
    format        TEXT NOT NULL,        -- jsonl | csv | parquet
    status        TEXT NOT NULL,        -- queued|running|completed|completed_with_errors|failed
    params        JSONB NOT NULL,       -- dedupe mode, filter, masking profile, dry_run, …
    rows_total    BIGINT,
    rows_processed BIGINT NOT NULL DEFAULT 0,
    rows_created  BIGINT NOT NULL DEFAULT 0,
    rows_upserted BIGINT NOT NULL DEFAULT 0,
    rows_to_review BIGINT NOT NULL DEFAULT 0,
    rows_errored  BIGINT NOT NULL DEFAULT 0,
    actor         TEXT,                 -- bearer sub
    idempotency_key TEXT,               -- client-supplied; dedupes a retried submit
    input_url     TEXT,                 -- uploaded source artifact
    result_url    TEXT,                 -- export output / import receipt
    error_report_url TEXT,              -- downloadable per-row errors (§7)
    created_at    TIMESTAMPTZ NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL,
    expires_at    TIMESTAMPTZ,          -- artifact + row TTL
    UNIQUE (entity, kind, idempotency_key)   -- retried submit ⇒ same job
);
```

Artifacts (the uploaded file, the export output, the error report) live in an
object store (S3-compatible in deployment, local fs in dev — config-driven,
§12), referenced by short-lived, access-controlled URLs. Rows + artifacts are
TTL'd (`expires_at`).

## 4. API surface (uniform, per entity service)

```
POST   /api/v1/<plural>/import        202 {job_id}  — body: format, dedupe_mode, dry_run; file upload
GET    /api/v1/<plural>/import/{id}    job status + counts + errors_url + review_url
POST   /api/v1/<plural>/export        202 {job_id}  — body: format, filter, fields, include_soft_deleted, masking_profile
GET    /api/v1/<plural>/export/{id}    job status + download_url
GET    /api/v1/<plural>/bulk-jobs      list (filter by kind/status); GET .../{id} for one
```

- **Import** accepts a `dedupe_mode` (default per §6), an optional `dry_run`
  (validate + classify, commit nothing, return the would-be report), and the
  file (multipart upload or a presigned-URL handoff for large files).
- **Export** takes the entity's existing **list/search filter** (so "export
  everything matching X"), an optional field projection, an
  `include_soft_deleted` flag (default `false`, gated), and a
  `masking_profile` (§8).

## 5. File formats

- **JSONL (reference, lossless).** One JSON record per line, each line the
  entity's API wire type (same Serde shape as `GET /<plural>/{id}`).
  Streaming read/write, bounded memory, round-trips losslessly including
  nested identifiers / addresses / contacts / tags / relationships. **Prefer
  JSONL when fidelity matters.**
- **CSV (operator / spreadsheet).** Flat columns. Nested and repeated fields
  need a documented flattening convention, fixed family-wide:
  - **scalar fields** → one column each;
  - **single nested object** (e.g. primary address) → dotted columns
    (`address.postcode`);
  - **arrays / arrays-of-objects** (identifiers, contacts, tags,
    relationships, `entity_links`) → a single **JSON-encoded cell**.
  CSV is inherently lossy for deep nesting; the per-entity spec (§10) lists
  the exact column set, and the doc steers fidelity-sensitive use to JSONL.
- **Parquet (analytics / large export).** Columnar binary, same logical schema
  as JSONL (nested via Parquet nested types or JSON-encoded columns). Heavier
  dependency (`arrow`/`parquet`), **feature-gated** and **export-first** in v1
  (import is roadmap, §12).

## 6. Import semantics

Per row, in the worker:

```
parse + validate (same validators as single create; §7 on failure)
  │
  ├─ stable key present (§10) AND matches an existing record
  │     → UPSERT in place         (idempotent: re-running the file is safe)
  │
  └─ no stable key, or no match
        → run the entity's duplicate detection (same path as create)
            ├─ likely duplicate → REVIEW QUEUE, provenance = import
            └─ otherwise        → CREATE
```

- **Stable key** is each entity's declared upsert key (§10) — typically a
  scheme-scoped external identifier (often the same identifier the matcher
  short-circuits on) or the record `pid` when present.
- **Idempotency** falls out of upsert-by-key: re-submitting a file re-upserts
  the same rows to the same state. Combined with the job `idempotency_key`
  (§3), both the submission and the row effects are safe to retry.
- **Events + audit are not bypassed.** Every created/upserted row emits its
  normal `created`/`updated` event ([event-bus.md](event-bus.md), batched via
  the outbox for large loads) and writes its audit record — so downstream
  consumers (search re-index, the cross-service link aggregator) stay in sync.
- **`provenance = import`** tags review-queue items (and matches the
  cross-service-linking provenance vocabulary) so operators can tell bulk-
  sourced candidates from interactive ones.
- **Dry-run** runs parse + validate + dedupe-classification and returns the
  would-be counts and error report without committing.

## 7. Per-row error handling

- Valid rows commit; invalid rows are **skipped and recorded** — one bad row
  never aborts the load.
- A downloadable **error report** (CSV, or JSONL mirroring input) lists
  `row_number, source_line, field, code, message` for each failure (validation
  `422` reasons reuse the single-create validators).
- Final counts reconcile: `rows_total = rows_created + rows_upserted +
  rows_to_review + rows_errored`. Status is `completed` (zero errors) or
  `completed_with_errors`.
- Recovery loop: operator fixes the error file and re-submits; idempotent
  upsert means re-submitting the previously-good rows is harmless.

## 8. Export — privacy & audit

- **Authorisation + masking match the read API** ([privacy.md](privacy.md)).
  A `masking_profile` selects masked (default) vs full output; full /
  unmasked export requires elevated authorisation — a bulk export must never
  reveal more than the caller could read one record at a time.
- **`include_soft_deleted`** defaults `false` and is gated; soft-deleted rows
  are exported only with explicit authorisation.
- **Filtered** via the entity's existing list/search query, so exports are
  scoped, not all-or-nothing.
- **Every export is audited** — actor, filter, format, row count, masking
  profile, timestamp — because a bulk extract of personal data is itself a
  compliance event (HIPAA / GDPR; acute for person, worker, case). The audit
  row is written even for a zero-row export.
- The single-record **GDPR export** that some entities already provide becomes
  the single-subject special case of this machinery (filter = one `pid`).
- Output streams to the artifact store; the `download_url` is short-lived and
  access-controlled.

## 9. Relationship to other subsystems

- **Matcher / review queue** — import dedupe reuses the existing duplicate
  detection and review queue verbatim; no new matching logic, no new queue.
- **Event bus** — bulk writes are a legitimate event source; they emit the
  same envelopes (batched through the outbox), keeping consumers consistent.
- **Cross-service links** — `entity_links`
  ([cross-service-linking.md](cross-service-linking.md)) are bulk-importable
  too: the `provenance = import` value and the idempotent upsert key
  (`UNIQUE (from_pid, kind, to_ref, valid_from)`) already exist for exactly
  this. A per-entity link-import is an optional extension of the same job.
- **Audit** — import and export are first-class audited actions, not silent
  batch paths.

## 10. Per-entity adoption (what each service declares)

The contract above is identical for every entity. Each entity service spec
adds one section + a §13 task declaring only what differs:

1. **Stable key(s)** — which identifier(s) drive upsert (e.g. person: a
   national / scheme-scoped identifier or `pid`; organization: LEI / DUNS /
   `pid`; course: provider-scoped course code / DOI; case: agency-scoped case
   number / `pid`). Listed explicitly so re-import idempotency is well-defined.
2. **CSV column set + flattening** — the exact flat columns and which fields
   are dotted vs JSON-in-cell (§5), since CSV shape is entity-specific.
3. **Export sensitivity** — any entity-specific masking/authorisation beyond
   the default (personal-data entities — person, worker, case — and the
   `case ↔ person` link especially).
4. **§13 task** — the code follow-up: `bulk_jobs` migration, the five
   endpoints (§4), the `bg_pg` worker, the JSONL/CSV/Parquet codecs, the
   per-row pipeline reusing the single-create validators + matcher + review
   queue, the error report, export masking + audit, and tests (idempotent
   re-import, per-row error report, dedupe-to-review, masked vs full export,
   export audit).

## 11. Rollout

1. **Reference entity (person).** `bulk_jobs` + the job API + the `bg_pg`
   worker + JSONL import/export, upsert-by-key, per-row error report.
2. **CSV + review routing.** Add the CSV flattening convention and the
   keyless-row → duplicate-detection → review-queue path.
3. **Export hardening.** Masking profiles + per-export audit +
   `include_soft_deleted` gating.
4. **Parquet export** (feature-gated).
5. **Roll across the other entities** — uniform contract; only the per-entity
   stable key + CSV columns + sensitivity differ.

## 12. Open questions

- **Artifact store** — S3-compatible in deployment, local fs in dev; config
  shape (mirror the event-bus/JWT env-var pattern). Confirm the dev default.
- **Parquet import** — export-only in v1, or both? (Lean: export-only;
  import is roadmap.)
- **File-size ceiling** — max upload before requiring chunked / presigned
  multipart, and a row cap per job.
- **CSV nested convention** — JSON-in-cell for arrays/objects + dotted for
  single nested is the proposed fix (§5); confirm before entities enumerate
  columns.
- **Event volume** — batch event emission for very large imports (one outbox
  batch vs per-row) to avoid flooding consumers. (Lean: batch via the outbox.)
