-- `review_queue.provenance` (BLK-5): how a candidate pair was first
-- surfaced: `operator` batch scan vs `import` bulk-import keyless-row
-- duplicate detection. Existing rows predate this column and were all
-- operator-triggered scans, hence the backfill default.
ALTER TABLE review_queue
    ADD COLUMN IF NOT EXISTS provenance VARCHAR NOT NULL DEFAULT 'operator';
