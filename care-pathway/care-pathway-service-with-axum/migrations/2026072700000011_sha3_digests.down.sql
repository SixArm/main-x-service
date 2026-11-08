ALTER TABLE audit_logs
    DROP COLUMN prev_hash_sha3,
    DROP COLUMN hash_sha3;

ALTER TABLE care_pathways DROP COLUMN content_hash_sha3;
