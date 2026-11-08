-- Bulk import/export jobs (agents/share/bulk-import-export.md §3).
-- Each async bulk operation is one row: an operator submits an import or
-- export, a `bg_pg` background worker drains it, and the row records the
-- status, the per-row counts, and the artifact references (input file,
-- export output, per-row error report). Rollout step 1 supports the
-- `jsonl` format only; CSV/Parquet, review-queue routing, and export
-- masking are later steps.

CREATE TABLE bulk_jobs (
    id               UUID PRIMARY KEY,
    kind             TEXT NOT NULL,                       -- import | export
    entity           TEXT NOT NULL,                       -- "person"
    format           TEXT NOT NULL,                       -- jsonl (csv | parquet later)
    status           TEXT NOT NULL,                       -- queued|running|completed|completed_with_errors|failed
    params           JSONB NOT NULL DEFAULT '{}',         -- dry_run, filter, …
    rows_total       BIGINT,
    rows_processed   BIGINT NOT NULL DEFAULT 0,
    rows_created     BIGINT NOT NULL DEFAULT 0,
    rows_upserted    BIGINT NOT NULL DEFAULT 0,
    rows_to_review   BIGINT NOT NULL DEFAULT 0,
    rows_errored     BIGINT NOT NULL DEFAULT 0,
    actor            TEXT,                                -- bearer sub, if any
    idempotency_key  TEXT,                                -- client-supplied; dedupes a retried submit
    input_url        TEXT,                                -- uploaded source artifact
    result_url       TEXT,                                -- export output / import receipt
    error_report_url TEXT,                                -- downloadable per-row errors (§7)
    created_at       TIMESTAMPTZ NOT NULL,
    updated_at       TIMESTAMPTZ NOT NULL,
    expires_at       TIMESTAMPTZ,                         -- artifact + row TTL
    UNIQUE (entity, kind, idempotency_key)                -- retried submit ⇒ same job
);

-- List/poll jobs newest-first, and filter by kind/status.
CREATE INDEX bulk_jobs_kind_status ON bulk_jobs (kind, status, created_at DESC);
