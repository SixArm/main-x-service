-- Match evidence (agents/share/match-evidence.md §4, §10 step 4): the
-- place-matcher's Fellegi-Sunter evidence for the pair (weight in bits,
-- fields compared, contradictions, strength, per-term listing), stored
-- beside score_breakdown so a reviewer sees why a pair was or was not
-- auto-merged. Component names and bits only, never field values.
-- Nullable: rows scanned before this column existed stay NULL.
ALTER TABLE review_queue ADD COLUMN IF NOT EXISTS evidence JSONB NULL;
