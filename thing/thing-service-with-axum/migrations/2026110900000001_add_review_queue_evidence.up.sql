-- Match evidence (agents/share/match-evidence.md) on each stored review
-- pair: the thing-matcher's Fellegi–Sunter bits, fields compared,
-- contradictions and strength, so a reviewer can see why a pair was or
-- was not auto-merged. Nullable: rows scanned before this column stay
-- NULL and read back as "no evidence recorded".
ALTER TABLE review_queue ADD COLUMN IF NOT EXISTS evidence JSONB NULL;
