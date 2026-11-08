-- The BLAKE3 companion digests: a parallel chain on `audit_logs`
-- (`prev_hash_blake3` / `hash_blake3`) and a second record digest on
-- `care_pathways`. Dropped again by 2026072700000012 (BLAKE3 is not
-- FIPS/NIST approved); kept here so the migration history matches every
-- database that applied it.
--
-- Nullable and never back-filled: a digest computed from current content
-- certifies whatever that content now is.
ALTER TABLE audit_logs
    ADD COLUMN IF NOT EXISTS prev_hash_blake3 character varying NULL,
    ADD COLUMN IF NOT EXISTS hash_blake3 character varying NULL;

ALTER TABLE care_pathways ADD COLUMN IF NOT EXISTS content_hash_blake3 character varying NULL;
