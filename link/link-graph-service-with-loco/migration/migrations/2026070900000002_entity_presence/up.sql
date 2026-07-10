-- The existence oracle, fed by every entity's created/deleted events
-- (spec §10.2). Source of the edge status lifecycle.
CREATE TABLE entity_presence (
    ref      TEXT PRIMARY KEY,          -- EntityRef URN
    alive    BOOLEAN NOT NULL,          -- created => true; deleted => false
    last_seq BIGINT NOT NULL            -- per-entity_pid monotonic (envelope seq)
);
