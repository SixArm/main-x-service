-- The SHA-3 (FIPS 202) companion digests over the same pre-image as
-- SHA-256. A sponge, structurally unrelated to SHA-256's Merkle-Damgard
-- chaining, so a cryptanalytic advance against one design family does
-- not transfer. The chain gets a parallel chain: the SHA-3 digest binds
-- the SHA-3 predecessor.
--
-- Nullable, never back-filled, for the same reason as the others.
ALTER TABLE audit_logs
    ADD COLUMN IF NOT EXISTS prev_hash_sha3 character varying NULL,
    ADD COLUMN IF NOT EXISTS hash_sha3 character varying NULL;

ALTER TABLE cases ADD COLUMN IF NOT EXISTS content_hash_sha3 character varying NULL;
