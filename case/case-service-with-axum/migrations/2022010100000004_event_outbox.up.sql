-- The `event_outbox` table: the transactional-outbox hand-off buffer for
-- the durable event bus (agents/share/event-bus.md §3). One row is written
-- in the same transaction as the entity mutation, so a committed change
-- always has its event and vice versa; the relay later drains unpublished
-- rows and stamps `published_at`.
CREATE TABLE IF NOT EXISTS event_outbox (
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Auto pk doubles as the global relay order (ORDER BY id).
    id SERIAL PRIMARY KEY,
    -- Envelope id: consumer dedup key (unique, below).
    event_id UUID NOT NULL,
    -- Entity name, e.g. "case".
    entity VARCHAR NOT NULL,
    -- The record pid: Fluvio partition key (per-record order).
    entity_pid UUID NOT NULL,
    -- created / updated / deleted / merged.
    kind VARCHAR NOT NULL,
    -- When the change occurred (stamped at enqueue).
    occurred_at TIMESTAMPTZ NOT NULL,
    -- Actor (bearer-token user pid), if any.
    actor VARCHAR NULL,
    -- Envelope schema version.
    schema_version INTEGER NOT NULL,
    -- The full envelope (§4) as JSONB.
    payload JSONB NOT NULL,
    -- NULL until the relay ships the row.
    published_at TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS event_outbox_event_id ON event_outbox (event_id);

-- The relay polls only unpublished rows in id order.
CREATE INDEX IF NOT EXISTS event_outbox_unpublished ON event_outbox (id) WHERE published_at IS NULL;
