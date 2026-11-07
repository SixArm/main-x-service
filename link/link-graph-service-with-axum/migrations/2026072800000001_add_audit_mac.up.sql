-- The keyed-integrity column on `audit_log`, plus the two unkeyed
-- digests beside it.
--
-- An HMAC-SHA256 over each audit row's pre-image, keyed by a secret held
-- in the service environment and never written to this database, so a
-- stolen backup, a replica, or a SQL-injection foothold cannot forge one.
-- SHA-256 and SHA3-256 over the same pre-image are written
-- unconditionally, so a deployment with no key still has a check.
--
-- Only the audit trail gets these. The `edges` table is a derived
-- read-model rebuilt from the entity event streams, so a MAC there would
-- attest to a projection rather than a source of truth.
--
-- Nullable and never back-filled: a MAC computed later would certify
-- whatever the row now says, which is the claim it exists to test.
ALTER TABLE audit_log
    ADD COLUMN IF NOT EXISTS hash varchar,
    ADD COLUMN IF NOT EXISTS hash_sha3 varchar,
    ADD COLUMN IF NOT EXISTS mac varchar;
