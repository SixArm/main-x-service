-- The `review_queue` table: the stored batch-deduplication review queue
-- (scan candidates + operator decisions). Pairs are stored normalized
-- (`record_id_a < record_id_b`) under a UNIQUE constraint so re-scans
-- upsert in place and decided rows keep their decision.
CREATE TABLE IF NOT EXISTS review_queue (
    id UUID PRIMARY KEY,
    record_id_a UUID NOT NULL,
    record_id_b UUID NOT NULL,
    match_score DOUBLE PRECISION NOT NULL,
    match_quality VARCHAR NOT NULL,
    detection_method VARCHAR NOT NULL,
    score_breakdown JSONB NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    reviewed_by VARCHAR NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reviewed_at TIMESTAMPTZ NULL,
    CONSTRAINT review_queue_pair_unique UNIQUE (record_id_a, record_id_b)
);

CREATE INDEX IF NOT EXISTS review_queue_status_idx ON review_queue (status);
