-- Transactional outbox for the durable event bus (Phase 2).
-- See agents/share/event-bus.md §3. One row is written in the SAME
-- transaction as the entity mutation, so a committed change always has
-- its event and vice versa; a relay worker (Phase 3, roadmap) later
-- drains unpublished rows to Fluvio and stamps published_at.

CREATE TABLE event_outbox (
    id             BIGSERIAL PRIMARY KEY,        -- global monotonic relay order
    event_id       UUID NOT NULL UNIQUE,         -- envelope id (consumer dedup key)
    entity         TEXT NOT NULL,                -- "event"
    entity_pid     UUID NOT NULL,                -- record pid (bus partition key)
    kind           TEXT NOT NULL,                -- created | updated | deleted | merged
    occurred_at    TIMESTAMPTZ NOT NULL,         -- when the change occurred (stamped at enqueue)
    actor          TEXT,                         -- acting user pid (bearer sub), if any
    schema_version INT NOT NULL DEFAULT 1,       -- envelope schema version
    payload        JSONB NOT NULL,               -- the full canonical envelope (§4)
    published_at   TIMESTAMPTZ                   -- NULL until the relay ships the row
);

-- The relay polls only unpublished rows in id order.
CREATE INDEX event_outbox_unpublished ON event_outbox (id) WHERE published_at IS NULL;
