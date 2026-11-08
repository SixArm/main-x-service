-- The keyed-integrity (HMAC) columns. The SHA-256 and SHA-3 digests are
-- unkeyed and their pre-image format is published, so anyone who can
-- write SQL can recompute them; an HMAC over the same pre-image needs a
-- key held in the service environment and never written to this
-- database.
--
-- Each column stores `"<key id>:<hex>"`, so a key rotation does not read
-- as mass tampering. Nullable and never back-filled: rows without one
-- are reported `mac_absent`, never as mismatches.
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS mac character varying NULL;

ALTER TABLE care_pathways ADD COLUMN IF NOT EXISTS content_mac character varying NULL;
