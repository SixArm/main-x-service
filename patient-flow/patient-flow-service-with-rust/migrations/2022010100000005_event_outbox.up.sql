-- The `event_outbox` table: the transactional-outbox hand-off buffer
-- for the durable event bus (Phase 2; `agents/share/event-bus.md` §3).
-- One row is written in the same transaction as the entity mutation, so
-- a committed change always has its event and vice versa; a relay
-- worker (Phase 3, roadmap) later drains unpublished rows and stamps
-- `published_at`.
--
-- Moved unchanged from the SeaORM migration, which was already this SQL.
-- `id` is SERIAL (integer), not the bigint identity of the other tables.

CREATE TABLE IF NOT EXISTS event_outbox (
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    id SERIAL PRIMARY KEY,
    event_id UUID NOT NULL,
    entity VARCHAR NOT NULL,
    entity_pid UUID NOT NULL,
    kind VARCHAR NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    actor VARCHAR NULL,
    schema_version INTEGER NOT NULL,
    payload JSONB NOT NULL,
    published_at TIMESTAMPTZ NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS event_outbox_event_id
    ON event_outbox (event_id);
-- The relay polls only unpublished rows in id order.
CREATE INDEX IF NOT EXISTS event_outbox_unpublished
    ON event_outbox (id) WHERE published_at IS NULL;
