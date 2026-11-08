-- The keyed-integrity column on `auth_events`, plus the two unkeyed
-- digests beside it.
--
-- An HMAC-SHA256 over each row's pre-image, keyed by a secret held in the
-- service environment and never written to this database, so a stolen
-- backup, a replica, or a SQL-injection foothold cannot forge one.
-- SHA-256 and SHA3-256 over the same pre-image are written
-- unconditionally.
--
-- This trail records who logged in and who was granted which
-- authorization attributes. An attacker who escalated privilege and could
-- then edit the `attributes_assigned` row would erase the only account of
-- how they did it.
--
-- Nullable and never back-filled: a MAC computed later would certify
-- whatever the row now says, which is the claim it exists to test.
ALTER TABLE auth_events
    ADD COLUMN IF NOT EXISTS hash character varying,
    ADD COLUMN IF NOT EXISTS hash_sha3 character varying,
    ADD COLUMN IF NOT EXISTS mac character varying;
