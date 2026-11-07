ALTER TABLE audit_logs
    DROP COLUMN hash,
    DROP COLUMN hash_sha3,
    DROP COLUMN mac;

ALTER TABLE organizations
    DROP COLUMN content_hash,
    DROP COLUMN content_hash_sha3,
    DROP COLUMN content_mac;
