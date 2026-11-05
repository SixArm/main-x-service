-- Deduplication review queue: one stored row per candidate duplicate
-- pair emitted by the batch scan. Pairs are stored in normalized order
-- (record_id_a < record_id_b) under a UNIQUE constraint so a re-scan
-- upserts the same row: scores refresh, decided rows keep their
-- decision. Statuses are the family's lowercase wire tokens
-- (pending / confirmed / rejected / automerged).
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
