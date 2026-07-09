-- The bidirectional, queryable graph (derived read-model; spec §10.1).
CREATE TABLE edges (
    edge_id         UUID PRIMARY KEY,       -- = source linked event's edge_id
    from_ref        TEXT NOT NULL,          -- EntityRef URN; canonical "from"
    to_ref          TEXT NOT NULL,          -- EntityRef URN
    kind            TEXT NOT NULL,          -- closed registry (design §9)
    directed        BOOLEAN NOT NULL,       -- false for symmetric kinds
    role            TEXT,                   -- e.g. job title for employed_by
    confidence      DOUBLE PRECISION,       -- 1.0 operator-asserted; <1 suggested
    provenance      TEXT NOT NULL,          -- operator | import | matcher_suggested
    valid_from      DATE,
    valid_to        DATE,
    status          TEXT NOT NULL,          -- unverified | verified | dangling
    observed_at     TIMESTAMPTZ NOT NULL,   -- when the linked event was consumed
    source_event_id UUID NOT NULL
);
CREATE INDEX edges_from ON edges (from_ref, kind);
CREATE INDEX edges_to ON edges (to_ref, kind);
CREATE INDEX edges_status ON edges (status);
