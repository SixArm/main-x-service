-- Match evidence on stored review-queue pairs (agents/share/match-evidence.md
-- §10 step 4): the matcher's `Evidence` for the pair (weight in bits,
-- fields compared, contradictions, strength, per-term listing), written
-- on insert and refreshed on every re-scan upsert. Nullable: rows queued
-- before this column existed stay NULL, and a reader treats NULL as
-- "evidence not recorded", never as "no evidence".
ALTER TABLE review_queue ADD COLUMN IF NOT EXISTS evidence JSONB NULL;
