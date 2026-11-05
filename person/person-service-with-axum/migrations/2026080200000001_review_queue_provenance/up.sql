-- BLK-2: how a review-queue candidate pair was first surfaced —
-- 'operator' (an interactive/batch scan, POST /persons/deduplicate) or
-- 'import' (bulk-import keyless-row duplicate detection). Matches the
-- cross-service-linking provenance vocabulary (operator | import |
-- matcher_suggested). Existing rows predate this column and were all
-- operator-triggered scans, hence the backfill default.
ALTER TABLE review_queue ADD COLUMN IF NOT EXISTS provenance VARCHAR NOT NULL DEFAULT 'operator';
