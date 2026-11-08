-- The `bulk_jobs` table: durable state for asynchronous bulk operations,
-- the family schema in agents/share/bulk-import-export.md §3. Its first
-- consumer was the FHIR Bulk Data `$export` operation; the import-side
-- columns were created at the same time so both halves share one shape.
CREATE TABLE IF NOT EXISTS bulk_jobs (
    id uuid NOT NULL PRIMARY KEY,
    kind character varying NOT NULL,
    entity character varying NOT NULL,
    format character varying NOT NULL,
    status character varying NOT NULL,
    params jsonb NOT NULL,
    rows_total bigint NULL,
    rows_processed bigint NOT NULL DEFAULT 0,
    rows_created bigint NOT NULL DEFAULT 0,
    rows_upserted bigint NOT NULL DEFAULT 0,
    rows_to_review bigint NOT NULL DEFAULT 0,
    rows_errored bigint NOT NULL DEFAULT 0,
    actor character varying NULL,
    idempotency_key character varying NULL,
    input_url character varying NULL,
    result_url character varying NULL,
    error_report_url character varying NULL,
    error character varying NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NULL
);

-- A retried submit must return the same job rather than starting a second
-- export of the same data (shared doc §3). Rows with no key do not
-- collide with each other, because NULLs are distinct.
CREATE UNIQUE INDEX bulk_jobs_idempotency_idx ON bulk_jobs (entity, kind, idempotency_key);

-- The status poll and the retention sweep both filter on status.
CREATE INDEX bulk_jobs_status_idx ON bulk_jobs (status);
