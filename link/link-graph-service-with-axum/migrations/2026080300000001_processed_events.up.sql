-- Bus-consumer idempotency table (spec §10.3; BUS-2). Under at-least-once
-- delivery a redelivered event must not re-apply; `apply_event_idempotent`
-- checks this table before folding an event into the read-model and
-- records the event_id after a successful apply.
CREATE TABLE processed_events (
    event_id     UUID PRIMARY KEY,        -- envelope event_id; idempotency key
    processed_at TIMESTAMPTZ NOT NULL
);
-- Retention purge scans by age; nothing else queries this table by time,
-- but the PK alone does not support an efficient range scan on it.
CREATE INDEX processed_events_processed_at ON processed_events (processed_at);
