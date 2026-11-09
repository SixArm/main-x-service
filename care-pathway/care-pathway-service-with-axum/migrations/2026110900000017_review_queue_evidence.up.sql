-- Match evidence on review-queue items (agents/share/match-evidence.md
-- §10 step 4). The matcher's `Evidence` (weight_bits, fields_compared,
-- contradictions, strength, components) for the pair, as computed when
-- the pair was queued or last re-scanned. It holds component names and
-- bits only, never field values.
--
-- Nullable: rows written before this migration carry no evidence and
-- stay NULL; nothing is backfilled, because evidence is computed from
-- the records as they stood at detection time.
ALTER TABLE review_queue ADD COLUMN IF NOT EXISTS evidence JSONB NULL;
