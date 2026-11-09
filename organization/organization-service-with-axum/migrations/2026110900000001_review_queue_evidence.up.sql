-- `review_queue.evidence`: the matcher's match evidence for the pair
-- (`agents/share/match-evidence.md` §3/§5): `weight_bits`,
-- `fields_compared`, `contradictions`, `strength`, `components`.
-- Component names and bits only, never field values. Written on insert
-- and refreshed on the re-scan upsert; rows queued before this column
-- existed stay NULL.
ALTER TABLE review_queue
    ADD COLUMN IF NOT EXISTS evidence JSONB NULL;
