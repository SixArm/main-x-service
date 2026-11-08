-- The `review_queue` table: the stored duplicate-review queue, adopted as
-- part of BLK-5 to route a keyless bulk-import row's likely duplicate to
-- an operator (agents/share/bulk-import-export.md §6). Pairs are stored
-- normalized (`record_id_a < record_id_b`) under a UNIQUE constraint so
-- re-scans upsert in place and decided rows keep their decision.
--
-- `provenance` is part of the initial table shape: how a pair was first
-- surfaced (`operator` / `import` / `matcher_suggested`), never touched
-- by a re-scan upsert.
CREATE TABLE IF NOT EXISTS review_queue (
    id UUID PRIMARY KEY,
    record_id_a UUID NOT NULL,
    record_id_b UUID NOT NULL,
    match_score DOUBLE PRECISION NOT NULL,
    match_quality VARCHAR NOT NULL,
    detection_method VARCHAR NOT NULL,
    score_breakdown JSONB NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    provenance VARCHAR NOT NULL DEFAULT 'operator',
    reviewed_by VARCHAR NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reviewed_at TIMESTAMPTZ NULL,
    CONSTRAINT review_queue_pair_unique UNIQUE (record_id_a, record_id_b)
);

CREATE INDEX IF NOT EXISTS review_queue_status_idx ON review_queue (status);
