-- Add the keyed-integrity (HMAC) columns.
--
-- The SHA-256 and SHA-3 digests are UNKEYED, and their pre-image format
-- is published in spec/12-compliance.md 12.4z. Anyone who can write SQL
-- can defeat them: edit the row, recompute both digests, update both
-- columns. What they detect is careless or unaware modification.
--
-- An HMAC over the same pre-image raises that bar to a key held in the
-- service environment and NEVER written to this database, so a stolen
-- backup, a replica, a SQL-injection foothold, or a DBA without
-- application-server access cannot forge one.
--
-- The column stores "<key id>:<hex>". The key id is what makes rotation
-- survivable: without it, changing the key would invalidate every
-- historical row at once, indistinguishable from mass tampering.
--
-- Nullable and never back-filled: a MAC computed later from current
-- content would certify whatever that content now is, which is the claim
-- it exists to test. Rows without one report mac_absent, not a mismatch.

ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS mac TEXT;
ALTER TABLE persons ADD COLUMN IF NOT EXISTS content_mac TEXT;
