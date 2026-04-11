-- Remove worker_type column from workers table

DROP INDEX IF EXISTS idx_workers_worker_type;
ALTER TABLE workers DROP COLUMN IF EXISTS worker_type;
