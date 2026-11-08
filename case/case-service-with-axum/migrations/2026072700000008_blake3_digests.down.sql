ALTER TABLE audit_logs
    DROP COLUMN prev_hash_blake3,
    DROP COLUMN hash_blake3;

ALTER TABLE cases DROP COLUMN content_hash_blake3;
