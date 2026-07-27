-- Add the SHA-3 companion digests.
--
-- Third algorithm alongside SHA-256 and BLAKE3, over the same pre-image.
-- The point is STRUCTURAL DIVERSITY: SHA-256 is Merkle-Damgard, BLAKE3 is
-- an ARX tree, and SHA-3 is a sponge, so a cryptanalytic advance against
-- one design family does not transfer to the others. SHA-3 is also FIPS
-- 202, so it carries NIST standing while sharing no lineage with SHA-2 --
-- which is what a deployment under strict FIPS rules should name, since
-- BLAKE3 is not FIPS-approved. See spec/12-compliance.md 12.4z.
--
-- Nullable, never back-filled, for the same reason as the others: a
-- digest computed from current content certifies whatever that content
-- now is, which is the claim these columns exist to test.

ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS prev_hash_sha3 TEXT;
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS hash_sha3 TEXT;
ALTER TABLE persons ADD COLUMN IF NOT EXISTS content_hash_sha3 TEXT;