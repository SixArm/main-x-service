-- Drop the assessment integrity column.
ALTER TABLE worker_assessments DROP COLUMN IF EXISTS content_hash;
