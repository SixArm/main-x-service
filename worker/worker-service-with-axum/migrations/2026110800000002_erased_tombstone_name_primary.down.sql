-- Deliberately a no-op. Un-flagging the tombstones would make erased rows
-- fail to assemble again (WK-E3), and the rows this migration flagged
-- cannot be told apart from ones new erasures wrote primary.
SELECT 1;
