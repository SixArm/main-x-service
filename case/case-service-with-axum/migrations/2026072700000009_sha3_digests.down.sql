ALTER TABLE audit_logs
    DROP COLUMN prev_hash_sha3,
    DROP COLUMN hash_sha3;

ALTER TABLE cases DROP COLUMN content_hash_sha3;
