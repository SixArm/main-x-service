-- Match evidence on each stored review-queue pair
-- (agents/share/match-evidence.md): Fellegi-Sunter bits, contradictions
-- and the strength verdict the batch scan's auto-merge gate read.
-- Component names and bits only, never field values. Nullable: rows
-- queued before this migration carry none.
ALTER TABLE review_queue ADD COLUMN IF NOT EXISTS evidence JSONB NULL;
