-- Match evidence on the review queue (agents/share/match-evidence.md §10
-- step 4). The matcher's Fellegi–Sunter evidence for the pair: component
-- names and bits, never field values. Nullable: rows written before this
-- column existed stay NULL and read back with no evidence.
ALTER TABLE course_match_scores ADD COLUMN IF NOT EXISTS evidence JSONB;
