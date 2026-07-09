-- Per-topic bus position and the freshness watermark backing `as_of`
-- (spec §10.3). The idempotency `processed_events` table is v1-deferred.
CREATE TABLE consumer_offsets (
    topic            TEXT PRIMARY KEY,          -- mxi.<entity>.events
    offset_val       BIGINT NOT NULL,           -- last committed bus offset
    last_occurred_at TIMESTAMPTZ NOT NULL       -- freshness watermark source
);
