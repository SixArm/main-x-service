-- The SHA-3 companion digests, over the same pre-image as SHA-256: a
-- parallel chain on `audit_logs` (`prev_hash_sha3` / `hash_sha3`) and a
-- second record digest on `care_pathways`. SHA-3 (FIPS 202) is a sponge,
-- structurally unrelated to SHA-256's Merkle-Damgard design, so a
-- cryptanalytic advance against one does not transfer to the other
-- (spec/12-compliance.md §12.4z).
--
-- Nullable and never back-filled: a digest computed from current content
-- certifies whatever that content now is.
ALTER TABLE audit_logs
    ADD COLUMN IF NOT EXISTS prev_hash_sha3 character varying NULL,
    ADD COLUMN IF NOT EXISTS hash_sha3 character varying NULL;

ALTER TABLE care_pathways ADD COLUMN IF NOT EXISTS content_hash_sha3 character varying NULL;
