ALTER TABLE audit_logs
    DROP COLUMN prev_hash_blake3,
    DROP COLUMN hash_blake3;

ALTER TABLE care_pathways DROP COLUMN content_hash_blake3;
