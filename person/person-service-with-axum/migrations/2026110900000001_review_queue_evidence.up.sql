-- Match evidence (agents/share/match-evidence.md §10 step 4): the
-- matcher's Fellegi–Sunter evidence for a review-queue pair — weight in
-- bits, fields compared, contradictions, strength and the per-term
-- listing. Component names and bits only, never field values.
--
-- Nullable and with no backfill: a row written before this column has
-- no evidence, and inventing one would claim a computation that never
-- ran. The next re-scan of the pair fills it.
ALTER TABLE review_queue ADD COLUMN IF NOT EXISTS evidence JSONB;
