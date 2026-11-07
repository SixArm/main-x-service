-- The `bulk_jobs` table: durable state for asynchronous bulk
-- import/export operations (BLK-5), per the family schema in
-- agents/share/bulk-import-export.md §3.
CREATE TABLE IF NOT EXISTS bulk_jobs (
    -- Job id (application-assigned, also the status-path segment).
    id uuid NOT NULL PRIMARY KEY,
    -- `import` | `export`.
    kind character varying NOT NULL,
    -- The entity the job operates on (`organization`).
    entity character varying NOT NULL,
    -- Input/output format (`jsonl` | `csv` | `tsv`).
    format character varying NOT NULL,
    -- `queued` | `running` | `completed` | `completed_with_errors` | `failed`.
    status character varying NOT NULL,
    -- Request parameters, kept so a job is interpretable after the fact.
    params jsonb NOT NULL,
    rows_total bigint NULL,
    rows_processed bigint NOT NULL DEFAULT 0,
    rows_created bigint NOT NULL DEFAULT 0,
    rows_upserted bigint NOT NULL DEFAULT 0,
    rows_to_review bigint NOT NULL DEFAULT 0,
    rows_errored bigint NOT NULL DEFAULT 0,
    -- The submitting caller's `sub`, when a verified token was presented.
    actor character varying NULL,
    -- Client-supplied key that makes a retried submit return the same job.
    idempotency_key character varying NULL,
    input_url character varying NULL,
    result_url character varying NULL,
    error_report_url character varying NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    -- When the job row and its artifacts may be swept (SEC-B4).
    expires_at timestamp with time zone NULL
);

-- A retried submit must return the same job rather than starting a second
-- import/export of the same data (shared doc §3). Rows with no key do not
-- collide with each other, because NULLs are distinct.
CREATE UNIQUE INDEX bulk_jobs_idempotency_idx ON bulk_jobs (entity, kind, idempotency_key);

-- The status poll and the retention sweep both filter on status.
CREATE INDEX bulk_jobs_status_idx ON bulk_jobs (status);
