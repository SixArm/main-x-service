-- Row-level record integrity: a per-row content hash on `workers`.
--
-- The audit chain proves the *trail* was not rewritten. It says nothing
-- about the worker records themselves: an attacker with SQL access could
-- edit a stored name, identifier, or address and, writing no audit row,
-- leave the chain verifying. This column closes that gap -- the one the
-- database audit triggers gestured at without closing, since an unchained
-- trigger row was as forgeable as the edit it claimed to witness.
--
-- NULLable, and existing rows are left NULL on purpose. Back-filling would
-- mean computing each hash from the current content, which asserts that
-- the current content is authentic -- exactly the claim this column exists
-- to test. A back-filled hash would certify whatever an attacker had
-- already changed. Rows are hashed on their next write; until then
-- verification reports them as `unhashed`, which is an honest gap rather
-- than a false clean bill of health.

ALTER TABLE workers ADD COLUMN IF NOT EXISTS content_hash TEXT;

COMMENT ON COLUMN workers.content_hash IS
    'SHA-256 over the assembled worker record (see src/compliance/record_integrity.rs). NULL means not yet hashed, never "verified".';
