DROP INDEX IF EXISTS bulk_jobs_artifact_sweep;
ALTER TABLE bulk_jobs DROP COLUMN IF EXISTS artifact_deleted_at;
