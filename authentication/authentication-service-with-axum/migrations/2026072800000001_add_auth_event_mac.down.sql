ALTER TABLE auth_events
    DROP COLUMN hash,
    DROP COLUMN hash_sha3,
    DROP COLUMN mac;
