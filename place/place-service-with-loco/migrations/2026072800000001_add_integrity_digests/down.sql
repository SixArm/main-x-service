ALTER TABLE places
    DROP COLUMN IF EXISTS content_hash,
    DROP COLUMN IF EXISTS content_hash_sha3,
    DROP COLUMN IF EXISTS content_mac;

ALTER TABLE audit_log
    DROP COLUMN IF EXISTS mac;
