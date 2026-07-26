-- Row-level record integrity for `worker_assessments`.
--
-- `workers.content_hash` (migration 2026072600000004) covers the assembled
-- worker: names, identifiers, addresses, contacts, documents, links. It
-- does NOT cover assessments, because assessment rows are reached through
-- their own sub-resource and are not part of the assembled `Worker` the
-- API serves. That left the crate's most sensitive table -- aptitude,
-- personality and psychometric results, with scores and score bands --
-- outside the only control that detects an out-of-band SQL edit.
--
-- Assessments get their OWN hash rather than being folded into the
-- worker's, deliberately:
--
--   * An assessment is written through its own endpoints, on its own
--     lifecycle. Folding it into the parent digest would mean every
--     assessment write loads and rehashes the whole worker -- coupling a
--     sub-resource to its parent for no benefit, and adding a read to
--     every write.
--   * A per-row hash names WHICH assessment was tampered with. A parent
--     digest could only say "something about this worker changed", which
--     is a materially worse answer for the table where a changed score
--     band is the whole point.
--
-- NULLable, and existing rows are left NULL for the same reason as
-- `workers.content_hash`: back-filling would compute each hash from the
-- current content, asserting that the current content is authentic --
-- exactly the claim the hash exists to test.

ALTER TABLE worker_assessments ADD COLUMN IF NOT EXISTS content_hash TEXT;

COMMENT ON COLUMN worker_assessments.content_hash IS
    'SHA-256 over the assessment row (see src/compliance/record_integrity.rs). NULL means not yet hashed, never "verified".';
