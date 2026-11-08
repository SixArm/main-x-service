-- WK-E3: an erasure's tombstone name was inserted with `is_primary` left
-- at its default, `false`, so an erased row failed to assemble ("Worker
-- has no primary name") on any read that includes deleted rows. New
-- erasures insert it primary; this flags the tombstones already written.
--
-- Narrow on purpose: only a `(erased)` name with no given names, on a
-- retired row whose digests erasure nulled (WK-E1), belonging to a worker
-- with no primary name at all. The digests stay NULL; nothing is rehashed.
UPDATE worker_names n
   SET is_primary = TRUE
 WHERE n.family = '(erased)'
   AND n.is_primary = FALSE
   AND cardinality(n.given) = 0
   AND NOT EXISTS (
       SELECT 1 FROM worker_names p
        WHERE p.worker_id = n.worker_id AND p.is_primary)
   AND EXISTS (
       SELECT 1 FROM workers w
        WHERE w.id = n.worker_id
          AND w.deleted_at IS NOT NULL
          AND w.content_hash IS NULL);
