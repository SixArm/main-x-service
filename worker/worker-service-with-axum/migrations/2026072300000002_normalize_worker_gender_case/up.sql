-- Normalize legacy capitalized `workers.gender` values to the
-- lowercase vocabulary the CHECK constraint admits
-- ('male' / 'female' / 'other' / 'unknown').
--
-- Why this exists: until 2026-07-23 the repository layer persisted the
-- domain enum's bare `Debug` form ("Male", "Unknown") at all three
-- write sites. On a schema carrying the `workers_gender_check`
-- constraint those writes were *rejected*, so no such row can exist —
-- this migration is a **no-op there** (0 rows updated). It matters for
-- deployments whose `workers` table was created without the constraint
-- (a hand-rolled schema, an older schema file, or a bulk load through
-- another tool), where the bad values were accepted and would now read
-- back as `Unknown` and block a later ADD CONSTRAINT.
--
-- Idempotent: the WHERE clause makes a re-run touch nothing.
UPDATE workers
   SET gender = lower(gender)
 WHERE gender <> lower(gender);

-- Deliberately NOT normalized: values that are still outside the
-- vocabulary after lowercasing (e.g. 'M', 'not stated', an empty
-- string). Rewriting those to 'unknown' would silently destroy data
-- whose original meaning only an operator can judge, so they are left
-- alone and a subsequent ADD CONSTRAINT will fail loudly on them.
-- Find any before adding the constraint with:
--
--   SELECT id, gender FROM workers
--    WHERE gender NOT IN ('male', 'female', 'other', 'unknown');
