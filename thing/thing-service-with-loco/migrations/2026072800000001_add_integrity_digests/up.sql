-- Row-level integrity columns.
--
-- Three values per row over one pre-image: SHA-256 (FIPS 180-4),
-- SHA3-256 (FIPS 202), and a keyed HMAC-SHA256 (FIPS 198-1).
--
-- The MAC is the one that defends against a deliberate edit. The two
-- digests are unkeyed and their pre-image format is published, so anyone
-- who can write SQL recomputes them; what they catch is careless or
-- unaware modification. The MAC raises that bar to a key held in the
-- service environment and never written to this database.
--
-- All nullable and never back-filled: a digest computed later from
-- current content would certify whatever that content now is, which is
-- the claim it exists to test. Rows without one are reported unhashed or
-- mac_absent, never as mismatches.
ALTER TABLE things
    ADD COLUMN IF NOT EXISTS content_hash TEXT,
    ADD COLUMN IF NOT EXISTS content_hash_sha3 TEXT,
    ADD COLUMN IF NOT EXISTS content_mac TEXT;

ALTER TABLE audit_log
    ADD COLUMN IF NOT EXISTS mac TEXT;
