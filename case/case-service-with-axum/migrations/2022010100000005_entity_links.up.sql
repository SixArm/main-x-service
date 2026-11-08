-- The `entity_links` table: the write side of cross-service entity
-- linking (agents/share/cross-service-linking.md §4.1). Each row is one
-- outbound edge this service originates from a case: for v1 the
-- `subject_of` (case -> person) edge. The write is optimistic: it stores
-- the assertion and emits a `linked` event, and never calls the target
-- service.
CREATE TABLE entity_links (
    created_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The edge id (also the event's `edge_id`), set by the app.
    id uuid NOT NULL,
    -- This service's originating case record (its `pid`).
    from_pid uuid NOT NULL,
    -- The edge kind token (§9 registry), e.g. `subject_of`.
    kind text NOT NULL,
    -- The far record's `EntityRef` URN, e.g. `person:<uuid>`.
    to_ref text NOT NULL,
    -- Optional role label (unused by `subject_of`).
    role text,
    -- Optional confidence: 1.0 operator-asserted, <1 suggested.
    confidence double precision,
    -- How the edge arose: operator | import | matcher_suggested.
    provenance text NOT NULL,
    -- Affiliation start (nullable; `subject_of` is temporal).
    valid_from date,
    -- Affiliation end ("former subject of" once past).
    valid_to date,
    -- Soft-delete timestamp (a withdrawn edge); null while live.
    deleted_at timestamp with time zone,
    CONSTRAINT entity_links_pkey PRIMARY KEY (id)
);

-- Idempotent-upsert key (§4.1). `NULLS NOT DISTINCT` so a null
-- `valid_from` (a non-temporal edge) still conflicts on re-assertion;
-- otherwise Postgres treats each NULL as distinct and the upsert would
-- insert duplicates.
CREATE UNIQUE INDEX IF NOT EXISTS entity_links_upsert
    ON entity_links (from_pid, kind, to_ref, valid_from) NULLS NOT DISTINCT;

-- Inverse-lookup / list-active support.
CREATE INDEX IF NOT EXISTS entity_links_from ON entity_links (from_pid) WHERE deleted_at IS NULL;
