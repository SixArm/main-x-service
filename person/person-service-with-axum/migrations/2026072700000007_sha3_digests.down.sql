-- Drop the SHA-3 companion digests.
ALTER TABLE audit_log DROP COLUMN IF EXISTS prev_hash_sha3;
ALTER TABLE audit_log DROP COLUMN IF EXISTS hash_sha3;
ALTER TABLE persons DROP COLUMN IF EXISTS content_hash_sha3;
