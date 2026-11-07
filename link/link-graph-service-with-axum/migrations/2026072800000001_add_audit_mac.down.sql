ALTER TABLE audit_log
    DROP COLUMN hash,
    DROP COLUMN hash_sha3,
    DROP COLUMN mac;
