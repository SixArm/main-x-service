-- Row-level integrity columns. All nullable and all never back-filled:
-- a digest computed later from current content would certify whatever
-- that content now is, which is the claim it exists to test.
--
-- `hash` / `content_hash`: SHA-256 (FIPS 180-4) over the row's pre-image.
-- `hash_sha3` / `content_hash_sha3`: SHA3-256 (FIPS 202), kept for
-- structural diversity (a sponge, unrelated to SHA-256's chaining).
-- `mac` / `content_mac`: HMAC-SHA256 (FIPS 198-1) over the same
-- pre-image, keyed, stored as "<scheme>.<key id>:<hex>". The MAC is the
-- one that matters against a deliberate edit: the digests are unkeyed and
-- their pre-image format is published.
ALTER TABLE audit_logs
    ADD COLUMN IF NOT EXISTS hash character varying NULL,
    ADD COLUMN IF NOT EXISTS hash_sha3 character varying NULL,
    ADD COLUMN IF NOT EXISTS mac character varying NULL;

ALTER TABLE organizations
    ADD COLUMN IF NOT EXISTS content_hash character varying NULL,
    ADD COLUMN IF NOT EXISTS content_hash_sha3 character varying NULL,
    ADD COLUMN IF NOT EXISTS content_mac character varying NULL;
