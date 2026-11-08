-- The keyed-integrity (HMAC-SHA256, FIPS 198-1) columns, stored as
-- "<key id>:<hex>". The SHA-256 and SHA-3 digests are unkeyed and their
-- pre-image format is published, so anyone who can write SQL can
-- recompute them; the MAC needs a key that is never written to this
-- database. The key id makes rotation survivable.
--
-- Nullable, and never back-filled. Rows without one are reported
-- `mac_absent`, never as mismatches.
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS mac character varying NULL;

ALTER TABLE cases ADD COLUMN IF NOT EXISTS content_mac character varying NULL;
