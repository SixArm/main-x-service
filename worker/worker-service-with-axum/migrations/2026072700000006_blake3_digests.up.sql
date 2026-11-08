-- Add the BLAKE3 companion digests.
--
-- The family keeps **two** integrity digests over the same pre-image --
-- SHA-256 and BLAKE3 -- rather than replacing one with the other. See
-- spec/12-compliance.md 12.4z for the full rationale; briefly:
--
--   * SHA-256 stays because it is the conservative choice: FIPS 180-4,
--     NIST-approved, and what a compliance reviewer expects to see named.
--   * BLAKE3 is added for speed (SIMD + parallel tree hashing), and
--     because holding a second independent digest is ALGORITHM AGILITY:
--     if either function is ever weakened, the already-stored history is
--     still verifiable under the other, with no flag day.
--
-- That agility has a deadline, which is why this is not deferred until a
-- weakness appears. A digest attests only to the content hashed at write
-- time, so re-hashing history later would compute digests from whatever
-- the rows contain then -- certifying content that may already have been
-- altered. The second digest has to be written now or the option is gone.
--
-- The chain gets a PARALLEL chain (prev_hash_blake3 / hash_blake3), not
-- merely a second digest of the same row: the BLAKE3 digest binds the
-- BLAKE3 predecessor, so neither chain's linkage depends on the other
-- algorithm's collision resistance. Binding the SHA-256 predecessor would
-- inherit SHA-256's weaknesses and defeat the point.
--
-- All columns are nullable and existing rows stay NULL: back-filling
-- would compute digests from current content, asserting exactly what
-- these columns exist to test. Verification reports them as
-- `blake3_unhashed`, an honest gap rather than a false clean bill.

ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS prev_hash_blake3 TEXT;
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS hash_blake3 TEXT;
ALTER TABLE workers ADD COLUMN IF NOT EXISTS content_hash_blake3 TEXT;

-- Assessments carry their own digest (they are not part of the
-- assembled Worker), so they need their own BLAKE3 companion too.
ALTER TABLE worker_assessments ADD COLUMN IF NOT EXISTS content_hash_blake3 TEXT;
