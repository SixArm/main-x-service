ALTER TABLE audit_logs
    ADD COLUMN IF NOT EXISTS prev_hash_blake3 character varying NULL,
    ADD COLUMN IF NOT EXISTS hash_blake3 character varying NULL;

ALTER TABLE cases ADD COLUMN IF NOT EXISTS content_hash_blake3 character varying NULL;
