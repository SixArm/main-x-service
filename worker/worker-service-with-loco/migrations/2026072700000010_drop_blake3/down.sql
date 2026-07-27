-- Recreate the BLAKE3 columns (empty).
--
-- Rollback restores the columns but not their values: the digests are
-- derived data, and recomputing them from current content would certify
-- whatever that content now is -- the claim these columns exist to test.

ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS prev_hash_blake3 TEXT;
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS hash_blake3 TEXT;
ALTER TABLE workers ADD COLUMN IF NOT EXISTS content_hash_blake3 TEXT;
ALTER TABLE worker_assessments ADD COLUMN IF NOT EXISTS content_hash_blake3 TEXT;
