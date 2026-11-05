-- Drop the BLAKE3 companion digests.
--
-- BLAKE3 was added as a second digest for speed and algorithm agility,
-- then removed: it is **not FIPS/NIST approved**, and these services are
-- built for regimes that require an approved hash. Keeping a digest that
-- cannot be named in a control document costs a column and a hash pass
-- per write while contributing nothing an auditor may rely on.
--
-- The integrity property is unchanged. SHA-256 (FIPS 180-4) and SHA-3
-- (FIPS 202) both remain, and they are the *better* pair for the
-- structural-diversity argument anyway: Merkle-Damgard against sponge,
-- two unrelated design families, both approved. BLAKE3's ARX tree was a
-- third family but an unusable one for compliance purposes.
--
-- The columns are dropped rather than left in place. A digest column
-- nothing maintains is worse than no column: it reads as coverage that
-- does not exist, and its values silently rot from the first write after
-- this migration.

ALTER TABLE audit_log DROP COLUMN IF EXISTS prev_hash_blake3;
ALTER TABLE audit_log DROP COLUMN IF EXISTS hash_blake3;
ALTER TABLE workers DROP COLUMN IF EXISTS content_hash_blake3;
ALTER TABLE worker_assessments DROP COLUMN IF EXISTS content_hash_blake3;
