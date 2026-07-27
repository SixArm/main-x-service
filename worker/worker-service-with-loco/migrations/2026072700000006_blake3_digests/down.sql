-- Drop the BLAKE3 companion digests.
ALTER TABLE audit_log DROP COLUMN IF EXISTS prev_hash_blake3;
ALTER TABLE audit_log DROP COLUMN IF EXISTS hash_blake3;
ALTER TABLE workers DROP COLUMN IF EXISTS content_hash_blake3;
ALTER TABLE worker_assessments DROP COLUMN IF EXISTS content_hash_blake3;
