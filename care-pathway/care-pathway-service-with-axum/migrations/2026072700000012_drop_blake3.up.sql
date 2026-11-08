-- Drop the BLAKE3 companion digests. BLAKE3 is not FIPS/NIST approved,
-- and these services are built for regimes that require an approved
-- hash. SHA-256 (FIPS 180-4) and SHA-3 (FIPS 202) both remain.
--
-- Dropped rather than left in place: a digest column nothing maintains
-- reads as coverage that does not exist. Rollback restores the columns
-- but not their values.
ALTER TABLE audit_logs
    DROP COLUMN prev_hash_blake3,
    DROP COLUMN hash_blake3;

ALTER TABLE care_pathways DROP COLUMN content_hash_blake3;
